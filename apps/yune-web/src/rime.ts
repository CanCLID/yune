import type { Actions, ListenerArgsMap, Message } from "./types";

type ListenerPayload = {
	[K in keyof ListenerArgsMap]: {
		type: "listener";
		name: K;
		args: ListenerArgsMap[K];
	};
}[keyof ListenerArgsMap];

interface SuccessPayload {
	type: "success";
	result: ReturnType<Actions[keyof Actions]>;
	elapsedMs?: number;
	workerStartedAt?: number;
	workerFinishedAt?: number;
}

interface ErrorPayload {
	type: "error";
	error: unknown;
	elapsedMs?: number;
	workerStartedAt?: number;
	workerFinishedAt?: number;
}

interface DiagnosticPayload {
	type: "diagnostic";
	source: string;
	marker: unknown;
}

type Payload = ListenerPayload | SuccessPayload | ErrorPayload | DiagnosticPayload;

type Listeners = { [K in keyof ListenerArgsMap]: (this: Worker, ...args: ListenerArgsMap[K]) => void };

let running: Message | null = null;
const queue: Message[] = [];

const allListenerTypes: (keyof Listeners)[] = [
	"deployStatusChanged",
	"schemaChanged",
	"optionChanged",
	"initialized",
];

const listeners = {} as { [K in keyof Listeners]: Listeners[K][] };
for (const type of allListenerTypes) {
	listeners[type] = [];
}
const lastListenerArgs = {} as Partial<{ [K in keyof ListenerArgsMap]: ListenerArgsMap[K] }>;

const YUNE_WEB_WORKER_VERSION = "m27-startup-runtime-v1";
const debugWindow = window as typeof window & { __YUNE_RIME_VERSION__?: string };
debugWindow.__YUNE_RIME_VERSION__ = YUNE_WEB_WORKER_VERSION;
document.documentElement.dataset["yuneRimeVersion"] = YUNE_WEB_WORKER_VERSION;
const worker = new Worker(`./worker.js?v=${YUNE_WEB_WORKER_VERSION}`);
worker.addEventListener("message", ({ data }: MessageEvent<Payload>) => {
	if (data.type === "diagnostic") {
		const diagnosticWindow = window as typeof window & {
			__YUNE_PERSISTENCE_DIAGNOSTICS__?: DiagnosticPayload[];
		};
		(diagnosticWindow.__YUNE_PERSISTENCE_DIAGNOSTICS__ ??= []).push(data);
		appendPersistenceDiagnostic(data);
		if (shouldLogDebugMessages()) {
			console.info("diagnostic", JSON.stringify(data));
		}
		return;
	}
	if (shouldLogDebugMessages()) console.log("receive", JSON.stringify(data));
	const { type } = data;
	if (type === "listener") {
		const { name, args } = data;
		lastListenerArgs[name] = args as never;
		if (name === "initialized") {
			document.documentElement.dataset["yuneInitialized"] = String(args[0]);
		}
		if (name === "schemaChanged") {
			document.documentElement.dataset["yuneActiveSchema"] = args[0];
			document.documentElement.dataset["yuneActiveSchemaName"] = args[1];
		}
		for (const listener of listeners[name]) {
			// @ts-expect-error Unactionable
			listener.apply(worker, args);
		}
	}
	else if (running) {
		const { resolve, reject } = running;
		const receivedAt = nowMs();
		appendActionDiagnostic({
			action: running.name,
			input: typeof running.args[0] === "string" ? running.args[0] : undefined,
			enqueuedAt: running.enqueuedAt,
			sentAt: running.sentAt,
			receivedAt,
			workerStartedAt: data.workerStartedAt,
			workerFinishedAt: data.workerFinishedAt,
			queueWaitMs: Math.round(((running.sentAt ?? receivedAt) - (running.enqueuedAt ?? receivedAt))),
			workerRoundtripMs: Math.round(receivedAt - (running.sentAt ?? receivedAt)),
			workerMs: data.elapsedMs,
			totalMs: Math.round(receivedAt - (running.enqueuedAt ?? receivedAt)),
		});
		const nextMessage = queue.shift();
		if (nextMessage) {
			postMessage(nextMessage);
		}
		else {
			running = null;
		}
		if (type === "success") {
			resolve(data.result);
		}
		else {
			reject(data.error);
		}
	}
});

function nowMs() {
	return performance.timeOrigin + performance.now();
}

function postMessage(message: Message) {
	if (shouldLogDebugMessages()) console.log("post", JSON.stringify({ name: message.name, args: message.args }));
	message.sentAt = nowMs();
	const { name, args } = running = message;
	worker.postMessage({ name, args });
}

function shouldLogDebugMessages() {
	return import.meta.env.DEV || new URLSearchParams(location.search).has("debug");
}

function appendPersistenceDiagnostic(data: DiagnosticPayload) {
	const existing = document.documentElement.dataset["yunePersistenceDiagnostics"];
	const diagnostics = existing ? JSON.parse(existing) as DiagnosticPayload[] : [];
	diagnostics.push(data);
	document.documentElement.dataset["yunePersistenceDiagnostics"] = JSON.stringify(diagnostics);
}

function appendActionDiagnostic(diagnostic: {
	action: keyof Actions;
	input?: string;
	enqueuedAt?: number;
	sentAt?: number;
	receivedAt: number;
	workerStartedAt?: number;
	workerFinishedAt?: number;
	queueWaitMs: number;
	workerRoundtripMs: number;
	workerMs?: number;
	totalMs: number;
}) {
	const existing = document.documentElement.dataset["yuneActionDiagnostics"];
	const diagnostics = existing ? JSON.parse(existing) as typeof diagnostic[] : [];
	diagnostics.push(diagnostic);
	document.documentElement.dataset["yuneActionDiagnostics"] = JSON.stringify(diagnostics.slice(-100));
}

const allActions: (keyof Actions)[] = [
	"setOption",
	"selectSchema",
	"getUserdbSnapshot",
	"processKey",
	"stageAi",
	"selectCandidate",
	"deleteCandidate",
	"flipPage",
	"customize",
	"deploy",
];

const Rime = {} as Actions;
for (const action of allActions) {
	Rime[action] = registerAction(action) as never;
}
export default Rime;

function registerAction<K extends keyof Actions>(name: K): Actions[K] {
	// @ts-expect-error Unactionable
	return (...args: Parameters<Actions[K]>) =>
		new Promise((resolve, reject) => {
			const message: Message = { name, args, resolve, reject, enqueuedAt: nowMs() };
			if (running) {
				queue.push(message);
			}
			else {
				postMessage(message);
			}
		});
}

export function subscribe<K extends keyof Listeners>(type: K, callback: Listeners[K]) {
	listeners[type].push(callback);
	const cachedArgs = lastListenerArgs[type];
	if (cachedArgs) {
		queueMicrotask(() => {
			if (listeners[type].includes(callback)) {
				callback.apply(worker, cachedArgs);
			}
		});
	}
	return () => {
		listeners[type] = listeners[type].filter(listener => listener !== callback) as never;
	};
}

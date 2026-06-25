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

interface ActionDiagnostic {
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
}

interface SerializedError {
	name?: string;
	message?: string;
	stack?: string;
	value?: string;
}

interface ActionErrorDiagnostic extends ActionDiagnostic {
	args: unknown[];
	error: SerializedError;
}

interface YuneWebDebugApi {
	resetStorage(): Promise<void>;
	actionDiagnostics(): ActionDiagnostic[];
	actionErrors(): ActionErrorDiagnostic[];
	persistenceDiagnostics(): DiagnosticPayload[];
}

type DebugWindow = typeof window & {
	__YUNE_RIME_VERSION__?: string;
	__YUNE_WEB_DEBUG__?: YuneWebDebugApi;
	__YUNE_ACTION_DIAGNOSTICS__?: ActionDiagnostic[];
	__YUNE_ACTION_ERRORS__?: ActionErrorDiagnostic[];
	__YUNE_PERSISTENCE_DIAGNOSTICS__?: DiagnosticPayload[];
};

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
const debugWindow = window as DebugWindow;
debugWindow.__YUNE_RIME_VERSION__ = YUNE_WEB_WORKER_VERSION;
document.documentElement.dataset["yuneRimeVersion"] = YUNE_WEB_WORKER_VERSION;
installDebugHelpers();
const worker = new Worker(`./worker.js?v=${YUNE_WEB_WORKER_VERSION}`);
worker.addEventListener("message", ({ data }: MessageEvent<Payload>) => {
	if (data.type === "diagnostic") {
		(debugWindow.__YUNE_PERSISTENCE_DIAGNOSTICS__ ??= []).push(data);
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
		const currentMessage = running;
		const { resolve, reject } = currentMessage;
		const receivedAt = nowMs();
		const diagnostic = {
			action: currentMessage.name,
			input: typeof currentMessage.args[0] === "string" ? currentMessage.args[0] : undefined,
			enqueuedAt: currentMessage.enqueuedAt,
			sentAt: currentMessage.sentAt,
			receivedAt,
			workerStartedAt: data.workerStartedAt,
			workerFinishedAt: data.workerFinishedAt,
			queueWaitMs: Math.round(((currentMessage.sentAt ?? receivedAt) - (currentMessage.enqueuedAt ?? receivedAt))),
			workerRoundtripMs: Math.round(receivedAt - (currentMessage.sentAt ?? receivedAt)),
			workerMs: data.elapsedMs,
			totalMs: Math.round(receivedAt - (currentMessage.enqueuedAt ?? receivedAt)),
		} satisfies ActionDiagnostic;
		appendActionDiagnostic(diagnostic);
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
			appendActionErrorDiagnostic({
				...diagnostic,
				args: currentMessage.args,
				error: serializeError(data.error),
			});
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

function appendActionDiagnostic(diagnostic: ActionDiagnostic) {
	const existing = document.documentElement.dataset["yuneActionDiagnostics"];
	const diagnostics = existing ? JSON.parse(existing) as ActionDiagnostic[] : [];
	diagnostics.push(diagnostic);
	const latest = diagnostics.slice(-100);
	debugWindow.__YUNE_ACTION_DIAGNOSTICS__ = latest;
	document.documentElement.dataset["yuneActionDiagnostics"] = JSON.stringify(latest);
}

function appendActionErrorDiagnostic(diagnostic: ActionErrorDiagnostic) {
	const existing = document.documentElement.dataset["yuneActionErrors"];
	const diagnostics = existing ? JSON.parse(existing) as ActionErrorDiagnostic[] : [];
	diagnostics.push(diagnostic);
	const latest = diagnostics.slice(-25);
	debugWindow.__YUNE_ACTION_ERRORS__ = latest;
	document.documentElement.dataset["yuneLastActionError"] = JSON.stringify(diagnostic);
	document.documentElement.dataset["yuneActionErrors"] = JSON.stringify(latest);
	console.error("YUNE_WORKER_ACTION_ERROR", diagnostic);
}

function serializeError(error: unknown): SerializedError {
	if (error instanceof Error) {
		return {
			name: error.name,
			message: error.message,
			stack: error.stack,
		};
	}
	if (error && typeof error === "object") {
		const record = error as Record<string, unknown>;
		return {
			name: typeof record["name"] === "string" ? record["name"] : undefined,
			message: typeof record["message"] === "string" ? record["message"] : undefined,
			stack: typeof record["stack"] === "string" ? record["stack"] : undefined,
			value: stringifyUnknown(error),
		};
	}
	return { value: String(error) };
}

function stringifyUnknown(value: unknown) {
	try {
		return JSON.stringify(value);
	}
	catch {
		return String(value);
	}
}

function installDebugHelpers() {
	debugWindow.__YUNE_WEB_DEBUG__ = {
		resetStorage: resetYuneWebStorage,
		actionDiagnostics: () => parseDatasetJson<ActionDiagnostic[]>("yuneActionDiagnostics", []),
		actionErrors: () => parseDatasetJson<ActionErrorDiagnostic[]>("yuneActionErrors", []),
		persistenceDiagnostics: () => parseDatasetJson<DiagnosticPayload[]>("yunePersistenceDiagnostics", []),
	};
}

function parseDatasetJson<T>(key: string, fallback: T): T {
	const raw = document.documentElement.dataset[key];
	if (!raw) {
		return fallback;
	}
	try {
		return JSON.parse(raw) as T;
	}
	catch {
		return fallback;
	}
}

export async function resetYuneWebStorage() {
	window.localStorage?.clear();
	window.sessionStorage?.clear();

	if ("caches" in window) {
		const cacheNames = await window.caches.keys();
		await Promise.all(cacheNames.map(cacheName => window.caches.delete(cacheName)));
	}

	const indexedDb = window.indexedDB as (IDBFactory & {
		databases?: () => Promise<Array<{ name?: string | null }>>;
	}) | undefined;
	if (indexedDb) {
		const databaseNames = new Set<string>(["/rime"]);
		if (indexedDb.databases) {
			for (const database of await indexedDb.databases()) {
				if (database.name) {
					databaseNames.add(database.name);
				}
			}
		}
		await Promise.all([...databaseNames].map(name => deleteIndexedDbDatabase(indexedDb, name)));
	}

	console.info("Yune web storage reset; reloading page.");
	window.location.reload();
}

function deleteIndexedDbDatabase(indexedDb: IDBFactory, name: string) {
	return new Promise<void>((resolve, reject) => {
		const request = indexedDb.deleteDatabase(name);
		request.onsuccess = () => resolve();
		request.onerror = () => reject(request.error);
		request.onblocked = () => {
			console.warn(`Yune web storage reset blocked while deleting IndexedDB database "${name}". Close other tabs for this origin if reset does not complete.`);
			resolve();
		};
	});
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

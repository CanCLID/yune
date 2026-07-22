import type {
	Actions,
	ListenerArgsMap,
	Message,
	RimeSchemaId,
	Web06ActionContext,
	Web06ActionIdentity,
	Web06ControlEventLike,
	Web06DomEventIdentity,
	Web06DomEventSnapshot,
	Web06EventMapResult,
	Web06FanoutAction,
	Web06MappedAction,
	Web06PresentationOutcomeInput,
	Web06WorkerResponseEnvelope,
} from "./types";
import { WEB06_PRIVATE_PROTOCOL_VERSION } from "./types";
import { IS_PUBLIC_DEMO, isRimeSchemaId } from "./consts";
import {
	createWeb06MainProtocol,
	type Web06OwnedEffectProof,
	type YuneWeb06DebugApi,
} from "./yune-integration/web06-main-protocol";
import {
	WEB06_MODE_QUERY,
} from "./yune-integration/private-protocol";
import {
	logWeb06ActionError,
	shouldLogWeb06ProductDebug,
} from "./yune-integration/web06-product-console";

type ListenerPayload = {
	[K in keyof ListenerArgsMap]: {
		type: "listener";
		name: K;
		args: ListenerArgsMap[K];
		web06?: {
			protocolVersion: typeof WEB06_PRIVATE_PROTOCOL_VERSION;
			actionId: string;
			sequenceId: number;
		};
	};
}[keyof ListenerArgsMap];

interface SuccessPayload {
	type: "success";
	result: ReturnType<Actions[keyof Actions]>;
	elapsedMs?: number;
	workerStartedAt?: number;
	workerFinishedAt?: number;
	workerBaseElapsedMs?: number;
	workerAmplificationMs?: number;
	workerActionMultiplier?: number;
}

interface ErrorPayload {
	type: "error";
	error: unknown;
	elapsedMs?: number;
	workerStartedAt?: number;
	workerFinishedAt?: number;
	workerBaseElapsedMs?: number;
	workerAmplificationMs?: number;
	workerActionMultiplier?: number;
}

interface DiagnosticPayload {
	type: "diagnostic";
	source: string;
	marker: unknown;
}

type Payload = ListenerPayload | SuccessPayload | ErrorPayload | DiagnosticPayload | Web06WorkerResponseEnvelope;

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
	workerBaseElapsedMs?: number;
	workerAmplificationMs?: number;
	workerActionMultiplier?: number;
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
	__YUNE_WEB06__?: YuneWeb06DebugApi;
};

let running: Message | null = null;
const queue: Message[] = [];

const allListenerTypes: (keyof Listeners)[] = [
	"deployStatusChanged",
	"schemaChanged",
	"grammarDiagnosticChanged",
	"optionChanged",
	"initialized",
];

const listeners = {} as { [K in keyof Listeners]: Listeners[K][] };
for (const type of allListenerTypes) {
	listeners[type] = [];
}
const lastListenerArgs = {} as Partial<{ [K in keyof ListenerArgsMap]: ListenerArgsMap[K] }>;

const YUNE_WEB_WORKER_VERSION = "yune-web-wasm-heap-v1";
const debugWindow = window as DebugWindow;
debugWindow.__YUNE_RIME_VERSION__ = YUNE_WEB_WORKER_VERSION;
document.documentElement.dataset["yuneRimeVersion"] = YUNE_WEB_WORKER_VERSION;
document.documentElement.dataset["yuneDeployStatus"] = "idle";
let worker: Worker;
const web06Protocol = createWeb06MainProtocol({
	queueState: () => ({
		queueDepth: queue.length + (running === null ? 0 : 1),
		runningIdentity: running?.web06,
	}),
	postClockPing: envelope => worker.postMessage(envelope),
});
const web06Mode = web06Protocol.mode;
const web06ModeProvenance = web06Protocol.modeProvenance;
installDebugHelpers();
worker = new Worker(workerUrl());
worker.addEventListener("message", (event: MessageEvent<Payload>) => {
	const mainResponseReceivedAt = web06Now();
	const { data } = event;
	if ("kind" in data && data.kind === "clock-echo") {
		web06Protocol.handleClockEcho(data, mainResponseReceivedAt);
		return;
	}
	if ("kind" in data && data.kind === "action-result") {
		const currentMessage = running;
		web06Protocol.handleActionResult(data, currentMessage?.web06, mainResponseReceivedAt);
		if (currentMessage === null) return;
		const legacyPayload: SuccessPayload | ErrorPayload = data.resultType === "success"
			? {
				type: "success",
				result: data.result as ReturnType<Actions[keyof Actions]>,
				elapsedMs: data.elapsedMs,
				workerStartedAt: data.workerStartedAt,
				workerFinishedAt: data.workerFinishedAt,
				workerBaseElapsedMs: data.workerBaseElapsedMs,
				workerAmplificationMs: data.workerAmplificationMs,
				workerActionMultiplier: data.workerActionMultiplier,
			}
			: {
				type: "error",
				error: data.error,
				elapsedMs: data.elapsedMs,
				workerStartedAt: data.workerStartedAt,
				workerFinishedAt: data.workerFinishedAt,
				workerBaseElapsedMs: data.workerBaseElapsedMs,
				workerAmplificationMs: data.workerAmplificationMs,
				workerActionMultiplier: data.workerActionMultiplier,
			};
		completeAction(currentMessage, legacyPayload, nowMs());
		return;
	}
	if (data.type === "diagnostic") {
		if (data.source === "web06-observer-failure") {
			web06Protocol.invalidate("WORKER_OBSERVER_FAILURE", stringifyUnknown(data.marker));
		}
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
		if (name === "deployStatusChanged") {
			document.documentElement.dataset["yuneDeployStatus"] = String(args[0]);
		}
		if (name === "initialized") {
			document.documentElement.dataset["yuneInitialized"] = String(args[0]);
		}
		if (name === "schemaChanged") {
			document.documentElement.dataset["yuneActiveSchema"] = args[0];
			document.documentElement.dataset["yuneActiveSchemaName"] = args[1];
		}
		if (name === "grammarDiagnosticChanged") {
			document.documentElement.dataset["yuneGrammarDiagnostic"] = JSON.stringify(args[0]);
		}
		if (name === "optionChanged") {
			document.documentElement.dataset["yuneLastOptionChanged"] = `${args[0]}:${args[1]}`;
			document.documentElement.dataset[optionDatasetKey(String(args[0]))] = String(args[1]);
		}
		for (const listener of listeners[name]) {
			// @ts-expect-error Unactionable
			listener.apply(worker, args);
		}
		web06Protocol.recordListener(name, args, data.web06, running?.web06, mainResponseReceivedAt);
	}
	else if (running && (type === "success" || type === "error")) {
		web06Protocol.invalidate("LEGACY_WORKER_RESULT", "Worker returned an unversioned action result", running.web06);
		const currentMessage = running;
		const receivedAt = nowMs();
		completeAction(currentMessage, data, receivedAt);
	}
});

function completeAction(
	currentMessage: Message,
	data: SuccessPayload | ErrorPayload,
	receivedAt: number,
) {
	const { resolve, reject } = currentMessage;
	const diagnostic = {
		action: currentMessage.name,
		input: typeof currentMessage.args[0] === "string" ? currentMessage.args[0] : undefined,
		enqueuedAt: currentMessage.enqueuedAt,
		sentAt: currentMessage.sentAt,
		receivedAt,
		workerStartedAt: data.workerStartedAt,
		workerFinishedAt: data.workerFinishedAt,
		workerBaseElapsedMs: data.workerBaseElapsedMs,
		workerAmplificationMs: data.workerAmplificationMs,
		workerActionMultiplier: data.workerActionMultiplier,
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
	if (data.type === "success") {
		appendLastActionResult(currentMessage.name, data.result);
		resolve(data.result as never);
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

function nowMs() {
	return performance.timeOrigin + performance.now();
}

function web06Now() {
	return performance.now();
}

function optionDatasetKey(option: string): string {
	return `yuneOption${option
		.split(/[_-]+/)
		.filter(Boolean)
		.map(part => `${part[0]?.toUpperCase() ?? ""}${part.slice(1)}`)
		.join("")}`;
}


function postMessage(message: Message) {
	if (shouldLogDebugMessages()) console.log("post", JSON.stringify({ name: message.name, args: message.args }));
	message.sentAt = nowMs();
	const { name, args } = running = message;
	const envelope = {
		protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
		kind: "action",
		mode: web06Mode,
		modeProvenance: web06ModeProvenance,
		identity: web06Protocol.markActionSent(message.web06),
		name,
		args,
	} as const;
	worker.postMessage(envelope);
}

function shouldLogDebugMessages() {
	return shouldLogWeb06ProductDebug(web06Mode, import.meta.env.DEV, location.search);
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
	logWeb06ActionError(web06Mode, diagnostic);
}

function appendLastActionResult(action: keyof Actions, result: unknown) {
	if (IS_PUBLIC_DEMO) {
		return;
	}
	const payload = { action, result, recordedAt: new Date().toISOString() };
	document.documentElement.dataset["yuneLastActionResult"] = stringifyUnknown(payload);
	if (action === "deployCacheSnapshot" || action === "invalidateDeployCache") {
		const cache = result as { cacheFresh?: unknown };
		document.documentElement.dataset["yuneDeployCacheFresh"] = String(cache.cacheFresh ?? "");
	}
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
	debugWindow.__YUNE_WEB06__ = web06Protocol.debugApi;
}


// Keep the public application seam stable while the owning protocol module
// retains all WEB-06 state, receipt, fanout, and terminal logic.
export function invalidateWeb06Measurement(code: string, detail: string, identity?: Web06ActionIdentity): void {
	web06Protocol.invalidate(code, detail, identity);
}

export function observeWeb06Measurement<T>(
	operation: string,
	identity: Partial<Pick<Web06ActionIdentity, "actionId" | "eventId">> | undefined,
	callback: () => T,
): T | undefined {
	return web06Protocol.observeMeasurement(operation, identity, callback);
}

export function recordWeb06DomEvent(
	snapshot: Web06DomEventSnapshot,
	mapping: Web06EventMapResult,
	deliveredAtOverride?: number,
): Web06DomEventIdentity {
	return web06Protocol.recordDomEvent(snapshot, mapping, deliveredAtOverride ?? web06Now());
}

export function withWeb06ControlEvent<T>(event: Web06ControlEventLike, callback: () => T): T {
	return web06Protocol.withControlEvent(event, callback);
}

export function declareWeb06ControlFanout(reason: string, plannedActions: Web06FanoutAction[]): Web06DomEventIdentity {
	return web06Protocol.declareControlFanout(reason, plannedActions);
}

export function registerWeb06EventFanout(event: Web06DomEventIdentity, plannedActions: Web06FanoutAction[]): void {
	web06Protocol.registerEventFanout(event, plannedActions);
}

export function resolveWeb06DeferredFanoutAction(event: Web06DomEventIdentity, eventActionIndex: number, args: unknown[]): boolean {
	return web06Protocol.resolveDeferredFanoutAction(event, eventActionIndex, args);
}

export function cancelWeb06EventFanout(event: Web06DomEventIdentity, reason: string): void {
	web06Protocol.cancelEventFanout(event, reason);
}

export function withWeb06OwnedAction<T>(
	owner: string,
	name: keyof Actions,
	args: unknown[],
	backgroundReason: string,
	causedBy: Web06ActionIdentity | undefined,
	action: () => T,
): T {
	return web06Protocol.withOwnedAction(owner, name, args, backgroundReason, causedBy, action);
}

export function web06MappedActionContext(
	event: Web06DomEventIdentity,
	action: Web06MappedAction,
	eventActionIndex: number,
	rawKey: string,
): Web06ActionContext {
	return web06Protocol.mappedActionContext(event, action, eventActionIndex, rawKey);
}

export function advanceWeb06Boundary(boundary: Web06ActionIdentity["boundary"]): void {
	web06Protocol.advanceBoundary(boundary);
}

export function withWeb06ActionContext<T>(context: Web06ActionContext, action: () => T): T {
	return web06Protocol.withActionContext(context, action);
}

export function web06ActionIdentityFor(promise: Promise<unknown>): Web06ActionIdentity | undefined {
	return web06Protocol.actionIdentityFor(promise);
}

export function recordWeb06ResponseMapping(identity: Web06ActionIdentity, startedAt: number, finishedAt: number): void {
	web06Protocol.recordResponseMapping(identity, startedAt, finishedAt);
}

export function recordWeb06PresentationOutcome(outcome: Web06PresentationOutcomeInput): void {
	web06Protocol.recordPresentationOutcome(outcome);
}

export function recordWeb06OwnedResultEffect(
	promise: Promise<unknown>,
	effect: "ui-userdb-refresh" | "ui-diagnostic-refresh" | "cache-invalidation",
	proof: Web06OwnedEffectProof,
): void {
	web06Protocol.recordOwnedResultEffect(promise, effect, proof);
}

export function canWeb06Supersede(from: Web06ActionIdentity, to: Web06ActionIdentity): boolean {
	return web06Protocol.canSupersede(from, to);
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
	"importUserdb",
	"processKey",
	"stageAi",
	"selectCandidate",
	"deleteCandidate",
	"flipPage",
	"customize",
	"customizeValue",
	"deploy",
	"deployCacheSnapshot",
	"invalidateDeployCache",
	"injectedAssetsManifest",
];

const Rime = {} as Actions;
for (const action of allActions) {
	Rime[action] = registerAction(action) as never;
}
export default Rime;

function registerAction<K extends keyof Actions>(name: K): Actions[K] {
	// @ts-expect-error The indexed Actions call signature is intentionally recovered per action.
	return (...args: Parameters<Actions[K]>) => {
		const identity = web06Protocol.createActionIdentity(name, args);
		const promise = new Promise((resolve, reject) => {
			const message: Message = { name, args, resolve, reject, enqueuedAt: nowMs(), web06: identity };
			if (running) queue.push(message);
			else postMessage(message);
		});
		web06Protocol.bindActionPromise(promise, identity);
		return promise;
	};
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

function workerUrl() {
	const params = new URLSearchParams({
		v: YUNE_WEB_WORKER_VERSION,
		schema: initialWorkerSchema(),
	});
	// Preserve selector provenance across realms. Omitting the selector is the
	// production-default minimal lane; spelling `minimal` in the worker URL
	// would incorrectly turn it into the explicit-minimal lane.
	if (new URLSearchParams(location.search).has(WEB06_MODE_QUERY)) {
		params.set(WEB06_MODE_QUERY, web06Mode);
	}
	const attributionFamily = wasmAttributionFamily();
	if (attributionFamily) {
		params.set("assetFamily", attributionFamily);
	}
	const latencyWorkerActionMultiplier = new URLSearchParams(location.search)
		.get("yuneLatencyWorkerActionMultiplier");
	if (
		latencyWorkerActionMultiplier
		&& ["127.0.0.1", "localhost", "::1", "[::1]"].includes(location.hostname)
	) {
		params.set("latencyWorkerActionMultiplier", latencyWorkerActionMultiplier);
	}
	return `./worker.js?${params.toString()}`;
}

function wasmAttributionFamily() {
	const params = new URLSearchParams(location.search);
	return params.get("wasmAttributionFamily");
}

function initialWorkerSchema(): RimeSchemaId {
	try {
		const stored = window.localStorage?.getItem("activeSchema");
		return isRimeSchemaId(stored) ? stored : "jyut6ping3";
	}
	catch {
		return "jyut6ping3";
	}
}

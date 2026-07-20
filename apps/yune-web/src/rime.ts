import type {
	Actions,
	ListenerArgsMap,
	Message,
	RimeSchemaId,
	Web06ActionContext,
	Web06ActionIdentity,
	Web06ActionReceipt,
	Web06ClockEchoEnvelope,
	Web06ClockExchange,
	Web06ClockPingEnvelope,
	Web06CollectionMode,
	Web06ControlEventLike,
	Web06DomEventIdentity,
	Web06DomEventSnapshot,
	Web06EventMapResult,
	Web06FanoutAction,
	Web06MappedAction,
	Web06MeasurementInvalidation,
	Web06PresentationOutcomeReceipt,
	Web06WorkerResponseEnvelope,
} from "./types";
import { WEB06_PRIVATE_PROTOCOL_VERSION } from "./types";
import { IS_PUBLIC_DEMO, isRimeSchemaId } from "./consts";
import {
	BoundedReceiptMap,
	WEB06_FULL_RECEIPT_CAPACITY,
	WEB06_MODE_QUERY,
	web06ActionContract,
	web06ActionIdentitiesEqual,
	web06CollectionMode,
	web06PrivateActionArgs,
	web06PrivateMappedAction,
	web06ReceiptCapacity,
	web06TimestampsAreOrdered,
	snapshotWeb06ControlEvent,
} from "./yune-integration/private-protocol";

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

interface Web06EventReceipt {
	identity: Web06DomEventIdentity;
	mappedActions: Web06MappedAction[];
	linkedActionIds: string[];
}

interface Web06PendingFanout {
	fanout: Web06FanoutState;
	eventActionIndex: number;
	action: Web06MappedAction;
}

interface Web06FanoutState {
	fanoutId: string;
	event: Web06DomEventIdentity;
	nextActionIndex: number;
	totalActions: number;
}

interface Web06ControlEventDraft {
	snapshot: Web06DomEventSnapshot;
	deliveredAt: number;
	declared: boolean;
}

interface YuneWeb06DebugApi {
	readonly protocolVersion: typeof WEB06_PRIVATE_PROTOCOL_VERSION;
	readonly mode: Web06CollectionMode;
	status(): {
		valid: boolean;
		queueDepth: number;
		runningActionId?: string;
		lastEventSequenceId: number;
		lastActionSequenceId: number;
		receiptWindowStartEventSequenceId: number;
		receiptWindowStartActionSequenceId: number;
		pendingFanoutActions: number;
	};
	events(): Web06EventReceipt[];
	actions(): Web06ActionReceipt[];
	invalidations(): Web06MeasurementInvalidation[];
	clockPing(): Promise<Web06ClockExchange>;
	resetReceipts(): void;
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
const web06Mode = web06CollectionMode(location.search);
const web06ActionReceipts = new BoundedReceiptMap<Web06ActionReceipt>(web06ReceiptCapacity(web06Mode));
const web06ActionIdentityByPromise = new WeakMap<Promise<unknown>, Web06ActionIdentity>();
const web06ActionIdentityBySequence = new Map<number, Web06ActionIdentity>();
const web06EventReceipts: Web06EventReceipt[] = [];
const web06Invalidations: Web06MeasurementInvalidation[] = [];
const web06PendingClockPings = new Map<string, {
	resolve: (exchange: Web06ClockExchange) => void;
	reject: (reason: unknown) => void;
}>();
const web06PendingFanoutsByOwner = new Map<string, Web06PendingFanout[]>();
const web06AuditedIncompleteFanouts = new Set<string>();
let web06ScopedActionContext: Web06ActionContext | undefined;
let web06ScopedControlEvent: Web06ControlEventDraft | undefined;
let web06EventSequenceId = 0;
let web06ActionSequenceId = 0;
let web06FanoutSequenceId = 0;
let web06ControlCompositionEpochId = 1_000_000;
let web06ClockPingSequenceId = 0;
let web06LastEventTimestamp = Number.NEGATIVE_INFINITY;
let web06LastSentSequenceId = 0;
let web06LastCompletedSequenceId = 0;
let web06ReceiptWindowStartEventSequenceId = 1;
let web06ReceiptWindowStartActionSequenceId = 1;

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
installDebugHelpers();
const worker = new Worker(workerUrl());
worker.addEventListener("message", (event: MessageEvent<Payload>) => {
	const mainResponseReceivedAt = web06Now();
	const { data } = event;
	if ("kind" in data && data.kind === "clock-echo") {
		handleWeb06ClockEcho(data, mainResponseReceivedAt);
		return;
	}
	if ("kind" in data && data.kind === "action-result") {
		handleWeb06ActionResult(data, mainResponseReceivedAt);
		return;
	}
	if (data.type === "diagnostic") {
		if (data.source === "web06-observer-failure") {
			web06Invalidate("WORKER_OBSERVER_FAILURE", stringifyUnknown(data.marker));
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
	}
	else if (running && (type === "success" || type === "error")) {
		web06Invalidate("LEGACY_WORKER_RESULT", "Worker returned an unversioned action result", running.web06);
		const currentMessage = running;
		const receivedAt = nowMs();
		completeAction(currentMessage, data, receivedAt, mainResponseReceivedAt);
	}
});

function handleWeb06ClockEcho(data: Web06ClockEchoEnvelope, mainReceivedAt: number) {
	const pending = web06PendingClockPings.get(data.pingId);
	if (pending === undefined) {
		web06Invalidate("ORPHAN_CLOCK_ECHO", `Unknown worker clock echo ${data.pingId}`);
		return;
	}
	web06PendingClockPings.delete(data.pingId);
	const netRtt = (mainReceivedAt - data.mainSentAt) - (data.workerSentAt - data.workerReceivedAt);
	const offset = ((data.workerReceivedAt - data.mainSentAt) + (data.workerSentAt - mainReceivedAt)) / 2;
	const exchange = {
		pingId: data.pingId,
		mainSentAt: data.mainSentAt,
		workerReceivedAt: data.workerReceivedAt,
		workerSentAt: data.workerSentAt,
		mainReceivedAt,
		offset,
		netRtt,
		uncertainty: netRtt / 2,
	} satisfies Web06ClockExchange;
	if (!Object.values(exchange).every(value => typeof value === "string" || Number.isFinite(value)) || netRtt < 0) {
		const error = new Error(`Invalid worker clock exchange ${data.pingId}`);
		web06Invalidate("INVALID_CLOCK_EXCHANGE", error.message);
		pending.reject(error);
		return;
	}
	pending.resolve(exchange);
}

function handleWeb06ActionResult(data: Extract<Web06WorkerResponseEnvelope, { kind: "action-result" }>, mainResponseReceivedAt: number) {
	const currentMessage = running;
	if (currentMessage === null) {
		web06Invalidate("ORPHAN_ACTION_RESULT", `Worker returned ${data.identity.actionId} with no running action`, data.identity);
		return;
	}
	if (data.protocolVersion !== WEB06_PRIVATE_PROTOCOL_VERSION || data.mode !== web06Mode) {
		web06Invalidate(
			"WIRE_PROVENANCE_MISMATCH",
			`Expected ${WEB06_PRIVATE_PROTOCOL_VERSION}/${web06Mode}, got ${data.protocolVersion}/${data.mode}`,
			currentMessage.web06,
		);
	}
	if (!web06ActionIdentitiesEqual(data.identity, currentMessage.web06)) {
		web06Invalidate(
			"ACTION_RESULT_IDENTITY_MISMATCH",
			`Worker did not round-trip the complete private identity for ${currentMessage.web06.actionId}`,
			currentMessage.web06,
		);
	}
	if (data.identity.sequenceId <= web06LastCompletedSequenceId) {
		web06Invalidate("DUPLICATE_ACTION_RESULT", `Duplicate action result ${data.identity.sequenceId}`, data.identity);
	}
	else if (data.identity.sequenceId !== web06LastCompletedSequenceId + 1) {
		web06Invalidate(
			"REORDERED_ACTION_RESULT",
			`Expected completed sequence ${web06LastCompletedSequenceId + 1}, got ${data.identity.sequenceId}`,
			data.identity,
		);
	}
	web06LastCompletedSequenceId = Math.max(web06LastCompletedSequenceId, data.identity.sequenceId);
	const receipt = web06ActionReceipts.get(currentMessage.web06.sequenceId);
	if (receipt !== undefined) {
		receipt.returnedIdentity = cloneForExport(data.identity);
		receipt.mainResponseReceivedAt = mainResponseReceivedAt;
		receipt.worker = data.receipt;
		receipt.resultType = data.resultType;
		web06ActionReceipts.set(receipt);
	}
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
	completeAction(currentMessage, legacyPayload, nowMs(), mainResponseReceivedAt);
}

function completeAction(
	currentMessage: Message,
	data: SuccessPayload | ErrorPayload,
	receivedAt: number,
	mainResponseReceivedAt: number,
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
		const receipt = web06ActionReceipts.get(currentMessage.web06.sequenceId);
		if (receipt !== undefined) {
			receipt.mainResponseReceivedAt = mainResponseReceivedAt;
			receipt.resultType = "error";
			web06ActionReceipts.set(receipt);
		}
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
	message.web06.workerDispatchDepth = 1;
	if (message.web06.sequenceId <= web06LastSentSequenceId) {
		web06Invalidate("DUPLICATE_ACTION_SEND", `Duplicate worker send ${message.web06.sequenceId}`, message.web06);
	}
	else if (message.web06.sequenceId !== web06LastSentSequenceId + 1) {
		web06Invalidate(
			"REORDERED_ACTION_SEND",
			`Expected send sequence ${web06LastSentSequenceId + 1}, got ${message.web06.sequenceId}`,
			message.web06,
		);
	}
	web06LastSentSequenceId = Math.max(web06LastSentSequenceId, message.web06.sequenceId);
	const { name, args } = running = message;
	const envelope = {
		protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
		kind: "action",
		mode: web06Mode,
		identity: message.web06,
		name,
		args,
	} as const;
	message.web06.workerSentAt = web06Now();
	worker.postMessage(envelope);
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
	debugWindow.__YUNE_WEB06__ = {
		protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
		mode: web06Mode,
		status: () => ({
			valid: web06Invalidations.length === 0,
			queueDepth: queue.length + (running === null ? 0 : 1),
			runningActionId: running?.web06.actionId,
			lastEventSequenceId: web06EventSequenceId,
			lastActionSequenceId: web06ActionSequenceId,
			receiptWindowStartEventSequenceId: web06ReceiptWindowStartEventSequenceId,
			receiptWindowStartActionSequenceId: web06ReceiptWindowStartActionSequenceId,
			pendingFanoutActions: web06PendingFanoutActionCount(),
		}),
		events: () => cloneForExport(web06EventReceipts),
		actions: () => {
			auditWeb06PendingFanouts();
			return cloneForExport(web06ActionReceipts.values());
		},
		invalidations: () => {
			auditWeb06PendingFanouts();
			return cloneForExport(web06Invalidations);
		},
		clockPing: requestWeb06WorkerClockExchange,
		resetReceipts: resetWeb06Receipts,
	};
}

function cloneForExport<T>(value: T): T {
	return typeof structuredClone === "function"
		? structuredClone(value)
		: JSON.parse(JSON.stringify(value)) as T;
}

function resetWeb06Receipts() {
	if (running !== null || queue.length > 0 || web06PendingClockPings.size > 0) {
		web06Invalidate("RESET_WHILE_BUSY", "WEB-06 receipts may reset only while the action queue and clock lane are idle");
		return;
	}
	auditWeb06PendingFanouts();
	if (web06PendingFanoutActionCount() > 0) {
		return;
	}
	web06ActionReceipts.clear();
	web06EventReceipts.length = 0;
	web06Invalidations.length = 0;
	web06ReceiptWindowStartEventSequenceId = web06EventSequenceId + 1;
	web06ReceiptWindowStartActionSequenceId = web06ActionSequenceId + 1;
	web06AuditedIncompleteFanouts.clear();
}

function requestWeb06WorkerClockExchange(): Promise<Web06ClockExchange> {
	if (running !== null || queue.length > 0) {
		return Promise.reject(new Error("WEB-06 worker clock calibration requires an idle action queue"));
	}
	if (web06PendingClockPings.size >= 32) {
		web06Invalidate("CLOCK_PING_RING_OVERFLOW", "More than 32 worker clock pings were pending");
		return Promise.reject(new Error("WEB-06 worker clock ping capacity exceeded"));
	}
	const pingId = `web06-clock-${String(++web06ClockPingSequenceId).padStart(6, "0")}`;
	const mainSentAt = web06Now();
	const envelope: Web06ClockPingEnvelope = {
		protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
		kind: "clock-ping",
		pingId,
		mainSentAt,
	};
	return new Promise((resolve, reject) => {
		web06PendingClockPings.set(pingId, { resolve, reject });
		worker.postMessage(envelope);
	});
}

function web06Invalidate(
	code: string,
	detail: string,
	identity?: Partial<Pick<Web06ActionIdentity, "actionId" | "eventId">>,
) {
	try {
		if (web06Invalidations.length >= 256) {
			const last = web06Invalidations[255];
			if (last?.code !== "INVALIDATION_RING_OVERFLOW") {
				web06Invalidations[255] = {
					code: "INVALIDATION_RING_OVERFLOW",
					detail: "WEB-06 invalidation ring exceeded its fixed 256-record capacity",
					recordedAt: web06Now(),
				};
			}
			return;
		}
		web06Invalidations.push({
			code,
			detail,
			recordedAt: web06Now(),
			actionId: identity?.actionId,
			eventId: identity?.eventId,
		});
	}
	catch {
		// Measurement failure must never replace the product action result/error.
	}
}

export function invalidateWeb06Measurement(
	code: string,
	detail: string,
	identity?: Web06ActionIdentity,
) {
	web06Invalidate(code, detail, identity);
}

export function recordWeb06DomEvent(
	snapshot: Web06DomEventSnapshot,
	mapping: Web06EventMapResult,
	compositionEpochId: number,
	supersessionSubRunId: number,
	deliveredAtOverride?: number,
): Web06DomEventIdentity {
	const deliveredAt = deliveredAtOverride ?? web06Now();
	if (!Number.isFinite(snapshot.timeStamp) || snapshot.timeStamp < 0) {
		web06Invalidate("INVALID_EVENT_TIMESTAMP", `Invalid ${snapshot.type} timeStamp ${snapshot.timeStamp}`);
	}
	if (snapshot.timeStamp < web06LastEventTimestamp) {
		web06Invalidate(
			"DECREASING_EVENT_TIMESTAMP",
			`Event timestamp decreased from ${web06LastEventTimestamp} to ${snapshot.timeStamp}`,
		);
	}
	web06LastEventTimestamp = Math.max(web06LastEventTimestamp, snapshot.timeStamp);
	const eventSequenceId = ++web06EventSequenceId;
	const identity: Web06DomEventIdentity = {
		protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
		eventId: `web06-event-${String(eventSequenceId).padStart(8, "0")}`,
		eventSequenceId,
		eventDeliveredAt: deliveredAt,
		classification: mapping.classification,
		reason: mapping.reason,
		mappedActionCount: mapping.actions.length,
		compositionEpochId,
		supersessionSubRunId,
		...snapshot,
	};
	if (web06Mode !== "off") {
		web06EventReceipts.push({
			identity,
			mappedActions: cloneForExport(mapping.actions.map(web06PrivateMappedAction)),
			linkedActionIds: [],
		});
		const capacity = web06ReceiptCapacity(web06Mode);
		if (web06EventReceipts.length > capacity) {
			web06Invalidate(
				"EVENT_RING_OVERFLOW",
				`WEB-06 event ring exceeded its fixed ${capacity}-record capacity`,
			);
			web06EventReceipts.splice(0, web06EventReceipts.length - capacity);
		}
	}
	return identity;
}

export function withWeb06ControlEvent<T>(event: Web06ControlEventLike, callback: () => T): T {
	const deliveredAt = web06Now();
	if (web06ScopedControlEvent !== undefined) {
		web06Invalidate("NESTED_CONTROL_EVENT", "A WEB-06 control event scope was nested");
	}
	const draft: Web06ControlEventDraft = {
		snapshot: snapshotWeb06ControlEvent(event),
		deliveredAt,
		declared: false,
	};
	web06ScopedControlEvent = draft;
	try {
		return callback();
	} finally {
		if (!draft.declared) {
			recordWeb06DomEvent(
				draft.snapshot,
				{
					classification: "frontend-consumed",
					reason: "control-event-without-engine-action",
					preventDefault: false,
					actions: [],
				},
				++web06ControlCompositionEpochId,
				0,
				draft.deliveredAt,
			);
		}
		web06ScopedControlEvent = undefined;
	}
}

export function declareWeb06ControlFanout(
	reason: string,
	plannedActions: Web06FanoutAction[],
): Web06DomEventIdentity {
	const draft = web06ScopedControlEvent;
	if (draft === undefined) {
		throw new Error("WEB-06 control fanout must be declared inside withWeb06ControlEvent");
	}
	if (draft.declared) {
		throw new Error("WEB-06 control event declared more than one fanout");
	}
	draft.declared = true;
	const actions = plannedActions.map(planned => planned.action);
	const event = recordWeb06DomEvent(
		draft.snapshot,
		{
			classification: actions.length === 0 ? "frontend-consumed" : "mapped-action(s)",
			reason,
			preventDefault: false,
			actions,
		},
		++web06ControlCompositionEpochId,
		0,
		draft.deliveredAt,
	);
	registerWeb06EventFanout(event, plannedActions);
	return event;
}

export function registerWeb06EventFanout(
	event: Web06DomEventIdentity,
	plannedActions: Web06FanoutAction[],
): void {
	const receipt = web06EventReceipts.find(candidate => candidate.identity.eventId === event.eventId);
	if (web06Mode !== "off") {
		const declared = receipt?.mappedActions ?? [];
		if (
			declared.length !== plannedActions.length
			|| declared.some((action, index) => !sameAction(action, plannedActions[index]?.action))
		) {
			web06Invalidate(
				"FANOUT_DECLARATION_MISMATCH",
				`Fanout actions do not match event ${event.eventId}`,
				{ eventId: event.eventId },
			);
			return;
		}
	}
	if (plannedActions.length === 0) return;
	const fanout: Web06FanoutState = {
		fanoutId: `web06-fanout-${String(++web06FanoutSequenceId).padStart(8, "0")}`,
		event,
		nextActionIndex: 0,
		totalActions: plannedActions.length,
	};
	plannedActions.forEach((planned, eventActionIndex) => {
		const pending = web06PendingFanoutsByOwner.get(planned.owner) ?? [];
		pending.push({
			fanout,
			eventActionIndex,
			action: web06PrivateMappedAction(planned.action),
		});
		web06PendingFanoutsByOwner.set(planned.owner, pending);
	});
	if (web06PendingFanoutActionCount() > 256) {
		web06Invalidate(
			"FANOUT_RING_OVERFLOW",
			"WEB-06 pending control fanouts exceeded 256 actions",
			{ eventId: event.eventId },
		);
	}
}

export function resolveWeb06DeferredFanoutAction(
	event: Web06DomEventIdentity,
	eventActionIndex: number,
	args: unknown[],
): boolean {
	for (const pending of web06PendingFanoutsByOwner.values()) {
		const expected = pending.find(candidate =>
			candidate.fanout.event.eventId === event.eventId
			&& candidate.eventActionIndex === eventActionIndex
			);
			if (expected === undefined) continue;
		expected.action.args = cloneForExport(web06PrivateActionArgs(expected.action.name, args));
		const receipt = web06EventReceipts.find(candidate => candidate.identity.eventId === event.eventId);
		if (receipt !== undefined && receipt.mappedActions[eventActionIndex] !== undefined) {
			receipt.mappedActions[eventActionIndex]!.args = cloneForExport(
				web06PrivateActionArgs(expected.action.name, args),
			);
		}
		return true;
	}
	web06Invalidate(
		"DEFERRED_FANOUT_RESOLUTION_MISSING",
		`No pending fanout action ${event.eventId}/${eventActionIndex} accepted deferred arguments`,
		{ eventId: event.eventId },
	);
	return false;
}

export function cancelWeb06EventFanout(
	event: Web06DomEventIdentity,
	reason: string,
): void {
	let matched = false;
	for (const [owner, pending] of web06PendingFanoutsByOwner) {
		const retained: Web06PendingFanout[] = [];
		for (const expected of pending) {
			if (expected.fanout.event.eventId !== event.eventId) {
				retained.push(expected);
				continue;
			}
			matched = true;
			if (expected.fanout.nextActionIndex > 0) {
				web06Invalidate(
					"CANCELLED_PARTIAL_FANOUT",
					`Cannot cancel ${expected.fanout.fanoutId} after an action was consumed`,
					{ eventId: event.eventId },
				);
				retained.push(expected);
			}
		}
		if (retained.length === 0) web06PendingFanoutsByOwner.delete(owner);
		else web06PendingFanoutsByOwner.set(owner, retained);
	}
	if (!matched) return;
	event.classification = "frontend-consumed";
	event.reason = reason;
	event.mappedActionCount = 0;
	const receipt = web06EventReceipts.find(candidate => candidate.identity.eventId === event.eventId);
	if (receipt !== undefined) {
		receipt.mappedActions = [];
		receipt.linkedActionIds = [];
	}
}

export function withWeb06OwnedAction<T>(
	owner: string,
	name: keyof Actions,
	args: unknown[],
	backgroundReason: string,
	causedBy: Web06ActionIdentity | undefined,
	action: () => T,
): T {
	const pending = takeWeb06FanoutContext(owner, name, args);
	if (pending !== undefined) {
		return withWeb06ActionContext(pending, action);
	}
	const contract = web06ActionContract(name);
	return withWeb06ActionContext({
		compositionEpochId: causedBy?.compositionEpochId ?? ++web06ControlCompositionEpochId,
		supersessionSubRunId: causedBy?.supersessionSubRunId ?? 0,
		actionClass: contract.actionClass,
		supersedable: false,
		boundary: contract.boundary,
		rawInputSequence: causedBy === undefined ? [] : [...causedBy.rawInputSequence],
		originKind: "background",
		originReason: backgroundReason,
		causedByActionId: causedBy?.actionId,
		causedBySequenceId: causedBy?.sequenceId,
		causedByEventId: causedBy?.eventId,
		causedByEventSequenceId: causedBy?.eventSequenceId,
	}, action);
}

function takeWeb06FanoutContext(
	owner: string,
	name: keyof Actions,
	args: unknown[],
): Web06ActionContext | undefined {
	const pending = web06PendingFanoutsByOwner.get(owner);
	const expected = pending?.[0];
	if (expected === undefined) return undefined;
	if (
		expected.action.name !== name
		|| !sameArgs(expected.action.args, web06PrivateActionArgs(name, args))
	) {
		web06Invalidate(
			"FANOUT_OWNER_ACTION_MISMATCH",
			`${owner} expected ${expected.action.name} at fanout index ${expected.eventActionIndex}, got ${name}`,
			{ eventId: expected.fanout.event.eventId },
		);
		return undefined;
	}
	pending?.shift();
	if (pending?.length === 0) {
		web06PendingFanoutsByOwner.delete(owner);
	}
	if (expected.eventActionIndex !== expected.fanout.nextActionIndex) {
		web06Invalidate(
			"FANOUT_GLOBAL_ORDER_MISMATCH",
			`${expected.fanout.fanoutId} expected global index ${expected.fanout.nextActionIndex}, got ${expected.eventActionIndex}`,
			{ eventId: expected.fanout.event.eventId },
		);
	}
	expected.fanout.nextActionIndex = Math.max(
		expected.fanout.nextActionIndex,
		expected.eventActionIndex + 1,
	);
	return {
		event: expected.fanout.event,
		eventActionIndex: expected.eventActionIndex,
		compositionEpochId: expected.fanout.event.compositionEpochId,
		supersessionSubRunId: expected.fanout.event.supersessionSubRunId,
		actionClass: expected.action.actionClass,
		supersedable: expected.action.supersedable,
		boundary: expected.action.boundary,
		rawInputSequence: [],
		originKind: "dom-event",
		originReason: expected.fanout.event.reason,
	};
}

function sameAction(left: Web06MappedAction, right: Web06MappedAction | undefined): boolean {
	return right !== undefined
		&& left.name === right.name
		&& sameArgs(left.args, web06PrivateActionArgs(right.name, right.args))
		&& left.actionClass === right.actionClass
		&& left.supersedable === right.supersedable
		&& left.boundary === right.boundary;
}

function web06PendingFanoutActionCount(): number {
	let count = 0;
	for (const pending of web06PendingFanoutsByOwner.values()) {
		count += pending.length;
	}
	return count;
}

function auditWeb06PendingFanouts(): void {
	for (const pending of web06PendingFanoutsByOwner.values()) {
		for (const expected of pending) {
			if (web06AuditedIncompleteFanouts.has(expected.fanout.fanoutId)) continue;
			web06AuditedIncompleteFanouts.add(expected.fanout.fanoutId);
			web06Invalidate(
				"UNCONSUMED_CONTROL_FANOUT",
				`${expected.fanout.fanoutId} consumed ${expected.fanout.nextActionIndex}/${expected.fanout.totalActions} actions`,
				{ eventId: expected.fanout.event.eventId },
			);
		}
	}
}

export function withWeb06ActionContext<T>(context: Web06ActionContext, action: () => T): T {
	if (web06ScopedActionContext !== undefined) {
		web06Invalidate("NESTED_ACTION_CONTEXT", "A WEB-06 action context was nested before enqueue");
	}
	web06ScopedActionContext = context;
	try {
		return action();
	}
	finally {
		web06ScopedActionContext = undefined;
	}
}

export function web06ActionIdentityFor(promise: Promise<unknown>): Web06ActionIdentity | undefined {
	return web06ActionIdentityByPromise.get(promise);
}

export function recordWeb06ResponseMapping(
	identity: Web06ActionIdentity,
	responseMappingStartedAt: number,
	responseMappingFinishedAt: number,
) {
	observeWeb06("record-response-mapping", identity, () => {
		const receipt = web06ActionReceipts.get(identity.sequenceId);
		if (receipt === undefined) return;
		if (!web06TimestampsAreOrdered(responseMappingStartedAt, responseMappingFinishedAt)) {
			web06Invalidate(
				"RESPONSE_MAPPING_TIMESTAMP_ORDER",
				`Action ${identity.actionId} recorded invalid response-mapping timestamps`,
				identity,
			);
		}
		receipt.responseMappingStartedAt = responseMappingStartedAt;
		receipt.responseMappingFinishedAt = responseMappingFinishedAt;
		web06ActionReceipts.set(receipt);
	});
}

export function recordWeb06PresentationOutcome(outcome: Web06PresentationOutcomeReceipt) {
	observeWeb06("record-presentation-outcome", outcome.identity, () => {
		const receipt = web06ActionReceipts.get(outcome.identity.sequenceId);
		if (receipt === undefined) return;
		if (receipt.presentation !== undefined) {
			web06Invalidate(
				"DUPLICATE_TERMINAL_OUTCOME",
				`Action ${outcome.identity.actionId} already has a terminal outcome`,
				outcome.identity,
			);
			return;
		}
		if (!web06TimestampsAreOrdered(
			receipt.responseMappingFinishedAt,
			outcome.stateUpdateScheduledAt,
			outcome.stateCommittedAt,
			outcome.firstRafAt,
			outcome.terminalObservedAt,
		)) {
			web06Invalidate(
				"PRESENTATION_TIMESTAMP_ORDER",
				`Action ${outcome.identity.actionId} recorded out-of-order presentation timestamps`,
				outcome.identity,
			);
		}
		receipt.presentation = outcome;
		web06ActionReceipts.set(receipt);
	});
}

export function canWeb06Supersede(from: Web06ActionIdentity, to: Web06ActionIdentity): boolean {
	if (
		!from.supersedable
		|| !to.supersedable
		|| from.compositionEpochId !== to.compositionEpochId
		|| from.supersessionSubRunId !== to.supersessionSubRunId
		|| to.sequenceId <= from.sequenceId
		|| to.sequenceId - from.sequenceId > 2
		|| !strictArrayPrefix(from.rawInputSequence, to.rawInputSequence)
	) {
		return false;
	}
	for (let sequenceId = from.sequenceId + 1; sequenceId <= to.sequenceId; sequenceId += 1) {
		const identity = web06ActionIdentityBySequence.get(sequenceId);
		if (
			identity === undefined
			|| !identity.supersedable
			|| identity.compositionEpochId !== from.compositionEpochId
			|| identity.supersessionSubRunId !== from.supersessionSubRunId
		) {
			return false;
		}
	}
	return true;
}

function strictArrayPrefix(left: string[], right: string[]): boolean {
	return left.length < right.length && left.every((value, index) => right[index] === value);
}

function observeWeb06(operation: string, identity: Web06ActionIdentity, callback: () => void) {
	const startedAt = web06Now();
	try {
		callback();
	}
	catch (error) {
		web06Invalidate(
			"OBSERVER_CALLBACK_FAILURE",
			`${operation}: ${error instanceof Error ? `${error.name}: ${error.message}` : String(error)}`,
			identity,
		);
	}
	finally {
		const duration = web06Now() - startedAt;
		if (duration >= 5) {
			web06Invalidate(
				"OBSERVER_CALLBACK_OVER_5MS",
				`${operation} took ${duration} ms`,
				identity,
			);
		}
	}
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
	// @ts-expect-error Unactionable
	return (...args: Parameters<Actions[K]>) => {
		const context = takeWeb06ActionContext(name, args);
		const sequenceId = ++web06ActionSequenceId;
		const identity: Web06ActionIdentity = {
			protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
			actionId: `web06-action-${String(sequenceId).padStart(8, "0")}`,
			sequenceId,
			eventId: context.event?.eventId,
			eventSequenceId: context.event?.eventSequenceId,
			eventActionIndex: context.eventActionIndex,
			compositionEpochId: context.compositionEpochId,
			supersessionSubRunId: context.supersessionSubRunId,
			actionClass: context.actionClass,
			supersedable: context.supersedable,
			boundary: context.boundary,
			rawInputSequence: [...context.rawInputSequence],
			originKind: context.originKind ?? (context.event === undefined ? "background" : "dom-event"),
			originReason: context.originReason ?? context.event?.reason ?? `unclassified:${name}`,
			causedByActionId: context.causedByActionId,
			causedBySequenceId: context.causedBySequenceId,
			causedByEventId: context.causedByEventId,
			causedByEventSequenceId: context.causedByEventSequenceId,
			actionEnqueuedAt: web06Now(),
			mainQueueDepthAtEnqueue: queue.length + (running === null ? 0 : 1),
		};
		if (identity.originReason.startsWith("unclassified:")) {
			web06Invalidate(
				"UNCLASSIFIED_ACTION_ORIGIN",
				`Action ${identity.actionId} has no explicit DOM-event or background origin`,
				identity,
			);
		}
		web06ActionIdentityBySequence.set(sequenceId, identity);
		while (web06ActionIdentityBySequence.size > WEB06_FULL_RECEIPT_CAPACITY) {
			web06Invalidate(
				"ACTION_IDENTITY_RING_OVERFLOW",
				`WEB-06 action identity ring exceeded ${WEB06_FULL_RECEIPT_CAPACITY} records`,
				identity,
			);
			const first = web06ActionIdentityBySequence.keys().next().value as number | undefined;
			if (first === undefined) break;
			web06ActionIdentityBySequence.delete(first);
		}
		linkWeb06ActionToEvent(identity, name);
			const receiptOverflowed = web06ActionReceipts.set({
				identity,
				name,
				args: cloneForExport(web06PrivateActionArgs(name, args)),
		});
		if (receiptOverflowed) {
			web06Invalidate(
				"ACTION_RECEIPT_RING_OVERFLOW",
				`WEB-06 action receipt ring exceeded ${web06ReceiptCapacity(web06Mode)} records`,
				identity,
			);
		}
		const promise = new Promise((resolve, reject) => {
			const message: Message = { name, args, resolve, reject, enqueuedAt: nowMs(), web06: identity };
			if (running) {
				queue.push(message);
			}
			else {
				postMessage(message);
			}
		});
		web06ActionIdentityByPromise.set(promise, identity);
		return promise;
	};
}

function takeWeb06ActionContext<K extends keyof Actions>(
	name: K,
	args: Parameters<Actions[K]>,
): Web06ActionContext {
	if (web06ScopedActionContext !== undefined) {
		return web06ScopedActionContext;
	}
	const contract = web06ActionContract(name);
	return {
		compositionEpochId: 0,
		supersessionSubRunId: 0,
		actionClass: contract.actionClass,
		supersedable: false,
		boundary: contract.boundary,
		rawInputSequence: [],
		originKind: "background",
		originReason: `unclassified:${name}`,
	};
}

function sameArgs(left: unknown[], right: readonly unknown[]): boolean {
	try {
		return JSON.stringify(left) === JSON.stringify(right);
	}
	catch {
		return false;
	}
}

function linkWeb06ActionToEvent(identity: Web06ActionIdentity, name: keyof Actions) {
	if (identity.eventId === undefined) return;
	const event = web06EventReceipts.find(receipt => receipt.identity.eventId === identity.eventId);
	if (event === undefined) {
		if (web06Mode !== "off") {
			web06Invalidate("ORPHANED_ACTION_EVENT", `Action ${identity.actionId} references missing event ${identity.eventId}`, identity);
		}
		return;
	}
	const index = identity.eventActionIndex;
	if (index === undefined || index < 0 || index >= event.identity.mappedActionCount) {
		web06Invalidate("ACTION_INDEX_OUT_OF_RANGE", `Action ${identity.actionId} has invalid event index ${index}`, identity);
		return;
	}
	if (event.linkedActionIds[index] !== undefined) {
		web06Invalidate("DUPLICATE_EVENT_ACTION", `Event ${event.identity.eventId} action ${index} was linked twice`, identity);
		return;
	}
	const expected = event.mappedActions[index];
	if (expected?.name !== name) {
		web06Invalidate(
			"EVENT_ACTION_KIND_MISMATCH",
			`Event ${event.identity.eventId} expected ${expected?.name ?? "missing"}, got ${name}`,
			identity,
		);
	}
	event.linkedActionIds[index] = identity.actionId;
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
		[WEB06_MODE_QUERY]: web06Mode,
	});
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

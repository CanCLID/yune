import type {
	Actions,
	ListenerArgsMap,
	Web06ActionContext,
	Web06ActionIdentity,
	Web06ActionReceipt,
	Web06ClockEchoEnvelope,
	Web06ClockExchange,
	Web06ClockPingEnvelope,
	Web06CollectionMode,
	Web06CollectionModeProvenance,
	Web06ControlEventLike,
	Web06DomEventIdentity,
	Web06DomEventSnapshot,
	Web06EventMapResult,
	Web06FanoutAction,
	Web06LifecycleOutcomeReceipt,
	Web06MappedAction,
	Web06MeasurementInvalidation,
	Web06PresentationOutcomeInput,
	Web06PresentationOutcomeReceipt,
	Web06WorkerLifecycleEffect,
	Web06WorkerResponseEnvelope,
} from "../types";
import { WEB06_PRIVATE_PROTOCOL_VERSION } from "../types";
import {
	BoundedReceiptMap,
	web06ActionContract,
	web06ActionIdentitiesEqual,
	web06AdapterProjectionFingerprintsEqual,
	web06CollectionMode,
	web06CollectionModeProvenance,
	web06ExpectedActionFailure,
	web06PresentationFingerprintDigest,
	web06PresentationFingerprintsEqual,
	web06PresentationStateDigest,
	web06ReceiptCapacity,
	web06RetainedActionArgs,
	web06RetainedActionIdentity,
	web06RetainedEventIdentity,
	web06RetainedMappedAction,
	web06StableDigest,
	web06TerminalContract,
	web06TimestampsAreOrdered,
	web06ValuesEqual,
	snapshotWeb06ControlEvent,
} from "./private-protocol";

interface Web06EventReceipt {
	identity: Web06DomEventIdentity;
	mappedActions: Web06MappedAction[];
	linkedActionIds: string[];
}

interface Web06PendingFanout {
	fanout: Web06FanoutState;
	eventActionIndex: number;
	action: Web06MappedAction;
	deferredResolved: boolean;
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

interface Web06ReloadContinuityToken {
	measurementId: string;
	continuityNonce: string;
}

interface Web06ReloadContinuityPreReceipt extends Web06ReloadContinuityToken {
	phase: "pre-reload";
	protocolVersion: typeof WEB06_PRIVATE_PROTOCOL_VERSION;
	pageInstanceId: string;
	timeOrigin: number;
	preparedAt: number;
	receiptWindowStartEventSequenceId: number;
	receiptWindowStartActionSequenceId: number;
	lastEventSequenceId: number;
	lastActionSequenceId: number;
	lastSentSequenceId: number;
	lastCompletedSequenceId: number;
	compositionEpochId: number;
	supersessionSubRunId: number;
	terminal: {
		actionId: string;
		sequenceId: number;
		outcome: "committed";
		terminalObservedAt: number;
		presentationDigest: string;
		committedTextDigest: string;
		committedUtf16Length: number;
		persistenceCompleted: true;
	};
	userdb: {
		actionId: string;
		sequenceId: number;
		digest: string;
		rowCount: number;
		bytes: number;
	};
	queueIdle: true;
	allActionsCompleted: true;
	storagePayloadKeys: readonly ["measurementId", "continuityNonce"];
}

interface Web06ReloadContinuityPostReceipt extends Web06ReloadContinuityToken {
	phase: "post-reload-arrived" | "post-reload-bound";
	protocolVersion: typeof WEB06_PRIVATE_PROTOCOL_VERSION;
	pageInstanceId: string;
	timeOrigin: number;
	consumedAt: number;
	storageRemoved: true;
	oneShot: true;
	boundAt?: number;
	receiptWindowStartEventSequenceId?: number;
	receiptWindowStartActionSequenceId?: number;
	lastEventSequenceId?: number;
	lastActionSequenceId?: number;
	compositionEpochId?: number;
	supersessionSubRunId?: number;
	requiresFreshDriverPageCalibration?: true;
	requiresFreshWorkerCalibration?: true;
}

export interface Web06DebugStatus {
	valid: boolean;
	queueDepth: number;
	runningActionId?: string;
	lastEventSequenceId: number;
	lastActionSequenceId: number;
	receiptWindowStartEventSequenceId: number;
	receiptWindowStartActionSequenceId: number;
	pendingFanoutActions: number;
	pendingTerminalActions: number;
	pageInstanceId: string;
	measurementId?: string;
	continuityNonce?: string;
	reloadContinuityPhase?: Web06ReloadContinuityPreReceipt["phase"] | Web06ReloadContinuityPostReceipt["phase"];
	compositionEpochId: number;
	supersessionSubRunId: number;
	mainObserverCallbackCount: number;
	mainObserverCallbackCapacity: number;
	mainObserverCallbackOverflowCount: number;
	workerObserverFailureCount: number;
	workerObserverFailuresAudited: boolean;
}

export interface Web06MainObserverCallbackInterval {
	callbackId: string;
	sequenceId: number;
	operation: string;
	startedAt: number;
	finishedAt: number;
	durationMs: number;
	actionId?: string;
	eventId?: string;
}

interface Web06MainListenerEffect {
	name: keyof ListenerArgsMap;
	argsDigest: string;
	args: unknown[];
	stateKey: string;
	expectedState: Record<string, unknown>;
	readState: () => Record<string, unknown>;
	appliedAt: number;
}

export interface Web06OwnedEffectProof {
	expectedState: Record<string, unknown>;
	readObservedState(): Record<string, unknown> | undefined;
}

interface Web06PendingLifecycle {
	identity: Web06ActionIdentity;
	name: keyof Actions;
	workerEffectDigest: string;
	mainEffectDigest?: string;
	listenerEffectCount: number;
	persistenceCompleted: boolean;
	expectedOwnerEffect?: "ui-userdb-refresh" | "ui-diagnostic-refresh" | "cache-invalidation";
	listenerEffects?: Web06MainListenerEffect[];
	ownerProof?: Web06OwnedEffectProof;
	scheduled: boolean;
}

interface Web06PresentationTerminalProof {
	outcome: Web06PresentationOutcomeReceipt["outcome"];
	expected: Web06PresentationOutcomeInput["presentationExpected"];
	observed: Web06PresentationOutcomeInput["domObserved"];
}

interface Web06AtomicSnapshot {
	header: {
		protocolVersion: typeof WEB06_PRIVATE_PROTOCOL_VERSION;
		mode: Web06CollectionMode;
		modeProvenance: Web06CollectionModeProvenance;
		pageInstanceId: string;
		timeOrigin: number;
		measurementId?: string;
		continuityNonce?: string;
	};
	status: Web06DebugStatus;
	events: Web06EventReceipt[];
	actions: Web06ActionReceipt[];
	invalidations: Web06MeasurementInvalidation[];
	mainObserverCallbacks: Web06MainObserverCallbackInterval[];
	mainObserverCallbacksMs: number[];
	reloadContinuity?: Web06ReloadContinuityPreReceipt | Web06ReloadContinuityPostReceipt;
}

export interface YuneWeb06DebugApi {
	readonly protocolVersion: typeof WEB06_PRIVATE_PROTOCOL_VERSION;
	readonly mode: Web06CollectionMode;
	readonly modeProvenance: Web06CollectionModeProvenance;
	status(): Web06DebugStatus;
	events(): Web06EventReceipt[];
	actions(): Web06ActionReceipt[];
	invalidations(): Web06MeasurementInvalidation[];
	mainObserverCallbacks(): Web06MainObserverCallbackInterval[];
	mainObserverCallbacksMs(): number[];
	snapshot(): Web06AtomicSnapshot;
	clockPing(): Promise<Web06ClockExchange>;
	resetReceipts(): void;
	prepareLearnedReloadContinuity(
		measurementId: string,
		expectedTerminalActionId?: string,
	): Web06ReloadContinuityPreReceipt;
	bindLearnedReloadWindow(
		measurementId: string,
		continuityNonce: string,
	): Web06ReloadContinuityPostReceipt;
}

export interface Web06MainProtocolHooks {
	queueState(): { queueDepth: number; runningIdentity?: Web06ActionIdentity };
	postClockPing(envelope: Web06ClockPingEnvelope): void;
}

export function createWeb06MainProtocol(hooks: Web06MainProtocolHooks) {
	const mode = web06CollectionMode(location.search);
	const modeProvenance = web06CollectionModeProvenance(location.search);
	const receiptCapacity = web06ReceiptCapacity(mode);
	const actionReceipts = new BoundedReceiptMap<Web06ActionReceipt>(web06ReceiptCapacity(mode));
	const actionIdentityByPromise = new WeakMap<Promise<unknown>, Web06ActionIdentity>();
	const actionIdentityBySequence = new Map<number, Web06ActionIdentity>();
	const eventReceipts = new BoundedReceiptMap<Web06EventReceipt>(
		web06ReceiptCapacity(mode),
		receipt => receipt.identity.eventSequenceId,
	);
	const invalidations: Web06MeasurementInvalidation[] = [];
	const pendingClockPings = new Map<string, {
		resolve: (exchange: Web06ClockExchange) => void;
		reject: (reason: unknown) => void;
	}>();
	const pendingFanoutsByOwner = new Map<string, Web06PendingFanout[]>();
	const auditedIncompleteFanouts = new Set<string>();
	const mainObserverCallbacks: Web06MainObserverCallbackInterval[] = [];
	const mainListenerEffectsByAction = new Map<number, Web06MainListenerEffect[]>();
	const pendingLifecycles = new Map<number, Web06PendingLifecycle>();
	const pendingPresentations = new Set<number>();
	const presentationTerminalProofs = new Map<number, Web06PresentationTerminalProof>();
	const auditedMissingTerminals = new Set<number>();
	const pendingFanoutActionCapacity = 256;
	const listenerEffectsPerActionCapacity = 64;
	const rawInputSequenceCapacity = 8_192;
	const reloadContinuityStorageKey = "__yune_web06_reload_continuity_v1__";
	let scopedActionContext: Web06ActionContext | undefined;
	let scopedControlEvent: Web06ControlEventDraft | undefined;
	let eventSequenceId = 0;
	let actionSequenceId = 0;
	let fanoutSequenceId = 0;
	let compositionEpochId = 1;
	let supersessionSubRunId = 1;
	let rawInputSequence: string[] = [];
	let clockPingSequenceId = 0;
	let lastEventTimestamp = Number.NEGATIVE_INFINITY;
	let lastSentSequenceId = 0;
	let lastCompletedSequenceId = 0;
	let receiptWindowStartEventSequenceId = 1;
	let receiptWindowStartActionSequenceId = 1;
	let mainObserverCallbackOverflowCount = 0;
	let mainObserverCallbackSequenceId = 0;
	let workerObserverFailureCount = 0;
	let workerObserverFailuresAudited = true;
	let preparedContinuityVoided = false;
	const pageInstanceId = createPageInstanceId();
	let reloadContinuity: Web06ReloadContinuityPreReceipt | Web06ReloadContinuityPostReceipt | undefined =
		consumeReloadContinuity();

	function now(): number {
		return performance.now();
	}

	function cloneForExport<T>(value: T): T {
		return typeof structuredClone === "function"
			? structuredClone(value)
			: JSON.parse(JSON.stringify(value)) as T;
	}

	function invalidate(
		code: string,
		detail: string,
		identity?: Partial<Pick<Web06ActionIdentity, "actionId" | "eventId">>,
	): void {
		try {
			if (invalidations.length >= 256) {
				const last = invalidations[255];
				if (last?.code !== "INVALIDATION_RING_OVERFLOW") {
					invalidations[255] = {
						code: "INVALIDATION_RING_OVERFLOW",
						detail: "WEB-06 invalidation ring exceeded its fixed 256-record capacity",
						recordedAt: now(),
					};
				}
				return;
			}
			invalidations.push({
				code,
				detail,
				recordedAt: now(),
				actionId: identity?.actionId,
				eventId: identity?.eventId,
			});
		}
		catch {
			// Measurement failure must never replace product behavior.
		}
	}

	function invalidateAndError(
		code: string,
		detail: string,
		identity?: Partial<Pick<Web06ActionIdentity, "actionId" | "eventId">>,
	): Error {
		invalidate(code, detail, identity);
		return new Error(`${code}: ${detail}`);
	}

	function observe<T>(
		operation: string,
		identity: Partial<Pick<Web06ActionIdentity, "actionId" | "eventId">> | undefined,
		callback: () => T,
	): T | undefined {
		const startedAt = now();
		try {
			return callback();
		}
		catch (error) {
			invalidate(
				"OBSERVER_CALLBACK_FAILURE",
				`${operation}: ${error instanceof Error ? `${error.name}: ${error.message}` : String(error)}`,
				identity,
			);
		}
		finally {
			const finishedAt = now();
			const duration = finishedAt - startedAt;
			recordMainObserverCallbackInterval(startedAt, finishedAt, operation, identity);
			if (duration >= 5) {
				invalidate("OBSERVER_CALLBACK_OVER_5MS", `${operation} took ${duration} ms`, identity);
			}
		}
		return undefined;
	}

	function timeObserverCallback<T>(
		operation: string,
		identity: Partial<Pick<Web06ActionIdentity, "actionId" | "eventId">> | undefined,
		callback: () => T,
	): T {
		const startedAt = now();
		try {
			return callback();
		}
		finally {
			const finishedAt = now();
			const duration = finishedAt - startedAt;
			recordMainObserverCallbackInterval(startedAt, finishedAt, operation, identity);
			if (duration >= 5) {
				invalidate("OBSERVER_CALLBACK_OVER_5MS", `${operation} took ${duration} ms`, identity);
			}
		}
	}

	function recordMainObserverCallbackInterval(
		startedAt: number,
		finishedAt: number,
		operation: string,
		identity?: Partial<Pick<Web06ActionIdentity, "actionId" | "eventId">>,
	): void {
		if (mode === "off") return;
		const durationMs = finishedAt - startedAt;
		const valid = Number.isFinite(startedAt)
			&& Number.isFinite(finishedAt)
			&& Number.isFinite(durationMs)
			&& durationMs >= 0;
		if (!valid) {
			invalidate(
				"MAIN_OBSERVER_CALLBACK_CLOCK_INVALID",
				`${operation} produced invalid callback interval ${String(startedAt)}..${String(finishedAt)}`,
				identity,
			);
		}
		const sequenceId = ++mainObserverCallbackSequenceId;
		const capacity = web06ReceiptCapacity(mode);
		if (mainObserverCallbacks.length < capacity) {
			mainObserverCallbacks.push({
				callbackId: `web06-main-observer-${String(sequenceId).padStart(8, "0")}`,
				sequenceId,
				operation,
				startedAt,
				finishedAt,
				durationMs: valid ? durationMs : Number.NaN,
				...(identity?.actionId === undefined ? {} : { actionId: identity.actionId }),
				...(identity?.eventId === undefined ? {} : { eventId: identity.eventId }),
			});
			return;
		}
		mainObserverCallbackOverflowCount += 1;
		if (mainObserverCallbackOverflowCount === 1) {
			invalidate(
				"MAIN_OBSERVER_CALLBACK_RING_OVERFLOW",
				`${operation} exceeded the fixed ${capacity}-callback main observer ledger`,
				identity,
			);
		}
	}

	function mainObserverCallbackDurations(): number[] {
		return mainObserverCallbacks.map(callback => callback.durationMs);
	}

	function createPageInstanceId(): string {
		try {
			if (typeof globalThis.crypto?.randomUUID === "function") {
				return `web06-page-${globalThis.crypto.randomUUID()}`;
			}
			const bytes = new Uint8Array(16);
			globalThis.crypto?.getRandomValues(bytes);
			if (bytes.some(value => value !== 0)) return `web06-page-${hexBytes(bytes)}`;
		}
		catch {
			// Fail closed below without retaining content.
		}
		return `web06-page-unavailable-${Math.round(performance.timeOrigin)}-${Math.round(now())}`;
	}

	function createSecureNonce(): string {
		if (typeof globalThis.crypto?.randomUUID === "function") return globalThis.crypto.randomUUID();
		const bytes = new Uint8Array(24);
		globalThis.crypto?.getRandomValues(bytes);
		if (bytes.every(value => value === 0)) {
			throw invalidateAndError("RELOAD_CONTINUITY_NONCE_UNAVAILABLE", "A secure reload-continuity nonce could not be created");
		}
		return hexBytes(bytes);
	}

	function hexBytes(bytes: Uint8Array): string {
		return [...bytes].map(value => value.toString(16).padStart(2, "0")).join("");
	}

	function sessionStorageOrUndefined(): Storage | undefined {
		try {
			return window.sessionStorage;
		}
		catch {
			return undefined;
		}
	}

	function consumeReloadContinuity(): Web06ReloadContinuityPostReceipt | undefined {
		const storage = sessionStorageOrUndefined();
		if (storage === undefined) return undefined;
		let raw: string | null;
		try {
			raw = storage.getItem(reloadContinuityStorageKey);
		}
		catch {
			return undefined;
		}
		if (raw === null) return undefined;
		try {
			storage.removeItem(reloadContinuityStorageKey);
		}
		catch {
			return undefined;
		}
		try {
			const token = JSON.parse(raw) as Partial<Web06ReloadContinuityToken>;
			if (
				token === null
				|| typeof token !== "object"
				|| typeof token.measurementId !== "string"
				|| typeof token.continuityNonce !== "string"
				|| Object.keys(token).sort().join(",") !== "continuityNonce,measurementId"
			) return undefined;
			return {
				phase: "post-reload-arrived",
				protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
				measurementId: token.measurementId,
				continuityNonce: token.continuityNonce,
				pageInstanceId,
				timeOrigin: performance.timeOrigin,
				consumedAt: now(),
				storageRemoved: true,
				oneShot: true,
			};
		}
		catch {
			return undefined;
		}
	}

	function invalidatePreparedContinuity(operation: string): void {
		if (reloadContinuity?.phase !== "pre-reload" || preparedContinuityVoided) return;
		preparedContinuityVoided = true;
		try {
			sessionStorageOrUndefined()?.removeItem(reloadContinuityStorageKey);
		}
		catch {
			// The invalidation below remains authoritative.
		}
		invalidate("RELOAD_CONTINUITY_VOIDED", `Reload continuity was voided by ${operation}`);
	}

	function recordDomEvent(
		event: Web06DomEventSnapshot,
		mapping: Web06EventMapResult,
		eventDeliveredAt: number,
	): Web06DomEventIdentity {
		invalidatePreparedContinuity(`DOM event ${event.type}/${event.code}`);
		const sequenceId = ++eventSequenceId;
		const identity: Web06DomEventIdentity = {
			protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
			eventId: `web06-event-${String(sequenceId).padStart(8, "0")}`,
			eventSequenceId: sequenceId,
			eventDeliveredAt,
			classification: mapping.classification,
			reason: mapping.reason,
			mappedActionCount: mapping.actions.length,
			compositionEpochId,
			supersessionSubRunId,
			...event,
		};
		observe("record-dom-event", identity, () => {
			if (mode === "off") return;
			if (
				!Number.isFinite(event.timeStamp)
				|| event.timeStamp < 0
				|| event.timeStamp < lastEventTimestamp
				|| Math.abs(event.timeStamp - now()) > 60_000
			) {
				invalidate("EVENT_TIMESTAMP_DOMAIN_INVALID", `DOM event timestamp ${event.timeStamp} is not monotonic in the page performance.now() domain`, identity);
			}
			lastEventTimestamp = Math.max(lastEventTimestamp, event.timeStamp);
			if (mapping.actions.length > pendingFanoutActionCapacity) {
				invalidate(
					"EVENT_ACTION_MAP_OVERFLOW",
					`Event ${identity.eventId} mapped ${mapping.actions.length} actions; capacity is ${pendingFanoutActionCapacity}`,
					identity,
				);
			}
			const retainedActions = mapping.actions.slice(0, pendingFanoutActionCapacity);
			const overflowed = eventReceipts.set({
				identity: web06RetainedEventIdentity(mode, identity),
				mappedActions: retainedActions.map(action => web06RetainedMappedAction(mode, action)),
				linkedActionIds: new Array<string>(retainedActions.length),
			});
			if (overflowed) invalidate("EVENT_RECEIPT_RING_OVERFLOW", `WEB-06 event receipt ring exceeded ${web06ReceiptCapacity(mode)} records`, identity);
		});
		return identity;
	}

	function withControlEvent<T>(event: Web06ControlEventLike, callback: () => T): T {
		if (scopedControlEvent !== undefined) {
			invalidate("NESTED_CONTROL_EVENT", "A WEB-06 control event was nested before its fanout declaration");
		}
		const deliveredAt = now();
		const snapshot = observe("control-event-freeze", undefined, () => snapshotWeb06ControlEvent(event)) ?? {
			type: event.type as Web06DomEventSnapshot["type"],
			code: "",
			key: "",
			timeStamp: event.timeStamp,
			repeat: false,
			ctrlKey: event.ctrlKey ?? false,
			metaKey: event.metaKey ?? false,
			altKey: event.altKey ?? false,
			shiftKey: event.shiftKey ?? false,
		};
		const draft: Web06ControlEventDraft = { snapshot, deliveredAt, declared: false };
		scopedControlEvent = draft;
		try {
			return callback();
		}
		finally {
			if (!draft.declared) {
				recordDomEvent(snapshot, {
					classification: "frontend-consumed",
					reason: "control-event-no-action",
					preventDefault: false,
					actions: [],
				}, draft.deliveredAt);
			}
			scopedControlEvent = undefined;
		}
	}

	function declareControlFanout(reason: string, actions: Web06FanoutAction[]): Web06DomEventIdentity {
		return timeObserverCallback("declare-control-fanout", undefined, () => {
			const draft = scopedControlEvent;
			if (draft === undefined) throw invalidateAndError("CONTROL_EVENT_CONTEXT_MISSING", `${reason} declared fanout outside withWeb06ControlEvent`);
			if (draft.declared) throw invalidateAndError("CONTROL_EVENT_FANOUT_DUPLICATE", `${reason} declared more than one fanout for one control event`);
			draft.declared = true;
			const mapping: Web06EventMapResult = {
				classification: actions.length === 0 ? "frontend-consumed" : "mapped-action(s)",
				reason,
				preventDefault: false,
				actions: actions.map(item => item.action),
			};
			const event = recordDomEvent(draft.snapshot, mapping, draft.deliveredAt);
			registerEventFanout(event, actions);
			return event;
		});
	}

	function registerEventFanout(event: Web06DomEventIdentity, actions: Web06FanoutAction[]): void {
		timeObserverCallback("register-event-fanout", event, () => {
		if (actions.length > pendingFanoutActionCapacity) {
			invalidate("FANOUT_RING_OVERFLOW", `WEB-06 control fanout declared ${actions.length} actions; capacity is ${pendingFanoutActionCapacity}`, event);
			return;
		}
		if (mode !== "off") {
			const declared = eventReceipts.get(event.eventSequenceId)?.mappedActions ?? [];
			if (
				declared.length !== actions.length
				|| declared.some((action, index) => !sameAction(action, actions[index]?.action))
			) {
				invalidate("FANOUT_DECLARATION_MISMATCH", `Fanout actions do not match event ${event.eventId}`, event);
				return;
			}
		}
		if (actions.length === 0) return;
		const pendingBefore = pendingFanoutActionCount();
		if (pendingBefore + actions.length > pendingFanoutActionCapacity) {
			invalidate("FANOUT_RING_OVERFLOW", `WEB-06 pending control fanouts would exceed ${pendingFanoutActionCapacity} actions (${pendingBefore} + ${actions.length})`, event);
			return;
		}
		const fanout: Web06FanoutState = {
			fanoutId: `web06-fanout-${String(++fanoutSequenceId).padStart(8, "0")}`,
			event,
			nextActionIndex: 0,
			totalActions: actions.length,
		};
			actions.forEach((planned, eventActionIndex) => {
			const pending = pendingFanoutsByOwner.get(planned.owner) ?? [];
			// Keep exact arguments only in the bounded, short-lived working fanout
			// lane. Exported minimal receipts remain content-free.
			pending.push({
				fanout,
				eventActionIndex,
				action: cloneForExport(planned.action),
				deferredResolved: planned.action.deferred !== true,
			});
			pendingFanoutsByOwner.set(planned.owner, pending);
		});
		});
	}

	function resolveDeferredFanoutAction(event: Web06DomEventIdentity, eventActionIndex: number, args: unknown[]): boolean {
		return timeObserverCallback("resolve-deferred-fanout-action", event, () => {
		for (const pending of pendingFanoutsByOwner.values()) {
			const expected = pending.find(candidate => candidate.fanout.event.eventId === event.eventId
				&& candidate.eventActionIndex === eventActionIndex);
			if (expected === undefined) continue;
			if (expected.action.deferred !== true) {
				invalidate(
					"DEFERRED_FANOUT_ACTION_NOT_DEFERRED",
					`Fanout action ${event.eventId}/${eventActionIndex} was not declared deferred`,
					event,
				);
				return false;
			}
			if (expected.deferredResolved) {
				invalidate(
					"DEFERRED_FANOUT_ALREADY_RESOLVED",
					`Fanout action ${event.eventId}/${eventActionIndex} already accepted deferred arguments`,
					event,
				);
				return false;
			}
			expected.action.args = cloneForExport(args);
			expected.deferredResolved = true;
			const receipt = eventReceipts.get(event.eventSequenceId);
			if (receipt?.mappedActions[eventActionIndex] !== undefined) {
				receipt.mappedActions[eventActionIndex]!.args = cloneForExport(web06RetainedActionArgs(mode, expected.action.name, args));
			}
			return true;
		}
		invalidate("DEFERRED_FANOUT_RESOLUTION_MISSING", `No pending fanout action ${event.eventId}/${eventActionIndex} accepted deferred arguments`, event);
		return false;
		});
	}

	function cancelEventFanout(event: Web06DomEventIdentity, reason: string): void {
		timeObserverCallback("cancel-event-fanout", event, () => {
		const matching = [...pendingFanoutsByOwner.values()]
			.flatMap(pending => pending)
			.filter(expected => expected.fanout.event.eventId === event.eventId);
		if (matching.length === 0) return;
		const partial = matching.find(expected => expected.fanout.nextActionIndex > 0);
		if (partial !== undefined) {
			invalidate("CANCELLED_PARTIAL_FANOUT", `Cannot cancel ${partial.fanout.fanoutId} after an action was consumed`, event);
			return;
		}
		for (const [owner, pending] of pendingFanoutsByOwner) {
			for (const expected of pending) {
				if (expected.fanout.event.eventId === event.eventId) {
					auditedIncompleteFanouts.delete(`${expected.fanout.fanoutId}:${expected.eventActionIndex}`);
				}
			}
			const retained = pending.filter(expected => expected.fanout.event.eventId !== event.eventId);
			if (retained.length === 0) pendingFanoutsByOwner.delete(owner);
			else pendingFanoutsByOwner.set(owner, retained);
		}
		event.classification = "frontend-consumed";
		event.reason = reason;
		event.mappedActionCount = 0;
		const receipt = eventReceipts.get(event.eventSequenceId);
		if (receipt !== undefined) {
			receipt.identity = web06RetainedEventIdentity(mode, event);
			receipt.mappedActions = [];
			receipt.linkedActionIds = [];
		}
		});
	}

	function pendingFanoutActionCount(): number {
		let count = 0;
		for (const pending of pendingFanoutsByOwner.values()) count += pending.length;
		return count;
	}

	function auditPendingFanouts(): void {
		for (const pending of pendingFanoutsByOwner.values()) {
			for (const item of pending) {
				const key = `${item.fanout.fanoutId}:${item.eventActionIndex}`;
				if (auditedIncompleteFanouts.has(key)) continue;
				if (auditedIncompleteFanouts.size >= pendingFanoutActionCapacity) {
					invalidate("AUDITED_FANOUT_RING_OVERFLOW", `WEB-06 audited fanout IDs exceeded ${pendingFanoutActionCapacity} records`, item.fanout.event);
					return;
				}
				auditedIncompleteFanouts.add(key);
				invalidate("INCOMPLETE_EVENT_FANOUT", `${item.fanout.fanoutId} still owns action ${item.eventActionIndex}/${item.fanout.totalActions}`, item.fanout.event);
			}
		}
	}

	function mappedActionContext(
		event: Web06DomEventIdentity,
		action: Web06MappedAction,
		eventActionIndex: number,
		rawInput?: string,
	): Web06ActionContext {
		return timeObserverCallback("mapped-action-context", event, () => {
		if (action.supersedable && rawInput !== undefined) {
			if (rawInputSequence.length >= rawInputSequenceCapacity) {
				invalidate("RAW_INPUT_SEQUENCE_OVERFLOW", `WEB-06 raw input sequence exceeded ${rawInputSequenceCapacity} records`, event);
				supersessionSubRunId += 1;
				rawInputSequence = [];
			}
			rawInputSequence = [...rawInputSequence, rawInput];
		}
		const context: Web06ActionContext = {
			event,
			eventActionIndex,
			compositionEpochId,
			supersessionSubRunId,
			actionClass: action.actionClass,
			supersedable: action.supersedable,
			boundary: action.boundary,
			rawInputSequence: [...rawInputSequence],
			originKind: "dom-event",
			originReason: event.reason,
		};
		advanceBoundary(action.boundary);
		return context;
		});
	}

	function advanceBoundary(boundary: Web06ActionIdentity["boundary"]): void {
		timeObserverCallback("advance-action-boundary", undefined, () => {
		if (boundary === "none") return;
		supersessionSubRunId += 1;
		rawInputSequence = [];
		if (["commit", "cancel", "focus-loss", "schema", "option", "deploy", "persistence", "error"].includes(boundary)) {
			compositionEpochId += 1;
		}
		});
	}

	function withActionContext<T>(context: Web06ActionContext, action: () => T): T {
		if (scopedActionContext !== undefined) invalidate("NESTED_ACTION_CONTEXT", "A WEB-06 action context was nested before enqueue");
		scopedActionContext = context;
		try {
			return action();
		}
		finally {
			scopedActionContext = undefined;
		}
	}

	function sameAction(left: Web06MappedAction, right: Web06MappedAction | undefined): boolean {
		return right !== undefined
			&& left.name === right.name
			&& web06ValuesEqual(left.args, web06RetainedActionArgs(mode, right.name, right.args))
			&& left.actionClass === right.actionClass
			&& left.supersedable === right.supersedable
			&& left.boundary === right.boundary
			&& left.deferred === right.deferred;
	}

	function takeFanoutContext(owner: string, name: keyof Actions, args: unknown[]): Web06ActionContext | undefined {
		return timeObserverCallback("take-fanout-context", undefined, () => {
		const pending = pendingFanoutsByOwner.get(owner);
		const expected = pending?.[0];
		if (expected === undefined) return undefined;
		if (expected.action.deferred === true && !expected.deferredResolved) {
			invalidate(
				"UNRESOLVED_DEFERRED_FANOUT_ACTION",
				`${owner} attempted fanout action ${expected.eventActionIndex} before its deferred arguments were resolved`,
				expected.fanout.event,
			);
			return undefined;
		}
		if (
			expected.action.name !== name
			|| !web06ValuesEqual(expected.action.args, args)
		) {
			invalidate(
				"FANOUT_OWNER_ACTION_MISMATCH",
				`${owner} expected ${expected.action.name} at fanout index ${expected.eventActionIndex}, got ${name}`,
				expected.fanout.event,
			);
			return undefined;
		}
		pending!.splice(0, 1);
		auditedIncompleteFanouts.delete(`${expected.fanout.fanoutId}:${expected.eventActionIndex}`);
		if (pending!.length === 0) pendingFanoutsByOwner.delete(owner);
		if (expected.eventActionIndex !== expected.fanout.nextActionIndex) {
			invalidate(
				"FANOUT_GLOBAL_ORDER_MISMATCH",
				`${expected.fanout.fanoutId} expected global index ${expected.fanout.nextActionIndex}, got ${expected.eventActionIndex}`,
				expected.fanout.event,
			);
		}
		expected.fanout.nextActionIndex = Math.max(expected.fanout.nextActionIndex, expected.eventActionIndex + 1);
		const context: Web06ActionContext = {
			event: expected.fanout.event,
			eventActionIndex: expected.eventActionIndex,
			compositionEpochId,
			supersessionSubRunId,
			actionClass: expected.action.actionClass,
			supersedable: expected.action.supersedable,
			boundary: expected.action.boundary,
			rawInputSequence: [],
			originKind: "dom-event",
			originReason: expected.fanout.event.reason,
			originOwner: owner,
		};
		advanceBoundary(expected.action.boundary);
		return context;
		});
	}

	function withOwnedAction<T>(
		owner: string,
		name: keyof Actions,
		args: unknown[],
		backgroundReason: string,
		causedBy: Web06ActionIdentity | undefined,
		action: () => T,
	): T {
		const pending = takeFanoutContext(owner, name, args);
		if (pending !== undefined) return withActionContext(pending, action);
		const contract = web06ActionContract(name);
		const context: Web06ActionContext = {
			compositionEpochId,
			supersessionSubRunId,
			actionClass: contract.actionClass,
			supersedable: false,
			boundary: contract.boundary,
			rawInputSequence: [...rawInputSequence],
			originKind: "background",
			originReason: backgroundReason,
			originOwner: owner,
			causedByActionId: causedBy?.actionId,
			causedBySequenceId: causedBy?.sequenceId,
			causedByEventId: causedBy?.eventId,
			causedByEventSequenceId: causedBy?.eventSequenceId,
		};
		try {
			return withActionContext(context, action);
		}
		finally {
			advanceBoundary(contract.boundary);
		}
	}

	function takeActionContext(name: keyof Actions): Web06ActionContext {
		if (scopedActionContext !== undefined) return scopedActionContext;
		const contract = web06ActionContract(name);
		const context: Web06ActionContext = {
			compositionEpochId,
			supersessionSubRunId,
			actionClass: contract.actionClass,
			supersedable: false,
			boundary: contract.boundary,
			rawInputSequence: [...rawInputSequence],
			originKind: "background",
			originReason: `unclassified:${name}`,
		};
		advanceBoundary(contract.boundary);
		return context;
	}

	function createActionIdentity(name: keyof Actions, args: unknown[]): Web06ActionIdentity {
		return timeObserverCallback("create-action-identity", undefined, () => {
		invalidatePreparedContinuity(`action ${name} after reload preparation`);
		const context = takeActionContext(name);
		const sequenceId = ++actionSequenceId;
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
			originOwner: context.originOwner,
			causedByActionId: context.causedByActionId,
			causedBySequenceId: context.causedBySequenceId,
			causedByEventId: context.causedByEventId,
			causedByEventSequenceId: context.causedByEventSequenceId,
			actionEnqueuedAt: now(),
			mainQueueDepthAtEnqueue: hooks.queueState().queueDepth,
		};
		if (identity.originReason.startsWith("unclassified:")) {
			invalidate("UNCLASSIFIED_ACTION_ORIGIN", `Action ${identity.actionId} has no explicit DOM-event or background origin`, identity);
		}
		observe("record-action-enqueue", identity, () => {
			actionIdentityBySequence.set(sequenceId, identity);
			while (actionIdentityBySequence.size > 4) {
				const first = actionIdentityBySequence.keys().next().value as number | undefined;
				if (first === undefined) break;
				actionIdentityBySequence.delete(first);
			}
			linkActionToEvent(identity, name);
			if (mode === "off") return;
			const overflowed = actionReceipts.set({
				identity: web06RetainedActionIdentity(mode, identity),
				name,
				args: cloneForExport(web06RetainedActionArgs(mode, name, args)),
				expectedFailureControl: web06ExpectedActionFailure(name, args),
			});
			if (overflowed) invalidate("ACTION_RECEIPT_RING_OVERFLOW", `WEB-06 action receipt ring exceeded ${web06ReceiptCapacity(mode)} records`, identity);
		});
		return identity;
		});
	}

	function bindActionPromise(promise: Promise<unknown>, identity: Web06ActionIdentity): void {
		timeObserverCallback("bind-action-promise", identity, () => actionIdentityByPromise.set(promise, identity));
	}

	function actionIdentityFor(promise: Promise<unknown>): Web06ActionIdentity | undefined {
		return timeObserverCallback("lookup-action-promise", undefined, () => actionIdentityByPromise.get(promise));
	}

	function linkActionToEvent(identity: Web06ActionIdentity, name: keyof Actions): void {
		if (identity.eventId === undefined) return;
		const event = identity.eventSequenceId === undefined ? undefined : eventReceipts.get(identity.eventSequenceId);
		if (event === undefined) {
			if (mode !== "off") invalidate("ORPHANED_ACTION_EVENT", `Action ${identity.actionId} references missing event ${identity.eventId}`, identity);
			return;
		}
		const index = identity.eventActionIndex;
		if (index === undefined || index < 0 || index >= event.identity.mappedActionCount) {
			invalidate("ACTION_INDEX_OUT_OF_RANGE", `Action ${identity.actionId} has invalid event index ${String(index)}`, identity);
			return;
		}
		if (event.linkedActionIds[index] !== undefined) {
			invalidate("DUPLICATE_EVENT_ACTION", `Event ${event.identity.eventId} action ${index} was linked twice`, identity);
			return;
		}
		const expected = event.mappedActions[index];
		if (expected?.name !== name) invalidate("EVENT_ACTION_KIND_MISMATCH", `Event ${event.identity.eventId} expected ${expected?.name ?? "missing"}, got ${name}`, identity);
		event.linkedActionIds[index] = identity.actionId;
	}

	function markActionSent(identity: Web06ActionIdentity): Web06ActionIdentity {
		return timeObserverCallback("mark-action-sent", identity, () => {
		identity.workerDispatchDepth = 1;
		if (identity.sequenceId <= lastSentSequenceId) {
			invalidate("DUPLICATE_ACTION_SEND", `Duplicate worker send ${identity.sequenceId}`, identity);
		}
		else if (identity.sequenceId !== lastSentSequenceId + 1) {
			invalidate("REORDERED_ACTION_SEND", `Expected send sequence ${lastSentSequenceId + 1}, got ${identity.sequenceId}`, identity);
		}
		lastSentSequenceId = Math.max(lastSentSequenceId, identity.sequenceId);
		identity.workerSentAt = now();
		const receipt = actionReceipts.get(identity.sequenceId);
		if (receipt !== undefined) {
			receipt.identity = web06RetainedActionIdentity(mode, identity);
			actionReceipts.set(receipt);
		}
		return web06RetainedActionIdentity(mode, identity);
		});
	}

	function recordResponseMapping(identity: Web06ActionIdentity, startedAt: number, finishedAt: number): void {
		observe("record-response-mapping", identity, () => {
			const receipt = actionReceipts.get(identity.sequenceId);
			if (receipt === undefined) return;
			if (!web06TimestampsAreOrdered(startedAt, finishedAt)) invalidate("RESPONSE_MAPPING_TIMESTAMP_ORDER", `Action ${identity.actionId} recorded invalid response-mapping timestamps`, identity);
			receipt.responseMappingStartedAt = startedAt;
			receipt.responseMappingFinishedAt = finishedAt;
			actionReceipts.set(receipt);
		});
	}

	function recordListener<K extends keyof ListenerArgsMap>(
		name: K,
		args: ListenerArgsMap[K],
		metadata: { protocolVersion: typeof WEB06_PRIVATE_PROTOCOL_VERSION; actionId: string; sequenceId: number } | undefined,
		currentIdentity: Web06ActionIdentity | undefined,
		appliedAt: number,
	): void {
		if (metadata === undefined) return;
		if (
			metadata.protocolVersion !== WEB06_PRIVATE_PROTOCOL_VERSION
			|| currentIdentity === undefined
			|| metadata.actionId !== currentIdentity.actionId
			|| metadata.sequenceId !== currentIdentity.sequenceId
		) {
			invalidate(
				"LISTENER_ACTION_IDENTITY_MISMATCH",
				`Listener ${name} carried ${metadata.actionId}/${metadata.sequenceId} while ${currentIdentity?.actionId ?? "no action"} was running`,
				currentIdentity,
			);
			return;
		}
		observe("record-main-listener-effect", currentIdentity, () => {
			if (mode === "off") return;
			if (!mainListenerEffectsByAction.has(currentIdentity.sequenceId)
				&& mainListenerEffectsByAction.size >= receiptCapacity) {
				const oldest = mainListenerEffectsByAction.keys().next().value as number | undefined;
				if (oldest !== undefined) mainListenerEffectsByAction.delete(oldest);
				invalidate("MAIN_LISTENER_ACTION_RING_OVERFLOW", `WEB-06 main listener action ledger exceeded ${receiptCapacity} records`, currentIdentity);
			}
			const effects = mainListenerEffectsByAction.get(currentIdentity.sequenceId) ?? [];
			if (effects.length >= listenerEffectsPerActionCapacity) {
				invalidate("MAIN_LISTENER_EFFECT_RING_OVERFLOW", `Action ${currentIdentity.actionId} exceeded ${listenerEffectsPerActionCapacity} listener effects`, currentIdentity);
				return;
			}
			const proof = listenerEffectProof(name, args);
			effects.push({
				name,
				argsDigest: web06StableDigest(args),
				args: cloneForExport(args),
				...proof,
				appliedAt,
			});
			mainListenerEffectsByAction.set(currentIdentity.sequenceId, effects);
		});
	}

	function listenerEffectProof<K extends keyof ListenerArgsMap>(
		name: K,
		args: ListenerArgsMap[K],
	): Pick<Web06MainListenerEffect, "stateKey" | "expectedState" | "readState"> {
		const includeDeployView = name === "deployStatusChanged" && queryElement("[data-yune-deploy-status-view]") !== null;
		const option = name === "optionChanged" ? String(args[0]) : undefined;
		return {
			stateKey: option === undefined ? String(name) : `${name}:${option}`,
			expectedState: expectedListenerState(name, args, includeDeployView),
			readState: () => observedListenerState(name, option, includeDeployView),
		};
	}

	function expectedListenerState<K extends keyof ListenerArgsMap>(
		name: K,
		args: ListenerArgsMap[K],
		includeDeployView: boolean,
	): Record<string, unknown> {
		switch (name) {
			case "deployStatusChanged":
				return { root: String(args[0]), ...(includeDeployView ? { view: String(args[0]) } : {}) };
			case "schemaChanged":
				return { id: String(args[0]), name: String(args[1]) };
			case "grammarDiagnosticChanged":
				return { diagnostic: JSON.stringify(args[0]) };
			case "optionChanged": {
				const option = String(args[0]);
				return { last: `${option}:${String(args[1])}`, option, value: String(args[1]) };
			}
			case "initialized":
				return { initialized: String(args[0]) };
		}
	}

	function observedListenerState(
		name: keyof ListenerArgsMap,
		option: string | undefined,
		includeDeployView: boolean,
	): Record<string, unknown> {
		const dataset = document.documentElement.dataset;
		switch (name) {
			case "deployStatusChanged":
				return {
					root: dataset["yuneDeployStatus"],
					...(includeDeployView ? { view: queryElement("[data-yune-deploy-status-view]")?.dataset["yuneDeployStatusView"] } : {}),
				};
			case "schemaChanged":
				return { id: dataset["yuneActiveSchema"], name: dataset["yuneActiveSchemaName"] };
			case "grammarDiagnosticChanged":
				return { diagnostic: dataset["yuneGrammarDiagnostic"] };
			case "optionChanged":
				return {
					last: dataset["yuneLastOptionChanged"],
					option,
					value: option === undefined ? undefined : dataset[optionDatasetKey(option)],
				};
			case "initialized":
				return { initialized: dataset["yuneInitialized"] };
		}
	}

	function optionDatasetKey(option: string): string {
		return `yuneOption${option.split(/[_-]+/).filter(Boolean).map(part => `${part[0]?.toUpperCase() ?? ""}${part.slice(1)}`).join("")}`;
	}

	function queryElement(selector: string): HTMLElement | null {
		return typeof document.querySelector === "function" ? document.querySelector<HTMLElement>(selector) : null;
	}

	function handleClockEcho(data: Web06ClockEchoEnvelope, mainReceivedAt: number): void {
		const pending = pendingClockPings.get(data.pingId);
		if (pending === undefined) {
			invalidate("ORPHAN_CLOCK_ECHO", `Unknown worker clock echo ${data.pingId}`);
			return;
		}
		pendingClockPings.delete(data.pingId);
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
			invalidate("INVALID_CLOCK_EXCHANGE", error.message);
			pending.reject(error);
			return;
		}
		pending.resolve(exchange);
	}

	function handleActionResult(
		data: Extract<Web06WorkerResponseEnvelope, { kind: "action-result" }>,
		currentIdentity: Web06ActionIdentity | undefined,
		mainResponseReceivedAt: number,
	): void {
		observe("handle-action-result", currentIdentity, () => {
		if (currentIdentity === undefined) {
			invalidate("ORPHAN_ACTION_RESULT", `Worker returned ${data.identity.actionId} with no running action`, data.identity);
			return;
		}
		if (
			data.protocolVersion !== WEB06_PRIVATE_PROTOCOL_VERSION
			|| data.mode !== mode
			|| data.modeProvenance !== modeProvenance
		) {
			invalidate("WIRE_PROVENANCE_MISMATCH", `Expected ${WEB06_PRIVATE_PROTOCOL_VERSION}/${mode}/${modeProvenance}, got ${data.protocolVersion}/${data.mode}/${data.modeProvenance}`, currentIdentity);
		}
		if (!web06ActionIdentitiesEqual(data.identity, web06RetainedActionIdentity(mode, currentIdentity))) {
			invalidate("ACTION_RESULT_IDENTITY_MISMATCH", `Worker did not round-trip the complete private identity for ${currentIdentity.actionId}`, currentIdentity);
		}
		if (data.identity.sequenceId <= lastCompletedSequenceId) {
			invalidate("DUPLICATE_ACTION_RESULT", `Duplicate action result ${data.identity.sequenceId}`, data.identity);
		}
		else if (data.identity.sequenceId !== lastCompletedSequenceId + 1) {
			invalidate("REORDERED_ACTION_RESULT", `Expected completed sequence ${lastCompletedSequenceId + 1}, got ${data.identity.sequenceId}`, data.identity);
		}
		lastCompletedSequenceId = Math.max(lastCompletedSequenceId, data.identity.sequenceId);
		const receipt = actionReceipts.get(currentIdentity.sequenceId);
		if (receipt !== undefined) {
			workerObserverFailuresAudited = false;
			receipt.returnedIdentity = cloneForExport(data.identity);
			receipt.mainResponseReceivedAt = mainResponseReceivedAt;
			const retainedWorker = cloneForExport(data.receipt);
			retainedWorker.lifecycleEffects = retainedWorker.lifecycleEffects.map(effect => effect.kind === "listener"
				? { kind: effect.kind, name: effect.name, argsDigest: effect.argsDigest, recordedAt: effect.recordedAt }
				: effect);
			receipt.worker = retainedWorker;
			receipt.resultType = data.resultType;
			actionReceipts.set(receipt);
			for (const failure of data.receipt.observerFailures) {
				workerObserverFailureCount += 1;
				invalidate("WORKER_OBSERVER_FAILURE", `${data.identity.actionId}: ${failure}`, data.identity);
			}
			workerObserverFailuresAudited = true;
			observe("prepare-terminal-outcome", currentIdentity, () => prepareTerminalOutcome(receipt, data, mainResponseReceivedAt));
		}
		if (data.resultType === "error" && currentIdentity.boundary === "none") advanceBoundary("error");
		});
	}

	function prepareTerminalOutcome(
		receipt: Web06ActionReceipt,
		data: Extract<Web06WorkerResponseEnvelope, { kind: "action-result" }>,
		mainResponseReceivedAt: number,
	): void {
		const identity = receipt.identity;
		const contract = web06TerminalContract(receipt.name);
		const resultSummary = data.receipt.resultSummary;
		const semanticEffects = data.receipt.lifecycleEffects.filter((effect): effect is Exclude<Web06WorkerLifecycleEffect, { kind: "listener" }> => effect.kind !== "listener" && effect.name === receipt.name);
		const semanticEffect = contract.workerEffect === undefined
			? semanticEffects.length === 1 ? semanticEffects[0] : undefined
			: semanticEffects.find(effect => effect.kind === contract.workerEffect);
		const workerListenerEffects = data.receipt.lifecycleEffects
			.filter(effect => effect.kind === "listener")
			.map(effect => ({ name: effect.name, argsDigest: effect.argsDigest, args: effect.args }));
		const mainListenerProofs = [...(mainListenerEffectsByAction.get(identity.sequenceId) ?? [])];
		const mainListenerEffects = mainListenerProofs
			.map(effect => ({ name: effect.name, argsDigest: effect.argsDigest, args: effect.args }));
		// Listener proofs are copied into the terminal working record below. Retire
		// the per-action ingress ledger for every strategy, including presentation
		// and failure outcomes that never enter pendingLifecycles.
		mainListenerEffectsByAction.delete(identity.sequenceId);
		const workerEffectDigest = web06StableDigest({
			resultType: data.resultType,
			resultSummary,
			semanticEffect: semanticEffect === undefined ? undefined : { kind: semanticEffect.kind, name: semanticEffect.name, resultDigest: semanticEffect.resultDigest },
			listenerEffects: workerListenerEffects.map(({ name, argsDigest }) => ({ name, argsDigest })),
		});
		let mainEffectDigest = web06StableDigest({
			listenerEffects: mainListenerEffects.map(({ name, argsDigest }) => ({ name, argsDigest })),
		});
		const expectedFailure = receipt.expectedFailureControl === true;

		if (data.resultType === "error") {
			if (!expectedFailure) invalidate("UNEXPECTED_ACTION_FAILURE", `Action ${identity.actionId} failed outside the frozen error-control contract`, identity);
			const effectDigest = web06StableDigest({ workerEffectDigest, mainEffectDigest });
			recordLifecycleOutcome({
				identity,
				outcome: "failure",
				stateUpdateScheduledAt: mainResponseReceivedAt,
				terminalObservedAt: mainResponseReceivedAt,
				effect: "error",
				effectDigest,
				workerEffectDigest,
				mainEffectDigest,
				listenerEffectCount: workerListenerEffects.length,
				persistenceCompleted: false,
			});
			writeTerminalMarker(identity, "failure", effectDigest);
			return;
		}
		if (expectedFailure && resultSummary?.success === false) {
			const effectDigest = web06StableDigest({ workerEffectDigest, mainEffectDigest, expectedFailure: true });
			recordLifecycleOutcome({
				identity,
				outcome: "failure",
				stateUpdateScheduledAt: mainResponseReceivedAt,
				terminalObservedAt: mainResponseReceivedAt,
				effect: "error",
				effectDigest,
				workerEffectDigest,
				mainEffectDigest,
				listenerEffectCount: workerListenerEffects.length,
				persistenceCompleted: false,
			});
			writeTerminalMarker(identity, "failure", effectDigest);
			return;
		}
		if (expectedFailure) invalidate("EXPECTED_ACTION_FAILURE_MISSING", `Action ${identity.actionId} unexpectedly succeeded in the frozen error-control lane`, identity);
		if (contract.strategy === "presentation") {
			const capacity = web06ReceiptCapacity(mode);
			if (!pendingPresentations.has(identity.sequenceId) && pendingPresentations.size >= capacity) {
				const oldest = pendingPresentations.values().next().value as number | undefined;
				if (oldest !== undefined) pendingPresentations.delete(oldest);
				invalidate("PENDING_PRESENTATION_RING_OVERFLOW", `WEB-06 pending presentation terminals exceeded ${capacity} records`, identity);
			}
			if (capacity > 0) pendingPresentations.add(identity.sequenceId);
			return;
		}
		if (
			resultSummary === undefined
			|| resultSummary.success === false
			|| semanticEffect === undefined
			|| semanticEffects.length !== 1
			|| (semanticEffect.kind === "engine-persistence" && resultSummary.persistenceCompleted !== true)
		) {
			invalidate("LIFECYCLE_EFFECT_UNPROVED", `${receipt.name} lacks a successful ${contract.workerEffect ?? "worker"} effect proof`, identity);
			const effect = contract.ownerEffect ?? (contract.workerEffect === "engine-persistence" ? "engine-persistence" : contract.workerEffect === "cache-invalidation" ? "cache-invalidation" : "listener");
			const effectDigest = web06StableDigest({ workerEffectDigest, mainEffectDigest });
			recordLifecycleOutcome({
				identity,
				outcome: "failure",
				stateUpdateScheduledAt: mainResponseReceivedAt,
				terminalObservedAt: mainResponseReceivedAt,
				effect,
				effectDigest,
				workerEffectDigest,
				mainEffectDigest,
				listenerEffectCount: workerListenerEffects.length,
				persistenceCompleted: resultSummary?.persistenceCompleted === true,
			});
			writeTerminalMarker(identity, "failure", effectDigest);
			return;
		}

		const pending: Web06PendingLifecycle = {
			identity,
			name: receipt.name,
			workerEffectDigest,
			listenerEffectCount: workerListenerEffects.length,
			persistenceCompleted: resultSummary.persistenceCompleted,
			expectedOwnerEffect: contract.ownerEffect,
			...(contract.strategy === "listener" ? { listenerEffects: mainListenerProofs } : {}),
			scheduled: false,
		};
		pendingLifecycles.set(identity.sequenceId, pending);
		if (pendingLifecycles.size > web06ReceiptCapacity(mode)) {
			invalidate("PENDING_LIFECYCLE_RING_OVERFLOW", `WEB-06 pending lifecycle terminals exceeded ${web06ReceiptCapacity(mode)} records`, identity);
			recordAndRemoveLifecycleFailure(pending, mainResponseReceivedAt, "error");
			return;
		}
		if (contract.strategy === "listener") {
			if (
				workerListenerEffects.length === 0
				|| workerListenerEffects.some(effect => effect.args === undefined)
				|| !web06ValuesEqual(workerListenerEffects, mainListenerEffects)
			) {
				invalidate("LISTENER_EFFECT_MISMATCH", `${receipt.name} worker/main listener effects did not match exactly`, identity);
				recordAndRemoveLifecycleFailure(pending, mainResponseReceivedAt, "listener");
				return;
			}
			scheduleLifecycleTerminal(pending, "listener", mainResponseReceivedAt);
			return;
		}
		if (contract.strategy === "worker-effect") {
			const effect = semanticEffect.kind === "engine-state" ? "engine-state" : "engine-persistence";
			mainEffectDigest = web06StableDigest({ effect, workerEffectDigest, postEffectObservation: "double-raf-pending" });
			pending.mainEffectDigest = mainEffectDigest;
			scheduleLifecycleTerminal(pending, effect, mainResponseReceivedAt, contract.doubleRaf);
		}
	}

	function recordOwnedResultEffect(
		promise: Promise<unknown>,
		effect: "ui-userdb-refresh" | "ui-diagnostic-refresh" | "cache-invalidation",
		proof: Web06OwnedEffectProof,
	): void {
		observe("owner-effect-proof", actionIdentityByPromise.get(promise), () => {
		const identity = actionIdentityByPromise.get(promise);
		if (identity === undefined) {
			invalidate("OWNER_EFFECT_IDENTITY_MISSING", `No WEB-06 identity exists for ${effect}`);
			return;
		}
		const pending = pendingLifecycles.get(identity.sequenceId);
		if (pending === undefined) {
			invalidate("OWNER_EFFECT_PENDING_MISSING", `Action ${identity.actionId} has no pending ${effect} terminal`, identity);
			return;
		}
		if (pending.expectedOwnerEffect !== effect || pending.scheduled) {
			invalidate("OWNER_EFFECT_CONTRACT_MISMATCH", `Action ${identity.actionId} expected ${pending.expectedOwnerEffect ?? "no owner effect"}, got ${effect}`, identity);
			recordAndRemoveLifecycleFailure(pending, now(), effect);
			return;
		}
		if (Object.keys(proof.expectedState).length === 0) {
			invalidate("OWNER_EFFECT_PROOF_INVALID", `Action ${identity.actionId} supplied an empty expected ${effect} state`, identity);
			recordAndRemoveLifecycleFailure(pending, now(), effect);
			return;
		}
		pending.ownerProof = proof;
		scheduleLifecycleTerminal(pending, effect, now());
		});
	}

	function scheduleLifecycleTerminal(
		pending: Web06PendingLifecycle,
		effect: Web06LifecycleOutcomeReceipt["effect"],
		stateUpdateScheduledAt: number,
		doubleRaf = true,
	): void {
		if (pending.scheduled) {
			invalidate("DUPLICATE_TERMINAL_SCHEDULE", `Action ${pending.identity.actionId} scheduled more than one lifecycle terminal`, pending.identity);
			return;
		}
		pending.scheduled = true;
		const finish = (firstRafAt?: number) => {
			const terminalObservedAt = now();
			const verified = verifyLifecycleMainEffect(pending, effect);
			pending.mainEffectDigest = verified.digest;
			if (!verified.valid) {
				invalidate("LIFECYCLE_MAIN_EFFECT_MISMATCH", `Action ${pending.identity.actionId} did not prove its ${effect} terminal effect`, pending.identity);
				recordAndRemoveLifecycleFailure(pending, terminalObservedAt, effect);
				return;
			}
			const effectDigest = web06StableDigest({ workerEffectDigest: pending.workerEffectDigest, mainEffectDigest: verified.digest, effect });
			recordLifecycleOutcome({
				identity: pending.identity,
				outcome: "barrier-completed",
				stateUpdateScheduledAt,
				firstRafAt,
				terminalObservedAt,
				effect,
				effectDigest,
				workerEffectDigest: pending.workerEffectDigest,
				mainEffectDigest: verified.digest,
				listenerEffectCount: pending.listenerEffectCount,
				persistenceCompleted: pending.persistenceCompleted,
			});
			pendingLifecycles.delete(pending.identity.sequenceId);
			writeTerminalMarker(pending.identity, "barrier-completed", effectDigest);
		};
		if (!doubleRaf) {
			finish();
			return;
		}
		try {
			requestAnimationFrame(() => observe("lifecycle-first-raf", pending.identity, () => {
				const firstRafAt = now();
				try {
					requestAnimationFrame(() => observe("lifecycle-second-raf", pending.identity, () => finish(firstRafAt)));
				}
				catch (error) {
					invalidate("LIFECYCLE_RAF_SCHEDULE_FAILURE", error instanceof Error ? error.message : String(error), pending.identity);
					recordAndRemoveLifecycleFailure(pending, now(), effect);
				}
			}));
		}
		catch (error) {
			invalidate("LIFECYCLE_RAF_SCHEDULE_FAILURE", error instanceof Error ? error.message : String(error), pending.identity);
			recordAndRemoveLifecycleFailure(pending, now(), effect);
		}
	}

	function verifyLifecycleMainEffect(
		pending: Web06PendingLifecycle,
		effect: Web06LifecycleOutcomeReceipt["effect"],
	): { valid: boolean; digest: string } {
		try {
			if (effect === "listener") {
				const latestByState = new Map<string, Web06MainListenerEffect>();
				for (const listenerEffect of pending.listenerEffects ?? []) latestByState.set(listenerEffect.stateKey, listenerEffect);
				const states = [...latestByState.values()].map(listenerEffect => ({
					name: listenerEffect.name,
					argsDigest: listenerEffect.argsDigest,
					stateKey: listenerEffect.stateKey,
					expectedState: listenerEffect.expectedState,
					observedState: listenerEffect.readState(),
				}));
				return {
					valid: states.length > 0 && states.every(state => web06ValuesEqual(state.expectedState, state.observedState)),
					digest: web06StableDigest({ effect, states }),
				};
			}
			if (pending.expectedOwnerEffect !== undefined) {
				const expectedState = pending.ownerProof?.expectedState;
				const observedState = pending.ownerProof?.readObservedState();
				return {
					valid: expectedState !== undefined && observedState !== undefined && web06ValuesEqual(expectedState, observedState),
					digest: web06StableDigest({ effect, expectedState, observedState }),
				};
			}
			const digest = web06StableDigest({ effect, workerEffectDigest: pending.workerEffectDigest, postEffectObservation: "double-raf-complete" });
			return { valid: true, digest };
		}
		catch (error) {
			return { valid: false, digest: web06StableDigest({ effect, proofFailure: error instanceof Error ? `${error.name}: ${error.message}` : String(error) }) };
		}
	}

	function recordAndRemoveLifecycleFailure(
		pending: Web06PendingLifecycle,
		terminalObservedAt: number,
		effect: Web06LifecycleOutcomeReceipt["effect"],
	): void {
		const effectDigest = web06StableDigest({ workerEffectDigest: pending.workerEffectDigest, mainEffectDigest: pending.mainEffectDigest, effect, failure: true });
		recordLifecycleOutcome({
			identity: pending.identity,
			outcome: "failure",
			stateUpdateScheduledAt: terminalObservedAt,
			terminalObservedAt,
			effect,
			effectDigest,
			workerEffectDigest: pending.workerEffectDigest,
			mainEffectDigest: pending.mainEffectDigest,
			listenerEffectCount: pending.listenerEffectCount,
			persistenceCompleted: pending.persistenceCompleted,
		});
		pendingLifecycles.delete(pending.identity.sequenceId);
		writeTerminalMarker(pending.identity, "failure", effectDigest);
	}

	function recordLifecycleOutcome(outcome: Web06LifecycleOutcomeReceipt): void {
		observe("record-lifecycle-outcome", outcome.identity, () => {
			const receipt = actionReceipts.get(outcome.identity.sequenceId);
			if (receipt === undefined) return;
			if (receipt.presentation !== undefined || receipt.lifecycle !== undefined) {
				invalidate("DUPLICATE_TERMINAL_OUTCOME", `Action ${outcome.identity.actionId} already has a terminal outcome`, outcome.identity);
				return;
			}
			if (!web06TimestampsAreOrdered(receipt.mainResponseReceivedAt, outcome.stateUpdateScheduledAt, outcome.firstRafAt, outcome.terminalObservedAt)) {
				invalidate("LIFECYCLE_TIMESTAMP_ORDER", `Action ${outcome.identity.actionId} recorded out-of-order lifecycle timestamps`, outcome.identity);
			}
			receipt.lifecycle = { ...outcome, identity: web06RetainedActionIdentity(mode, outcome.identity) };
			actionReceipts.set(receipt);
		});
	}

	function writeTerminalMarker(identity: Web06ActionIdentity, outcome: "barrier-completed" | "failure", effectDigest: string): void {
		try {
			document.documentElement.dataset["yuneWeb06Terminal"] = [
				WEB06_PRIVATE_PROTOCOL_VERSION,
				identity.sequenceId,
				identity.compositionEpochId,
				identity.supersessionSubRunId,
				outcome,
				effectDigest,
			].join("|");
		}
		catch (error) {
			invalidate("DOM_TERMINAL_MARKER_FAILURE", error instanceof Error ? `${error.name}: ${error.message}` : String(error), identity);
		}
	}

	function recordPresentationOutcome(outcome: Web06PresentationOutcomeInput): void {
		observe("record-presentation-outcome", outcome.identity, () => {
			// Terminal arrival always retires its working token, even when the
			// authoritative bounded receipt has already been evicted.
			pendingPresentations.delete(outcome.identity.sequenceId);
			const receipt = actionReceipts.get(outcome.identity.sequenceId);
			if (receipt === undefined) return;
			if (receipt.presentation !== undefined || receipt.lifecycle !== undefined) {
				invalidate("DUPLICATE_TERMINAL_OUTCOME", `Action ${outcome.identity.actionId} already has a terminal outcome`, outcome.identity);
				return;
			}
			let legal = true;
			if (!web06ActionIdentitiesEqual(receipt.identity, web06RetainedActionIdentity(mode, outcome.identity))) {
				legal = false;
				invalidate("PRESENTATION_ACTION_IDENTITY_MISMATCH", `Action ${outcome.identity.actionId} terminal identity does not match its enqueue receipt`, outcome.identity);
			}
			if (!web06TimestampsAreOrdered(receipt.responseMappingFinishedAt, outcome.stateUpdateScheduledAt, outcome.stateCommittedAt, outcome.firstRafAt, outcome.paintObservedAt, outcome.terminalObservedAt)) {
				legal = false;
				invalidate("PRESENTATION_TIMESTAMP_ORDER", `Action ${outcome.identity.actionId} recorded out-of-order presentation timestamps`, outcome.identity);
			}
			const presentationExpectedDigest = web06PresentationFingerprintDigest(outcome.presentationExpected);
			const domObservedDigest = web06PresentationFingerprintDigest(outcome.domObserved);
			const rawProof = receipt.worker?.engineRaw;
			if (
				rawProof === undefined
				|| rawProof.action !== receipt.name
				|| rawProof.availability === "missing"
				|| rawProof.availability === "captured-error"
				|| (rawProof.availability === "captured" && (
					!rawProof.projectionMatches
					|| rawProof.adapterProjection === undefined
					|| !web06AdapterProjectionFingerprintsEqual(rawProof.adapterProjection, outcome.adapterProjection)
					|| rawProof.adapterProjectionDigest !== outcome.adapterProjectionDigest
				))
			) {
				legal = false;
				invalidate("ENGINE_RAW_PROJECTION_MISMATCH", `Action ${outcome.identity.actionId} did not prove its raw-to-adapter presentation source`, outcome.identity);
			}
			if (outcome.presentationDigest !== domObservedDigest) {
				legal = false;
				invalidate("PRESENTATION_DIGEST_MISMATCH", `Action ${outcome.identity.actionId} supplied a DOM digest that does not match its observed fingerprint`, outcome.identity);
			}
			const expectedObservedSequenceId = outcome.supersededBySequenceId ?? outcome.identity.sequenceId;
			if (outcome.presentationExpected.sequenceId !== outcome.identity.sequenceId || outcome.domObserved.sequenceId !== expectedObservedSequenceId) {
				legal = false;
				invalidate("PRESENTATION_SEQUENCE_TOKEN_MISMATCH", `Action ${outcome.identity.actionId} expected/observed sequence tokens ${outcome.presentationExpected.sequenceId}/${outcome.domObserved.sequenceId}; required ${outcome.identity.sequenceId}/${expectedObservedSequenceId}`, outcome.identity);
			}
			const violation = presentationOutcomeViolation(receipt, outcome);
			if (violation !== undefined) {
				legal = false;
				invalidate("ILLEGAL_PRESENTATION_OUTCOME", violation, outcome.identity);
			}
			const stored: Web06PresentationOutcomeReceipt = {
				identity: web06RetainedActionIdentity(mode, outcome.identity),
				outcome: legal ? outcome.outcome : "failure",
				stateUpdateScheduledAt: outcome.stateUpdateScheduledAt,
				stateCommittedAt: outcome.stateCommittedAt,
				firstRafAt: outcome.firstRafAt,
				paintObservedAt: outcome.paintObservedAt,
				terminalObservedAt: outcome.terminalObservedAt,
				beforePresentationDigest: web06PresentationStateDigest(outcome.beforePresentation),
				adapterProjectionDigest: outcome.adapterProjectionDigest,
				...(mode === "full" ? { adapterProjection: outcome.adapterProjection } : {}),
				presentationExpectedDigest,
				domObservedDigest,
				presentationDigest: domObservedDigest,
				supersededBySequenceId: outcome.supersededBySequenceId,
				supersessionSequenceLag: outcome.supersessionSequenceLag,
				...(mode === "full" ? { presentationExpected: outcome.presentationExpected, domObserved: outcome.domObserved } : {}),
			};
			receipt.presentation = stored;
			actionReceipts.set(receipt);
			presentationTerminalProofs.set(outcome.identity.sequenceId, {
				outcome: stored.outcome,
				expected: cloneForExport(outcome.presentationExpected),
				observed: cloneForExport(outcome.domObserved),
			});
			while (presentationTerminalProofs.size > 4) {
				const first = presentationTerminalProofs.keys().next().value as number | undefined;
				if (first === undefined) break;
				presentationTerminalProofs.delete(first);
			}
		});
	}

	function presentationOutcomeViolation(receipt: Web06ActionReceipt, outcome: Web06PresentationOutcomeInput): string | undefined {
		if (web06TerminalContract(receipt.name).strategy !== "presentation") return `${receipt.name} is not owned by the presentation terminal lane`;
		if (outcome.outcome === "failure") return undefined;
		const exact = web06PresentationFingerprintsEqual(outcome.presentationExpected, outcome.domObserved);
		const visualChanged = !web06PresentationFingerprintsEqual(outcome.beforePresentation, outcome.domObserved, false);
		switch (outcome.outcome) {
			case "painted":
				return outcome.paintObservedAt !== outcome.terminalObservedAt || !Number.isFinite(outcome.paintObservedAt)
					? `${receipt.name} painted without an exact finite paint observation`
					: !exact ? `${receipt.name} painted without an exact expected/observed fingerprint`
					: !visualChanged ? `${receipt.name} claimed painted without a visible state change` : undefined;
			case "committed": {
				const summary = receipt.worker?.resultSummary;
				return !exact ? `${receipt.name} committed without an exact expected/observed fingerprint`
					: !visualChanged ? `${receipt.name} committed without a visible terminal change`
					: summary?.committedTextDigest === undefined || summary.committedUtf16Length === undefined || summary.persistenceCompleted !== true
						? `${receipt.name} committed without exact worker commit/persistence proof` : undefined;
			}
			case "processed-no-visual-change":
				return receipt.name !== "stageAi" || outcome.identity.actionClass !== "adapter-only" || outcome.identity.boundary !== "none" || outcome.identity.supersedable
					? `${receipt.name} is not allowed to claim processed-no-visual-change`
					: !exact || visualChanged ? `${receipt.name} did not prove an exact unchanged presentation` : undefined;
			case "barrier-completed":
				return outcome.identity.actionClass !== "stateful-barrier" || outcome.identity.boundary === "none" || outcome.identity.supersedable
					? `${receipt.name} is not an unchanged-state presentation barrier`
					: !exact || visualChanged ? `${receipt.name} did not prove an exact unchanged barrier state` : undefined;
			case "superseded": {
				const targetSequenceId = outcome.supersededBySequenceId;
				const target = targetSequenceId === undefined ? undefined : actionIdentityBySequence.get(targetSequenceId);
				const covering = targetSequenceId === undefined ? undefined : presentationTerminalProofs.get(targetSequenceId);
				const lag = targetSequenceId === undefined ? undefined : targetSequenceId - outcome.identity.sequenceId;
				return outcome.paintObservedAt !== outcome.terminalObservedAt
					|| !Number.isFinite(outcome.paintObservedAt)
					|| target === undefined
					|| covering?.outcome !== "painted"
					|| !web06PresentationFingerprintsEqual(covering.expected, covering.observed)
					|| !web06PresentationFingerprintsEqual(covering.observed, outcome.domObserved)
					|| !canSupersede(outcome.identity, target)
					|| outcome.supersessionSequenceLag !== lag
					|| !outcome.domObserved.input.startsWith(outcome.presentationExpected.input)
					|| outcome.domObserved.input.length <= outcome.presentationExpected.input.length
					? `${receipt.name} supplied an invalid supersession link or covering state` : undefined;
			}
		}
	}

	function canSupersede(from: Web06ActionIdentity, to: Web06ActionIdentity): boolean {
		if (
			!from.supersedable
			|| !to.supersedable
			|| from.compositionEpochId !== to.compositionEpochId
			|| from.supersessionSubRunId !== to.supersessionSubRunId
			|| to.sequenceId <= from.sequenceId
			|| to.sequenceId - from.sequenceId > 2
			|| !strictArrayPrefix(from.rawInputSequence, to.rawInputSequence)
		) return false;
		for (let sequenceId = from.sequenceId + 1; sequenceId <= to.sequenceId; sequenceId += 1) {
			const identity = actionIdentityBySequence.get(sequenceId);
			if (identity === undefined || !identity.supersedable || identity.compositionEpochId !== from.compositionEpochId || identity.supersessionSubRunId !== from.supersessionSubRunId) return false;
		}
		return true;
	}

	function strictArrayPrefix(left: string[], right: string[]): boolean {
		return left.length < right.length && left.every((value, index) => right[index] === value);
	}

	function auditTerminalCompleteness(): void {
		for (const receipt of actionReceipts.values()) {
			if (receipt.resultType === undefined || receipt.presentation !== undefined || receipt.lifecycle !== undefined || auditedMissingTerminals.has(receipt.identity.sequenceId)) continue;
			if (auditedMissingTerminals.size >= web06ReceiptCapacity(mode)) {
				invalidate("AUDITED_TERMINAL_RING_OVERFLOW", `WEB-06 audited terminal IDs exceeded ${web06ReceiptCapacity(mode)} records`);
				return;
			}
			auditedMissingTerminals.add(receipt.identity.sequenceId);
			invalidate("MISSING_TERMINAL_OUTCOME", `Completed action ${receipt.identity.actionId} (${receipt.name}) has no terminal outcome`, receipt.identity);
		}
	}

	function status(): Web06DebugStatus {
		const pendingFanoutActions = pendingFanoutActionCount();
		const pendingTerminalActions = pendingLifecycles.size + pendingPresentations.size;
		const queue = hooks.queueState();
		return {
			valid: invalidations.length === 0 && workerObserverFailuresAudited && pendingFanoutActions === 0 && pendingTerminalActions === 0,
			queueDepth: queue.queueDepth,
			runningActionId: queue.runningIdentity?.actionId,
			lastEventSequenceId: eventSequenceId,
			lastActionSequenceId: actionSequenceId,
			receiptWindowStartEventSequenceId,
			receiptWindowStartActionSequenceId,
			pendingFanoutActions,
			pendingTerminalActions,
			pageInstanceId,
			...(reloadContinuity === undefined ? {} : {
				measurementId: reloadContinuity.measurementId,
				continuityNonce: reloadContinuity.continuityNonce,
				reloadContinuityPhase: reloadContinuity.phase,
			}),
			compositionEpochId,
			supersessionSubRunId,
			mainObserverCallbackCount: mainObserverCallbacks.length,
			mainObserverCallbackCapacity: web06ReceiptCapacity(mode),
			mainObserverCallbackOverflowCount,
			workerObserverFailureCount,
			workerObserverFailuresAudited,
		};
	}

	function snapshot(): Web06AtomicSnapshot {
		auditPendingFanouts();
		auditTerminalCompleteness();
		return cloneForExport({
			header: {
				protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
				mode,
				modeProvenance,
				pageInstanceId,
				timeOrigin: performance.timeOrigin,
				...(reloadContinuity === undefined ? {} : { measurementId: reloadContinuity.measurementId, continuityNonce: reloadContinuity.continuityNonce }),
			},
			status: status(),
			events: eventReceipts.values(),
			actions: actionReceipts.values(),
			invalidations,
			mainObserverCallbacks,
			mainObserverCallbacksMs: mainObserverCallbackDurations(),
			...(reloadContinuity === undefined ? {} : { reloadContinuity }),
		});
	}

	function resetReceipts(): void {
		if (reloadContinuity?.phase === "pre-reload") {
			invalidatePreparedContinuity("reset receipts after reload preparation");
			return;
		}
		const queue = hooks.queueState();
		if (queue.queueDepth > 0 || pendingClockPings.size > 0 || pendingLifecycles.size > 0 || pendingPresentations.size > 0) {
			invalidate("RESET_WHILE_BUSY", "WEB-06 receipts may reset only while the action queue and clock lane are idle");
			return;
		}
		auditPendingFanouts();
		if (pendingFanoutActionCount() > 0) return;
		actionReceipts.clear();
		eventReceipts.clear();
		invalidations.length = 0;
		mainObserverCallbacks.length = 0;
		mainObserverCallbackOverflowCount = 0;
		workerObserverFailureCount = 0;
		workerObserverFailuresAudited = true;
		mainListenerEffectsByAction.clear();
		pendingPresentations.clear();
		presentationTerminalProofs.clear();
		auditedMissingTerminals.clear();
		receiptWindowStartEventSequenceId = eventSequenceId + 1;
		receiptWindowStartActionSequenceId = actionSequenceId + 1;
		auditedIncompleteFanouts.clear();
	}

	function assertQueueAndProtocolIdle(operation: string): void {
		auditPendingFanouts();
		if (
			hooks.queueState().queueDepth > 0
			|| pendingClockPings.size > 0
			|| pendingFanoutActionCount() > 0
			|| pendingLifecycles.size > 0
			|| pendingPresentations.size > 0
		) throw invalidateAndError("RELOAD_CONTINUITY_NOT_IDLE", `${operation} requires idle action, clock, fanout, and terminal lanes`);
	}

	function clockPing(): Promise<Web06ClockExchange> {
		if (reloadContinuity?.phase === "pre-reload") {
			invalidatePreparedContinuity("worker clock calibration after reload preparation");
			return Promise.reject(new Error("WEB-06 reload continuity was voided by post-prepare calibration"));
		}
		if (hooks.queueState().queueDepth > 0) return Promise.reject(new Error("WEB-06 worker clock calibration requires an idle action queue"));
		if (pendingClockPings.size >= 32) {
			invalidate("CLOCK_PING_RING_OVERFLOW", "More than 32 worker clock pings were pending");
			return Promise.reject(new Error("WEB-06 worker clock ping capacity exceeded"));
		}
		const pingId = `web06-clock-${String(++clockPingSequenceId).padStart(6, "0")}`;
		const mainSentAt = now();
		const envelope: Web06ClockPingEnvelope = { protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION, kind: "clock-ping", pingId, mainSentAt };
		return new Promise((resolve, reject) => {
			pendingClockPings.set(pingId, { resolve, reject });
			hooks.postClockPing(envelope);
		});
	}

	function assertMeasurementId(measurementId: string): void {
		if (!/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(measurementId)) {
			throw invalidateAndError("RELOAD_CONTINUITY_MEASUREMENT_ID_INVALID", "Measurement identity must be 1..128 allowlisted ASCII characters");
		}
	}

	function prepareLearnedReloadContinuity(
		measurementId: string,
		expectedTerminalActionId?: string,
	): Web06ReloadContinuityPreReceipt {
		assertMeasurementId(measurementId);
		if (mode === "off") throw invalidateAndError("RELOAD_CONTINUITY_MODE_OFF", "Learned-row reload continuity requires minimal or full observation");
		if (reloadContinuity !== undefined) throw invalidateAndError("RELOAD_CONTINUITY_ALREADY_ACTIVE", "This page instance already owns a reload-continuity receipt");
		assertQueueAndProtocolIdle("prepare learned-row reload continuity");
		if (invalidations.length > 0) throw new Error("WEB-06 learned-row reload continuity cannot start after a measurement invalidation");
		if (lastSentSequenceId !== actionSequenceId || lastCompletedSequenceId !== actionSequenceId) {
			throw invalidateAndError("RELOAD_CONTINUITY_ACTION_LOSS", `Action counters disagree: created=${actionSequenceId}, sent=${lastSentSequenceId}, completed=${lastCompletedSequenceId}`);
		}
		const actions = actionReceipts.values();
		for (const receipt of actions) {
			if (
				receipt.resultType === undefined
				|| receipt.returnedIdentity === undefined
				|| receipt.worker === undefined
				|| receipt.worker.observerFailures.length > 0
				|| Number(receipt.presentation !== undefined) + Number(receipt.lifecycle !== undefined) !== 1
				|| receipt.presentation?.outcome === "failure"
				|| receipt.lifecycle?.outcome === "failure"
			) throw invalidateAndError("RELOAD_CONTINUITY_INCOMPLETE_ACTION", `Action ${receipt.identity.actionId} is not completely and cleanly acknowledged`, receipt.identity);
		}
		for (const event of eventReceipts.values()) {
			if (event.linkedActionIds.length !== event.mappedActions.length || event.linkedActionIds.some(actionId => typeof actionId !== "string")) {
				throw invalidateAndError("RELOAD_CONTINUITY_EVENT_ACTION_LOSS", `Event ${event.identity.eventId} did not link all ${event.mappedActions.length} mapped actions`, event.identity);
			}
		}
		const terminal = [...actions].reverse().find(receipt => (expectedTerminalActionId === undefined || receipt.identity.actionId === expectedTerminalActionId) && receipt.presentation?.outcome === "committed");
		if (terminal?.presentation === undefined || terminal.worker?.resultSummary === undefined) throw invalidateAndError("RELOAD_CONTINUITY_COMMIT_TERMINAL_MISSING", "No exact committed presentation with a worker result proof was found");
		const presentation = terminal.presentation;
		const terminalSummary = terminal.worker.resultSummary;
		if (
			presentation.presentationExpectedDigest !== presentation.domObservedDigest
			|| terminalSummary.committedTextDigest === undefined
			|| terminalSummary.committedUtf16Length === undefined
			|| terminalSummary.persistenceCompleted !== true
		) throw invalidateAndError("RELOAD_CONTINUITY_COMMIT_OR_PERSISTENCE_UNPROVED", `Action ${terminal.identity.actionId} lacks exact terminal/persistence proof`, terminal.identity);
		const userdb = [...actions].reverse().find(receipt => receipt.name === "getUserdbSnapshot" && receipt.identity.causedByActionId === terminal.identity.actionId && receipt.resultType === "success" && receipt.worker?.resultSummary?.kind === "userdb-snapshot");
		const userdbSummary = userdb?.worker?.resultSummary;
		if (userdb === undefined || userdbSummary?.userdbDigest === undefined || userdbSummary.userdbRowCount === undefined || userdbSummary.userdbBytes === undefined) {
			throw invalidateAndError("RELOAD_CONTINUITY_USERDB_PROOF_MISSING", `Committed action ${terminal.identity.actionId} has no completed causally linked userdb snapshot`, terminal.identity);
		}

		const continuityNonce = createSecureNonce();
		const storage = sessionStorageOrUndefined();
		if (storage === undefined) throw invalidateAndError("RELOAD_CONTINUITY_STORAGE_UNAVAILABLE", "sessionStorage is unavailable for the one-shot reload token");
		let existingToken: string | null;
		try { existingToken = storage.getItem(reloadContinuityStorageKey); }
		catch (error) { throw invalidateAndError("RELOAD_CONTINUITY_STORAGE_READ_FAILED", error instanceof Error ? error.message : String(error)); }
		if (existingToken !== null) throw invalidateAndError("RELOAD_CONTINUITY_STORAGE_OCCUPIED", "A prior one-shot reload token is still present");
		const token: Web06ReloadContinuityToken = { measurementId, continuityNonce };
		const tokenJson = JSON.stringify(token);
		let verifiedToken: string | null;
		try {
			storage.setItem(reloadContinuityStorageKey, tokenJson);
			verifiedToken = storage.getItem(reloadContinuityStorageKey);
		}
		catch (error) {
			try { storage.removeItem(reloadContinuityStorageKey); } catch { /* original failure is authoritative */ }
			throw invalidateAndError("RELOAD_CONTINUITY_STORAGE_WRITE_FAILED", error instanceof Error ? error.message : String(error));
		}
		if (verifiedToken !== tokenJson) {
			try { storage.removeItem(reloadContinuityStorageKey); } catch { /* mismatch already invalidates */ }
			throw invalidateAndError("RELOAD_CONTINUITY_STORAGE_VERIFY_FAILED", "The one-shot reload token did not round-trip byte-exactly");
		}
		const receipt: Web06ReloadContinuityPreReceipt = {
			phase: "pre-reload",
			protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
			...token,
			pageInstanceId,
			timeOrigin: performance.timeOrigin,
			preparedAt: now(),
			receiptWindowStartEventSequenceId,
			receiptWindowStartActionSequenceId,
			lastEventSequenceId: eventSequenceId,
			lastActionSequenceId: actionSequenceId,
			lastSentSequenceId,
			lastCompletedSequenceId,
			compositionEpochId,
			supersessionSubRunId,
			terminal: {
				actionId: terminal.identity.actionId,
				sequenceId: terminal.identity.sequenceId,
				outcome: "committed",
				terminalObservedAt: presentation.terminalObservedAt,
				presentationDigest: presentation.presentationDigest,
				committedTextDigest: terminalSummary.committedTextDigest,
				committedUtf16Length: terminalSummary.committedUtf16Length,
				persistenceCompleted: true,
			},
			userdb: { actionId: userdb.identity.actionId, sequenceId: userdb.identity.sequenceId, digest: userdbSummary.userdbDigest, rowCount: userdbSummary.userdbRowCount, bytes: userdbSummary.userdbBytes },
			queueIdle: true,
			allActionsCompleted: true,
			storagePayloadKeys: ["measurementId", "continuityNonce"],
		};
		reloadContinuity = receipt;
		return cloneForExport(receipt);
	}

	function bindLearnedReloadWindow(measurementId: string, continuityNonce: string): Web06ReloadContinuityPostReceipt {
		const arrived = reloadContinuity;
		if (arrived?.phase !== "post-reload-arrived") throw invalidateAndError("RELOAD_CONTINUITY_ARRIVAL_MISSING", "No unbound one-shot reload token arrived in this page realm");
		if (arrived.measurementId !== measurementId || arrived.continuityNonce !== continuityNonce) throw invalidateAndError("RELOAD_CONTINUITY_TOKEN_MISMATCH", "The post-reload measurement identity/nonce does not match the consumed token");
		assertQueueAndProtocolIdle("bind learned-row post-reload window");
		if (
			actionReceipts.values().length !== 0
			|| eventReceipts.size !== 0
			|| mainObserverCallbacks.length !== 0
			|| receiptWindowStartEventSequenceId !== eventSequenceId + 1
			|| receiptWindowStartActionSequenceId !== actionSequenceId + 1
		) throw invalidateAndError("RELOAD_CONTINUITY_WINDOW_NOT_RESET", "Post-reload receipts must be reset after initialization and before calibration/window binding");
		if (invalidations.length > 0) throw new Error("WEB-06 post-reload window cannot bind after a measurement invalidation");
		const bound: Web06ReloadContinuityPostReceipt = {
			...arrived,
			phase: "post-reload-bound",
			boundAt: now(),
			receiptWindowStartEventSequenceId,
			receiptWindowStartActionSequenceId,
			lastEventSequenceId: eventSequenceId,
			lastActionSequenceId: actionSequenceId,
			compositionEpochId,
			supersessionSubRunId,
			requiresFreshDriverPageCalibration: true,
			requiresFreshWorkerCalibration: true,
		};
		reloadContinuity = bound;
		return cloneForExport(bound);
	}

	const debugApi: YuneWeb06DebugApi = {
		protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
		mode,
		modeProvenance,
		status,
		events: () => cloneForExport(eventReceipts.values()),
		actions: () => {
			auditPendingFanouts();
			auditTerminalCompleteness();
			return cloneForExport(actionReceipts.values());
		},
		invalidations: () => {
			auditPendingFanouts();
			auditTerminalCompleteness();
			return cloneForExport(invalidations);
		},
		mainObserverCallbacks: () => cloneForExport(mainObserverCallbacks),
		mainObserverCallbacksMs: () => cloneForExport(mainObserverCallbackDurations()),
		snapshot,
		clockPing,
		resetReceipts,
		prepareLearnedReloadContinuity,
		bindLearnedReloadWindow,
	};

	return {
		mode,
		modeProvenance,
		debugApi,
		observeMeasurement: observe,
		invalidate,
		invalidatePreparedContinuity,
		recordDomEvent,
		withControlEvent,
		declareControlFanout,
		registerEventFanout,
		resolveDeferredFanoutAction,
		cancelEventFanout,
		withOwnedAction,
		mappedActionContext,
		advanceBoundary,
		withActionContext,
		createActionIdentity,
		bindActionPromise,
		actionIdentityFor,
		markActionSent,
		recordResponseMapping,
		recordPresentationOutcome,
		recordOwnedResultEffect,
		canSupersede,
		recordListener,
		handleClockEcho,
		handleActionResult,
	};
}

export type Web06MainProtocol = ReturnType<typeof createWeb06MainProtocol>;

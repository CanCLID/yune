import type {
	Actions,
	ListenerArgsMap,
	RimeResult,
	Web06ActionIdentity,
	Web06CollectionMode,
	Web06CollectionModeProvenance,
	Web06WorkerActionEnvelope,
	Web06WorkerReceipt,
	Web06WorkerResultSummary,
	YuneWebUserdbSnapshot,
} from "../types";
import { WEB06_PRIVATE_PROTOCOL_VERSION } from "../types";
import {
	web06AdapterProjectionFingerprint,
	web06AdapterProjectionFingerprintsEqual,
	web06DeployCacheSnapshotDigest,
	web06EngineRawAdapterProjection,
	web06EngineRawFingerprint,
	web06InjectedAssetManifestDigest,
	web06RawResponseNotApplicableReason,
	web06StableDigest,
	web06UserdbSnapshotDigest,
} from "./private-protocol";

import type { Web06AdapterObservation } from "./adapter";

const WEB06_WORKER_ACTION_ID_CAPACITY = 8_192;
const WEB06_WORKER_RECEIPT_LEDGER_CAPACITY = 256;
const WEB06_WORKER_FAILURE_CAPACITY = 64;

export interface Web06WorkerProtocolOptions {
	mode: Web06CollectionMode;
	modeProvenance: Web06CollectionModeProvenance;
	now(): number;
	postDiagnostic(marker: string): void;
}

export interface Web06WorkerActionSession {
	envelope: Web06WorkerActionEnvelope;
	receipt: Web06WorkerReceipt;
}

export interface Web06WorkerListenerMetadata {
	protocolVersion: typeof WEB06_PRIVATE_PROTOCOL_VERSION;
	actionId: string;
	sequenceId: number;
}

export function createWeb06WorkerProtocol(options: Web06WorkerProtocolOptions) {
	let activeReceipt: Web06WorkerReceipt | null = null;
	let activeIdentity: Web06ActionIdentity | null = null;
	let expectedActionSequenceId = 1;
	const seenActionIds = new Set<string>();
	const seenActionIdOrder = new Array<string>(WEB06_WORKER_ACTION_ID_CAPACITY);
	const ledgerOverflows = new Set<string>();
	let seenActionIdCount = 0;
	let seenActionIdCursor = 0;
	let seenActionIdOverflowed = false;

	function pushFailure(receipt: Web06WorkerReceipt, detail: string): void {
		if (receipt.observerFailures.length < WEB06_WORKER_FAILURE_CAPACITY) {
			receipt.observerFailures.push(detail);
			return;
		}
		if (!ledgerOverflows.has("observer-failures")) {
			ledgerOverflows.add("observer-failures");
			receipt.observerFailures[WEB06_WORKER_FAILURE_CAPACITY - 1] =
				`observer-failures exceeded ${WEB06_WORKER_FAILURE_CAPACITY} records`;
		}
	}

	function pushLedger<T>(
		ledger: T[],
		value: T,
		kind: string,
		receipt: Web06WorkerReceipt,
	): void {
		if (ledger.length < WEB06_WORKER_RECEIPT_LEDGER_CAPACITY) {
			ledger.push(value);
			return;
		}
		if (!ledgerOverflows.has(kind)) {
			ledgerOverflows.add(kind);
			pushFailure(receipt, `${kind} exceeded ${WEB06_WORKER_RECEIPT_LEDGER_CAPACITY} records`);
		}
	}

	function stringifyFailure(value: unknown): string {
		if (value instanceof Error) return `${value.name}: ${value.message}`;
		try {
			return JSON.stringify(value);
		}
		catch {
			return String(value);
		}
	}

	function recordCollectorCallback(operation: string, callback: () => void): void {
		const receipt = activeReceipt;
		const startedAt = options.now();
		let outcome: "success" | "error" = "success";
		try {
			callback();
		}
		catch (error) {
			outcome = "error";
			if (receipt !== null) pushFailure(receipt, `${operation}: ${stringifyFailure(error)}`);
		}
		finally {
			const finishedAt = options.now();
			if (receipt !== null && finishedAt - startedAt >= 5) {
				pushFailure(receipt, `${operation} callback took ${finishedAt - startedAt} ms`);
			}
			if (receipt !== null) {
				pushLedger(receipt.collectorSpans, {
					component: "collector",
					operation,
					stage: "callback",
					startedAt,
					finishedAt,
					outcome,
				}, "collector-spans", receipt);
			}
		}
	}

	const adapterObservation: Web06AdapterObservation = {
		mode: options.mode,
		now: options.now,
		onSpan(span) {
			recordCollectorCallback(`span:${span.component}:${span.stage}`, () => {
				const receipt = activeReceipt;
				if (receipt === null) return;
				switch (span.component) {
					case "runtime":
						pushLedger(receipt.runtimeSpans, span, "runtime-spans", receipt);
						break;
					case "adapter":
						pushLedger(receipt.adapterSpans, span, "adapter-spans", receipt);
						break;
					case "persistence":
						pushLedger(receipt.persistenceSpans, span, "persistence-spans", receipt);
						break;
					case "collector":
						pushLedger(receipt.collectorSpans, span, "collector-spans", receipt);
						break;
				}
			});
		},
		onEngineRawJson(copy) {
			recordCollectorCallback(`engine-raw:${copy.operation}`, () => {
				const receipt = activeReceipt;
				if (receipt === null) return;
				if (receipt.engineRawJson !== undefined) {
					pushFailure(receipt, `duplicate engineRaw JSON for ${copy.operation}`);
					return;
				}
				receipt.engineRawJson = copy.json;
				receipt.engineRawOperation = copy.operation;
			});
		},
		onFailure(failure) {
			recordCollectorCallback("observer-failure", () => {
				const detail = stringifyFailure(failure);
				if (activeReceipt !== null) pushFailure(activeReceipt, detail);
				else options.postDiagnostic(detail);
			});
		},
	};

	function retainActionId(actionId: string, failures: string[]): void {
		seenActionIds.add(actionId);
		if (seenActionIdCount < WEB06_WORKER_ACTION_ID_CAPACITY) {
			seenActionIdOrder[seenActionIdCount] = actionId;
			seenActionIdCount += 1;
			return;
		}
		if (!seenActionIdOverflowed) {
			seenActionIdOverflowed = true;
			failures.push(`worker action-ID ring exceeded ${WEB06_WORKER_ACTION_ID_CAPACITY} records`);
		}
		const evicted = seenActionIdOrder[seenActionIdCursor];
		if (evicted !== undefined) seenActionIds.delete(evicted);
		seenActionIdOrder[seenActionIdCursor] = actionId;
		seenActionIdCursor = (seenActionIdCursor + 1) % WEB06_WORKER_ACTION_ID_CAPACITY;
	}

	function beginAction(
		envelope: Web06WorkerActionEnvelope,
		workerMessageReceivedAt: number,
		workerActionStartedAt: number,
	): Web06WorkerActionSession {
		const protocolFailures: string[] = [];
		if (envelope.protocolVersion !== WEB06_PRIVATE_PROTOCOL_VERSION) {
			protocolFailures.push(`protocol ${envelope.protocolVersion}`);
		}
		if (envelope.mode !== options.mode) {
			protocolFailures.push(`mode ${envelope.mode}, expected ${options.mode}`);
		}
		if (envelope.modeProvenance !== options.modeProvenance) {
			protocolFailures.push(`mode provenance ${envelope.modeProvenance}, expected ${options.modeProvenance}`);
		}
		if (seenActionIds.has(envelope.identity.actionId)) {
			protocolFailures.push(`duplicate actionId ${envelope.identity.actionId}`);
		}
		if (envelope.identity.sequenceId !== expectedActionSequenceId) {
			protocolFailures.push(`sequence ${envelope.identity.sequenceId}, expected ${expectedActionSequenceId}`);
		}
		retainActionId(envelope.identity.actionId, protocolFailures);
		expectedActionSequenceId = Math.max(expectedActionSequenceId, envelope.identity.sequenceId + 1);
		ledgerOverflows.clear();
		const receipt: Web06WorkerReceipt = {
			workerMessageReceivedAt,
			workerActionStartedAt,
			workerFinishedAt: workerActionStartedAt,
			runtimeSpans: [],
			adapterSpans: [],
			persistenceSpans: [],
			collectorSpans: [],
			engineRaw: options.mode === "full"
				? { availability: "missing", action: envelope.name, reason: "required-runtime-response-missing" }
				: { availability: "not-collected", action: envelope.name, reason: "minimal-content-free" },
			observerFailures: protocolFailures,
			lifecycleEffects: [],
		};
		if (activeReceipt !== null) pushFailure(receipt, "worker received an action while another receipt was active");
		activeReceipt = receipt;
		activeIdentity = envelope.identity;
		return { envelope, receipt };
	}

	function listenerMetadata<K extends keyof ListenerArgsMap>(
		name: K,
		args: ListenerArgsMap[K],
	): Web06WorkerListenerMetadata | undefined {
		if (activeReceipt !== null) {
			recordCollectorCallback(`listener:${name}`, () => {
				const receipt = activeReceipt;
				if (receipt === null) return;
				pushLedger(receipt.lifecycleEffects, {
					kind: "listener",
					name,
					argsDigest: web06StableDigest(args),
					args: [...args],
					recordedAt: options.now(),
				}, "lifecycle-effects", receipt);
			});
		}
		return activeIdentity === null ? undefined : {
			protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
			actionId: activeIdentity.actionId,
			sequenceId: activeIdentity.sequenceId,
		};
	}

	function finalizeSuccess(session: Web06WorkerActionSession, result: unknown): void {
		recordCollectorCallback("engine-raw-proof", () => {
			finalizeEngineRawSuccess(session.receipt, session.envelope.name, session.envelope.args, result);
		});
		recordCollectorCallback("result-summary", () => {
			session.receipt.resultSummary = workerResultSummary(session.envelope.name, session.envelope.args, result);
			const semanticEffect = workerSemanticEffect(session.envelope.name, session.envelope.args);
			if (semanticEffect !== undefined) {
				pushLedger(session.receipt.lifecycleEffects, {
					kind: semanticEffect,
					name: session.envelope.name,
					resultDigest: session.receipt.resultSummary.resultDigest,
					recordedAt: options.now(),
				}, "lifecycle-effects", session.receipt);
			}
		});
	}

	function finalizeError(session: Web06WorkerActionSession): void {
		recordCollectorCallback("engine-raw-error-proof", () => {
			finalizeEngineRawError(session.receipt, session.envelope.name);
		});
	}

	function finishAction(session: Web06WorkerActionSession, workerFinishedAt: number): void {
		session.receipt.workerFinishedAt = workerFinishedAt;
		if (activeReceipt !== session.receipt || activeIdentity?.actionId !== session.envelope.identity.actionId) {
			pushFailure(session.receipt, "worker action session lost its active receipt identity");
		}
		activeReceipt = null;
		activeIdentity = null;
	}

	function finalizeEngineRawSuccess(
		receipt: Web06WorkerReceipt,
		name: keyof Actions,
		args: readonly unknown[],
		result: unknown,
	): void {
		if (options.mode !== "full") return;
		const json = receipt.engineRawJson;
		const operation = receipt.engineRawOperation;
		if (json === undefined || operation === undefined) {
			const reason = web06RawResponseNotApplicableReason(name, args);
			receipt.engineRaw = reason === undefined
				? { availability: "missing", action: name, reason: "required-runtime-response-missing" }
				: { availability: "not-applicable", action: name, reason };
			if (reason === undefined) pushFailure(receipt, `${name} completed without its required raw runtime response`);
			return;
		}
		if (!isRimeResult(result)) {
			pushFailure(receipt, `${name} emitted raw runtime JSON for a non-response action`);
			return;
		}
		const rawFingerprint = web06EngineRawFingerprint(name, operation, json);
		const rawProjection = web06EngineRawAdapterProjection(rawFingerprint);
		const adapterProjection = web06AdapterProjectionFingerprint(result);
		const projectionMatches = web06AdapterProjectionFingerprintsEqual(rawProjection, adapterProjection);
		receipt.engineRaw = {
			availability: "captured",
			action: name,
			operation,
			jsonDigest: web06StableDigest(json),
			rawFingerprintDigest: web06StableDigest(rawFingerprint),
			rawProjectionDigest: web06StableDigest(rawProjection),
			adapterProjectionDigest: web06StableDigest(adapterProjection),
			projectionMatches,
			adapterProjection,
			rawFingerprint,
		};
		if (!projectionMatches) pushFailure(receipt, `${name} raw-to-adapter projection digest mismatch`);
	}

	function finalizeEngineRawError(receipt: Web06WorkerReceipt, name: keyof Actions): void {
		if (options.mode !== "full") return;
		const json = receipt.engineRawJson;
		const operation = receipt.engineRawOperation;
		if (json === undefined || operation === undefined) {
			receipt.engineRaw = { availability: "not-applicable", action: name, reason: "action-failed-before-runtime-response" };
			return;
		}
		const rawFingerprint = web06EngineRawFingerprint(name, operation, json);
		receipt.engineRaw = {
			availability: "captured-error",
			action: name,
			operation,
			jsonDigest: web06StableDigest(json),
			rawFingerprintDigest: web06StableDigest(rawFingerprint),
			reason: "action-result-error",
			rawFingerprint,
		};
	}

	return {
		adapterObservation,
		beginAction,
		listenerMetadata,
		finalizeSuccess,
		finalizeError,
		finishAction,
	};
}

export async function invokeWeb06PublicAction(
	actions: Actions,
	name: keyof Actions,
	args: unknown[],
): Promise<unknown> {
	// The private envelope never reaches the public action. This is the sole
	// dispatch seam shared by the worker entrypoint and the equivalence test.
	return (actions[name] as (...actionArgs: unknown[]) => Promise<unknown>)(...args);
}

function workerSemanticEffect(
	name: keyof Actions,
	args: readonly unknown[],
): "engine-state" | "engine-persistence" | "cache-invalidation" | "snapshot-read" | undefined {
	switch (name) {
		case "setOption":
		case "selectSchema":
			return "engine-state";
		case "customize":
			return customizeUsesPersistence(args[0]) ? "engine-persistence" : "engine-state";
		case "importUserdb":
		case "customizeValue":
		case "deploy":
			return "engine-persistence";
		case "invalidateDeployCache":
			return "cache-invalidation";
		case "getUserdbSnapshot":
		case "deployCacheSnapshot":
		case "injectedAssetsManifest":
			return "snapshot-read";
		case "processKey":
		case "stageAi":
		case "selectCandidate":
		case "deleteCandidate":
		case "flipPage":
			return undefined;
	}
}

function workerResultSummary(
	name: keyof Actions,
	args: readonly unknown[],
	result: unknown,
): Web06WorkerResultSummary {
	const persistenceCompleted = name === "importUserdb"
		|| (name === "customize" && customizeUsesPersistence(args[0]))
		|| name === "customizeValue"
		|| name === "deploy"
		|| name === "invalidateDeployCache"
		|| ((name === "processKey" || name === "selectCandidate") && isRimeResult(result) && result.committed !== undefined);
	const successfulPersistence = persistenceCompleted
		&& (typeof result !== "boolean" || result)
		&& (!isRimeResult(result) || result.success);
	if (typeof result === "boolean") {
		return { kind: "boolean", resultDigest: web06StableDigest(result), success: result, persistenceCompleted: successfulPersistence };
	}
	if (result === undefined) {
		return { kind: "empty", resultDigest: web06StableDigest(null), success: true, persistenceCompleted: successfulPersistence };
	}
	if (isRimeResult(result)) {
		const summary = {
			success: result.success,
			isComposing: result.isComposing,
			committedTextDigest: result.committed === undefined ? undefined : web06StableDigest(result.committed),
			committedUtf16Length: result.committed?.length,
		};
		return {
			kind: "rime-result",
			resultDigest: web06StableDigest(summary),
			success: result.success,
			persistenceCompleted: successfulPersistence,
			...(result.committed === undefined ? {} : {
				committedTextDigest: summary.committedTextDigest,
				committedUtf16Length: summary.committedUtf16Length,
			}),
		};
	}
	if (isUserdbSnapshot(result)) {
		const userdbDigest = web06UserdbSnapshotDigest(result);
		return {
			kind: "userdb-snapshot",
			resultDigest: userdbDigest,
			success: true,
			persistenceCompleted: successfulPersistence,
			userdbDigest,
			userdbRowCount: result.rows.length,
			userdbBytes: result.bytes,
		};
	}
	if (isDeployCacheSnapshotResult(result)) {
		return { kind: "deploy-cache-snapshot", resultDigest: web06DeployCacheSnapshotDigest(result), success: true, persistenceCompleted: successfulPersistence };
	}
	return { kind: "asset-manifest", resultDigest: web06InjectedAssetManifestDigest(result as never), success: true, persistenceCompleted: successfulPersistence };
}

function customizeUsesPersistence(value: unknown): boolean {
	return value !== null
		&& typeof value === "object"
		&& !Array.isArray(value)
		&& Object.keys(value).some(key => key !== "enableAI");
}

function isRimeResult(value: unknown): value is RimeResult {
	return value !== null && typeof value === "object" && "isComposing" in value && "success" in value;
}

function isUserdbSnapshot(value: unknown): value is YuneWebUserdbSnapshot {
	return value !== null && typeof value === "object" && "rows" in value && "rawText" in value && "dictionaryId" in value;
}

function isDeployCacheSnapshotResult(
	value: unknown,
): value is Awaited<ReturnType<Actions["deployCacheSnapshot"]>> {
	return value !== null && typeof value === "object" && "cacheFresh" in value && "deployedSchemaExists" in value;
}

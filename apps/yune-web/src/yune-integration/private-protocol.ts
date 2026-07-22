import { OUTPUT_STANDARD_ENGINE_OPTIONS, RIME_KEY_MAP, outputOptionForStandard } from "../consts";

import type {
	Actions,
	RimeResult,
	Web06ActionIdentity,
	Web06AdapterProjectionFingerprint,
	RimeSchemaId,
	Web06ActionClass,
	Web06BoundaryKind,
	Web06CollectionMode,
	Web06CollectionModeProvenance,
	Web06ControlEventLike,
	Web06DomEventSnapshot,
	Web06DomEventIdentity,
	Web06EventMapResult,
	Web06MappedAction,
	Web06PresentationFingerprint,
	Web06EngineRawFingerprint,
	YuneDeployCacheSnapshot,
	YuneInjectedAssetManifest,
	YuneWebUserdbSnapshot,
} from "../types";
import type { OutputStandard } from "../consts";

export const WEB06_MODE_QUERY = "yuneWeb06Mode";
export const WEB06_IMPORT_CONTINUATION_EVENT = "yune-web06-import-enqueued";
export const WEB06_MINIMAL_RECEIPT_CAPACITY = 2_048;
export const WEB06_FULL_RECEIPT_CAPACITY = 8_192;
export const WEB06_REDACTED_USERDB_TEXT = "<web06-redacted:userdb-text>";
export const WEB06_REDACTED_CUSTOMIZE_VALUE = "<web06-redacted:customize-value>";
export const WEB06_REDACTED_DICTIONARY_EXCLUDE = "web06-redacted:dictionary-exclude";
// This frozen control deliberately exercises customizeValue's existing,
// mode-neutral public validation error. Observation mode must never change the
// action's public result or error shape.
export const WEB06_INJECTED_ERROR_CONFIG_ID = "";
export const WEB06_INJECTED_ERROR_KEY = "";
export const WEB06_INJECTED_ERROR_VALUE = "web06-unused-error-control-value";

export function web06ExpectedActionFailure(name: keyof Actions, args: readonly unknown[]): boolean {
	return name === "customizeValue"
		&& args[0] === WEB06_INJECTED_ERROR_CONFIG_ID
		&& args[1] === WEB06_INJECTED_ERROR_KEY
		&& args[2] === WEB06_INJECTED_ERROR_VALUE;
}

export function web06EnqueueThenSignal<T>(
	enqueue: () => T,
	signal: () => boolean,
): { result: T; signalAccepted: boolean } {
	const result = enqueue();
	const signalAccepted = signal();
	return { result, signalAccepted };
}

export interface Web06KeyboardMapContext {
	capturedHasComposition: boolean;
	currentHasComposition: boolean;
	isAsciiMode: boolean;
	isInputFocused: boolean;
	visibleCandidateCount: number;
	pendingAsciiModeShift?: string;
	pendingAsciiModeShiftWasChorded: boolean;
	asciiModeToggleActions: Web06MappedAction[];
}

export function web06AsciiModeToggleActions(options: {
	nextAsciiMode: boolean;
	isFullShape: boolean;
	outputStandard: OutputStandard;
	activeSchema: RimeSchemaId;
	isExtendedCharset: boolean;
	isDisabled: boolean;
}): Web06MappedAction[] {
	return web06LiveOptionActions(options, "modifier-release");
}

export function web06LiveOptionActions(options: {
	nextAsciiMode: boolean;
	isFullShape: boolean;
	outputStandard: OutputStandard;
	activeSchema: RimeSchemaId;
	isExtendedCharset: boolean;
	isDisabled: boolean;
}, boundary: Web06BoundaryKind = "option"): Web06MappedAction[] {
	const activeOutputOption = outputOptionForStandard(options.outputStandard, options.activeSchema);
	return [
		["soft_cursor", true],
		["ascii_mode", options.nextAsciiMode],
		["full_shape", options.isFullShape],
		["traditionalization", false],
		...OUTPUT_STANDARD_ENGINE_OPTIONS.map(optionName => [
			optionName,
			optionName === activeOutputOption,
		] as const),
		["extended_charset", options.isExtendedCharset],
		["disabled", options.isDisabled],
	].map(([option, value]) => ({
		name: "setOption" as const,
		args: [option, value],
		actionClass: "stateful-barrier" as const,
		supersedable: false,
		boundary,
	}));
}

export interface Web06ActionContract {
	actionClass: Web06ActionClass;
	boundary: Web06BoundaryKind;
}

const ACTION_CONTRACTS = {
	setOption: { actionClass: "stateful-barrier", boundary: "option" },
	selectSchema: { actionClass: "stateful-barrier", boundary: "schema" },
	getUserdbSnapshot: { actionClass: "read-only", boundary: "none" },
	importUserdb: { actionClass: "stateful-barrier", boundary: "persistence" },
	processKey: { actionClass: "native-key", boundary: "none" },
	stageAi: { actionClass: "adapter-only", boundary: "none" },
	selectCandidate: { actionClass: "stateful-barrier", boundary: "selection" },
	deleteCandidate: { actionClass: "stateful-barrier", boundary: "selection" },
	flipPage: { actionClass: "stateful-barrier", boundary: "paging" },
	customize: { actionClass: "stateful-barrier", boundary: "option" },
	customizeValue: { actionClass: "stateful-barrier", boundary: "option" },
	deploy: { actionClass: "stateful-barrier", boundary: "deploy" },
	deployCacheSnapshot: { actionClass: "read-only", boundary: "none" },
	invalidateDeployCache: { actionClass: "stateful-barrier", boundary: "deploy" },
	injectedAssetsManifest: { actionClass: "read-only", boundary: "none" },
} as const satisfies Record<keyof Actions, Web06ActionContract>;

export function web06ActionContract(name: keyof Actions): Web06ActionContract {
	return ACTION_CONTRACTS[name];
}

export function web06ControlAction<K extends keyof Actions>(
	name: K,
	args: Parameters<Actions[K]>,
): Web06MappedAction {
	const contract = web06ActionContract(name);
	return {
		name,
		args,
		actionClass: contract.actionClass,
		supersedable: false,
		boundary: contract.boundary,
	};
}

export function web06DeferredControlAction<K extends keyof Actions>(
	name: K,
	placeholderArgs: Parameters<Actions[K]>,
): Web06MappedAction {
	return {
		...web06ControlAction(name, placeholderArgs),
		deferred: true,
	};
}

export function web06PrivateActionArgs(name: keyof Actions, args: readonly unknown[]): unknown[] {
	switch (name) {
		case "importUserdb":
			return [WEB06_REDACTED_USERDB_TEXT];
		case "customizeValue":
			return [args[0], args[1], WEB06_REDACTED_CUSTOMIZE_VALUE];
		case "customize":
			return [sanitizeCustomizePreferences(args[0])];
		default:
			return [...args];
	}
}

export function web06PrivateMappedAction(action: Web06MappedAction): Web06MappedAction {
	return {
		...action,
		args: web06PrivateActionArgs(action.name, action.args),
	};
}

export function web06RetainedActionArgs(
	mode: Web06CollectionMode,
	name: keyof Actions,
	args: readonly unknown[],
): unknown[] {
	return mode === "full" ? web06PrivateActionArgs(name, args) : [];
}

export function web06RetainedMappedAction(
	mode: Web06CollectionMode,
	action: Web06MappedAction,
): Web06MappedAction {
	return {
		...action,
		args: web06RetainedActionArgs(mode, action.name, action.args),
	};
}

export function web06RetainedActionIdentity(
	mode: Web06CollectionMode,
	identity: Web06ActionIdentity,
): Web06ActionIdentity {
	return mode === "full"
		? { ...identity, rawInputSequence: [...identity.rawInputSequence] }
		: { ...identity, rawInputSequence: [] };
}

export function web06RetainedEventIdentity(
	mode: Web06CollectionMode,
	identity: Web06DomEventIdentity,
): Web06DomEventIdentity {
	return mode === "full"
		? { ...identity }
		: { ...identity, key: "", code: "" };
}

function sanitizeCustomizePreferences(value: unknown): Record<string, unknown> {
	if (value === null || typeof value !== "object" || Array.isArray(value)) {
		return {};
	}
	const sanitized: Record<string, unknown> = {};
	for (const [key, preference] of Object.entries(value)) {
		if (key !== "dictionaryExclude") {
			sanitized[key] = preference;
			continue;
		}
		if (
			preference !== null
			&& typeof preference === "object"
			&& !Array.isArray(preference)
			&& (preference as Record<string, unknown>)["kind"] === WEB06_REDACTED_DICTIONARY_EXCLUDE
		) {
			sanitized[key] = preference;
			continue;
		}
		sanitized[key] = {
			kind: WEB06_REDACTED_DICTIONARY_EXCLUDE,
			count: Array.isArray(preference) ? preference.length : 0,
		};
	}
	return sanitized;
}

export function web06CollectionMode(search: string): Web06CollectionMode {
	const parameters = new URLSearchParams(search);
	const values = parameters.getAll(WEB06_MODE_QUERY);
	if (values.length > 1) {
		throw new Error(`Invalid ${WEB06_MODE_QUERY}: selector must occur at most once`);
	}
	const raw = values[0] ?? null;
	if (raw === null || raw === "minimal") {
		return "minimal";
	}
	if (raw === "full") {
		return "full";
	}
	throw new Error(`Invalid ${WEB06_MODE_QUERY}: ${raw}`);
}

export function web06CollectionModeProvenance(search: string): Web06CollectionModeProvenance {
	const mode = web06CollectionMode(search);
	if (mode === "full") return "instrumented-explicit-full";
	return new URLSearchParams(search).has(WEB06_MODE_QUERY)
		? "instrumented-explicit-minimal"
		: "instrumented-default-minimal";
}

export type Web06TerminalContract = {
	strategy: "presentation" | "listener" | "worker-effect" | "owner-effect";
	workerEffect?: "engine-state" | "engine-persistence" | "cache-invalidation" | "snapshot-read";
	ownerEffect?: "ui-userdb-refresh" | "ui-diagnostic-refresh" | "cache-invalidation";
	doubleRaf: boolean;
};

/**
 * Exhaustive private terminal ownership for every public worker action. The
 * contract deliberately lives beside the action map so an added action cannot
 * silently become outcome-less.
 */
export function web06TerminalContract(
	name: keyof Actions,
	args: readonly unknown[],
): Web06TerminalContract {
	switch (name) {
		case "processKey":
		case "stageAi":
		case "selectCandidate":
		case "deleteCandidate":
		case "flipPage":
			return { strategy: "presentation", doubleRaf: true };
		case "setOption":
			return args[0] === "yune_inspector"
				? { strategy: "worker-effect", workerEffect: "engine-state", doubleRaf: true }
				: { strategy: "listener", workerEffect: "engine-state", doubleRaf: true };
		case "selectSchema":
			return { strategy: "listener", workerEffect: "engine-state", doubleRaf: true };
		case "deploy":
			return { strategy: "listener", workerEffect: "engine-persistence", doubleRaf: true };
		case "customize":
		case "customizeValue":
			return name === "customize"
				? { strategy: "worker-effect", doubleRaf: true }
				: { strategy: "worker-effect", workerEffect: "engine-persistence", doubleRaf: true };
		case "getUserdbSnapshot":
			return {
				strategy: "owner-effect",
				workerEffect: "snapshot-read",
				ownerEffect: "ui-userdb-refresh",
				doubleRaf: true,
			};
		case "importUserdb":
			return {
				strategy: "owner-effect",
				workerEffect: "engine-persistence",
				ownerEffect: "ui-userdb-refresh",
				doubleRaf: true,
			};
		case "deployCacheSnapshot":
		case "injectedAssetsManifest":
			return {
				strategy: "owner-effect",
				workerEffect: "snapshot-read",
				ownerEffect: "ui-diagnostic-refresh",
				doubleRaf: true,
			};
		case "invalidateDeployCache":
			return {
				strategy: "owner-effect",
				workerEffect: "cache-invalidation",
				ownerEffect: "cache-invalidation",
				doubleRaf: true,
			};
	}
}

export function web06ReceiptCapacity(mode: Web06CollectionMode): number {
	switch (mode) {
		case "off":
			return 0;
		case "minimal":
			return WEB06_MINIMAL_RECEIPT_CAPACITY;
		case "full":
			return WEB06_FULL_RECEIPT_CAPACITY;
	}
}

export function web06TimestampsAreOrdered(
	...timestamps: readonly (number | undefined)[]
): boolean {
	const present = timestamps.filter((value): value is number => value !== undefined);
	return present.every((value, index) =>
		Number.isFinite(value) && (index === 0 || present[index - 1]! <= value)
	);
}

export class BoundedReceiptMap<T extends { identity: object }> {
	readonly #capacity: number;
	readonly #sequenceId: (receipt: T) => number;
	readonly #values = new Map<number, T>();
	#overflowed = false;

	constructor(
		capacity: number,
		sequenceId: (receipt: T) => number = receipt =>
			(receipt.identity as { sequenceId: number }).sequenceId,
	) {
		if (!Number.isSafeInteger(capacity) || capacity < 0) {
			throw new Error(`Invalid receipt capacity: ${capacity}`);
		}
		this.#capacity = capacity;
		this.#sequenceId = sequenceId;
	}

	get size(): number {
		return this.#values.size;
	}

	get(sequenceId: number): T | undefined {
		return this.#values.get(sequenceId);
	}

	set(receipt: T): boolean {
		if (this.#capacity === 0) {
			return false;
		}
		const sequenceId = this.#sequenceId(receipt);
		let firstOverflow = false;
		if (!this.#values.has(sequenceId) && this.#values.size >= this.#capacity) {
			firstOverflow = !this.#overflowed;
			this.#overflowed = true;
			const evicted = this.#values.keys().next().value as number | undefined;
			if (evicted !== undefined) {
				this.#values.delete(evicted);
			}
		}
		this.#values.set(sequenceId, receipt);
		return firstOverflow;
	}

	values(): T[] {
		return [...this.#values.values()];
	}

	clear(): void {
		this.#values.clear();
		this.#overflowed = false;
	}
}

export function web06UserdbSnapshotDigest(snapshot: YuneWebUserdbSnapshot): string {
	return web06StableDigest({
		schemaId: snapshot.schemaId,
		dictionaryId: snapshot.dictionaryId,
		exists: snapshot.exists,
		bytes: snapshot.bytes,
		rows: snapshot.rows.map(row => ({
			text: row.text,
			code: row.code,
			commits: row.commits,
			dee: row.dee,
			tick: row.tick,
		})),
		parseErrors: snapshot.parseErrors,
	});
}

export function web06UserdbOwnerState(snapshot: YuneWebUserdbSnapshot) {
	return {
		digest: web06UserdbSnapshotDigest(snapshot),
		schemaId: snapshot.schemaId,
		dictionaryId: snapshot.dictionaryId,
		exists: snapshot.exists,
		bytes: snapshot.bytes,
		rowCount: snapshot.rows.length,
		parseErrorCount: snapshot.parseErrors.length,
	};
}

export function web06DeployCacheSnapshotDigest(snapshot: YuneDeployCacheSnapshot): string {
	return web06StableDigest({
		schemaId: snapshot.schemaId,
		dictionaryId: snapshot.dictionaryId,
		cacheFresh: snapshot.cacheFresh,
		deployedSchemaExists: snapshot.deployedSchemaExists,
		actualStamp: snapshot.actualStamp,
		expectedStamp: snapshot.expectedStamp,
	});
}

export function web06DeployCacheOwnerState(snapshot: YuneDeployCacheSnapshot) {
	return {
		digest: web06DeployCacheSnapshotDigest(snapshot),
		schemaId: snapshot.schemaId,
		dictionaryId: snapshot.dictionaryId,
		cacheFresh: snapshot.cacheFresh,
		deployedSchemaExists: snapshot.deployedSchemaExists,
	};
}

export function web06InjectedAssetManifestDigest(manifest: YuneInjectedAssetManifest): string {
	return web06StableDigest(manifest);
}

export function web06InjectedAssetsOwnerState(manifest: YuneInjectedAssetManifest) {
	return {
		digest: web06InjectedAssetManifestDigest(manifest),
		schemaId: manifest.schemaId,
		assetCount: manifest.assets.length,
		totalBytes: manifest.assets.reduce((total, asset) => total + asset.bytes, 0),
	};
}

export function web06PresentationStateDigest(fingerprint: Web06PresentationFingerprint): string {
	return web06PresentationFingerprintDigest(fingerprint, false);
}

export function web06PresentationFingerprintDigest(
	fingerprint: Web06PresentationFingerprint,
	includeSequenceId = true,
): string {
	const state = web06HashState();
	web06HashToken(state, "web06-presentation-v1");
	if (includeSequenceId) web06HashNumber(state, fingerprint.sequenceId);
	web06HashString(state, fingerprint.input);
	web06HashNumber(state, fingerprint.page);
	web06HashBoolean(state, fingerprint.isLastPage);
	web06HashNumber(state, fingerprint.highlightedIndex);
	web06HashNumber(state, fingerprint.candidates.length);
	for (const candidate of fingerprint.candidates) {
		web06HashString(state, candidate.label);
		web06HashString(state, candidate.text);
		web06HashString(state, candidate.comment);
		web06HashString(state, candidate.source);
	}
	web06HashUnknown(state, fingerprint.status);
	web06HashString(state, fingerprint.textareaValue);
	web06HashNumber(state, fingerprint.selectionStart);
	web06HashNumber(state, fingerprint.selectionEnd);
	return web06HashHex(state);
}

export function web06PresentationFingerprintsEqual(
	left: Web06PresentationFingerprint,
	right: Web06PresentationFingerprint,
	includeSequenceId = true,
): boolean {
	return (!includeSequenceId || left.sequenceId === right.sequenceId)
		&& left.input === right.input
		&& left.page === right.page
		&& left.isLastPage === right.isLastPage
		&& left.highlightedIndex === right.highlightedIndex
		&& left.textareaValue === right.textareaValue
		&& left.selectionStart === right.selectionStart
		&& left.selectionEnd === right.selectionEnd
		&& left.candidates.length === right.candidates.length
		&& left.candidates.every((candidate, index) => {
			const observed = right.candidates[index];
			return observed !== undefined
				&& candidate.label === observed.label
				&& candidate.text === observed.text
				&& candidate.comment === observed.comment
				&& candidate.source === observed.source;
		})
		&& web06StructuredEqual(left.status, right.status);
}

export function web06AdapterProjectionFingerprint(result: RimeResult): Web06AdapterProjectionFingerprint {
	return {
		success: result.success,
		isComposing: result.isComposing,
		input: result.isComposing
			? result.inputBuffer.before + result.inputBuffer.active + result.inputBuffer.after
			: "",
		page: result.isComposing ? result.page : 0,
		isLastPage: result.isComposing ? result.isLastPage : true,
		highlightedIndex: result.isComposing ? result.highlightedIndex : -1,
		candidates: result.isComposing
			? result.candidates.map(candidate => ({
				...(candidate.label === undefined ? {} : { label: candidate.label }),
				text: candidate.text,
				...(candidate.comment === undefined ? {} : { comment: candidate.comment }),
				...(candidate.source === undefined ? {} : { source: candidate.source }),
			}))
			: [],
		...(result.committed === undefined ? {} : { committed: result.committed }),
		status: result.status ?? null,
	};
}

export function web06AdapterProjectionFingerprintsEqual(
	left: Web06AdapterProjectionFingerprint,
	right: Web06AdapterProjectionFingerprint,
): boolean {
	return web06StructuredEqual(left, right);
}

export function web06EngineRawFingerprint(
	action: keyof Actions,
	operation: string,
	json: string,
): Web06EngineRawFingerprint {
	const root = web06RawRecord(JSON.parse(json) as unknown, "response");
	const contextValue = root["context"];
	const statusValue = root["status"];
	return {
		action,
		operation,
		handled: web06RawBoolean(root["handled"], "handled"),
		commits: web06RawArray(root["commits"], "commits")
			.map((value, index) => web06RawString(value, `commits[${index}]`)),
		context: contextValue === null
			? null
			: web06EngineRawContext(contextValue),
		status: statusValue === null ? null : web06EngineRawStatus(statusValue),
	};
}

export function web06EngineRawAdapterProjection(
	raw: Web06EngineRawFingerprint,
): Web06AdapterProjectionFingerprint {
	const committed = raw.commits.length === 0 ? undefined : raw.commits.join("");
	if (!raw.handled) {
		return {
			success: false,
			isComposing: false,
			input: "",
			page: 0,
			isLastPage: true,
			highlightedIndex: -1,
			candidates: [],
			status: null,
		};
	}
	if (raw.context !== null && raw.context.preedit !== "") {
		return {
			success: true,
			isComposing: true,
			input: raw.context.preedit,
			page: raw.context.page,
			isLastPage: raw.context.isLastPage,
			highlightedIndex: raw.context.highlightedIndex,
			candidates: raw.context.candidates.map((candidate, index) => ({
				...(raw.context?.selectLabels[index] === undefined
					? {}
					: { label: raw.context.selectLabels[index] }),
				text: candidate.text,
				comment: candidate.comment,
				...(candidate.source === undefined ? {} : { source: candidate.source }),
			})),
			...(committed === undefined ? {} : { committed }),
			status: raw.status,
		};
	}
	return {
		success: true,
		isComposing: false,
		input: "",
		page: 0,
		isLastPage: true,
		highlightedIndex: -1,
		candidates: [],
		...(committed === undefined ? {} : { committed }),
		status: raw.status,
	};
}

export function web06RawResponseNotApplicableReason(
	action: keyof Actions,
	args: readonly unknown[],
): "action-has-no-runtime-response" | "adapter-short-circuit" | undefined {
	if (
		action !== "processKey"
		&& action !== "stageAi"
		&& action !== "selectCandidate"
		&& action !== "deleteCandidate"
		&& action !== "flipPage"
	) {
		return "action-has-no-runtime-response";
	}
	if (action !== "processKey") return undefined;
	const input = typeof args[0] === "string" ? args[0] : "";
	return /^\{Release\+/.test(input)
		|| /^\{(?:Alt|Control|Meta|Shift|Super)(?:_[LR])?\}$/.test(input)
		? "adapter-short-circuit"
		: undefined;
}

function web06EngineRawContext(value: unknown): NonNullable<Web06EngineRawFingerprint["context"]> {
	const context = web06RawRecord(value, "context");
	return {
		input: web06RawString(context["input"], "context.input"),
		preedit: web06RawString(context["preedit"], "context.preedit"),
		caret: web06RawNumber(context["caret"], "context.caret"),
		page: web06RawNumber(context["page_no"], "context.page_no"),
		pageSize: web06RawNumber(context["page_size"], "context.page_size"),
		isLastPage: web06RawBoolean(context["is_last_page"], "context.is_last_page"),
		highlightedIndex: web06RawNumber(context["highlighted"], "context.highlighted"),
		selectLabels: web06RawArray(context["select_labels"], "context.select_labels")
			.map((item, index) => web06RawString(item, `context.select_labels[${index}]`)),
		candidates: web06RawArray(context["candidates"], "context.candidates").map((item, index) => {
			const candidate = web06RawRecord(item, `context.candidates[${index}]`);
			return {
				text: web06RawString(candidate["text"], `context.candidates[${index}].text`),
				comment: web06RawString(candidate["comment"], `context.candidates[${index}].comment`),
				...(candidate["source"] === undefined || candidate["source"] === null
					? {}
					: { source: web06RawString(candidate["source"], `context.candidates[${index}].source`) }),
			};
		}),
	};
}

function web06EngineRawStatus(value: unknown): NonNullable<Web06EngineRawFingerprint["status"]> {
	const status = web06RawRecord(value, "status");
	return {
		schema_id: web06RawString(status["schema_id"], "status.schema_id"),
		schema_name: web06RawString(status["schema_name"], "status.schema_name"),
		is_disabled: web06RawBoolean(status["is_disabled"], "status.is_disabled"),
		is_composing: web06RawBoolean(status["is_composing"], "status.is_composing"),
		is_ascii_mode: web06RawBoolean(status["is_ascii_mode"], "status.is_ascii_mode"),
		is_full_shape: web06RawBoolean(status["is_full_shape"], "status.is_full_shape"),
		is_simplified: web06RawBoolean(status["is_simplified"], "status.is_simplified"),
		is_traditional: web06RawBoolean(status["is_traditional"], "status.is_traditional"),
		is_ascii_punct: web06RawBoolean(status["is_ascii_punct"], "status.is_ascii_punct"),
	};
}

function web06RawRecord(value: unknown, name: string): Record<string, unknown> {
	if (value === null || typeof value !== "object" || Array.isArray(value)) {
		throw new Error(`WEB-06 raw ${name} must be an object`);
	}
	return value as Record<string, unknown>;
}

function web06RawArray(value: unknown, name: string): unknown[] {
	if (!Array.isArray(value)) throw new Error(`WEB-06 raw ${name} must be an array`);
	return value;
}

function web06RawString(value: unknown, name: string): string {
	if (typeof value !== "string") throw new Error(`WEB-06 raw ${name} must be a string`);
	return value;
}

function web06RawNumber(value: unknown, name: string): number {
	if (typeof value !== "number" || !Number.isFinite(value)) {
		throw new Error(`WEB-06 raw ${name} must be a finite number`);
	}
	return value;
}

function web06RawBoolean(value: unknown, name: string): boolean {
	if (typeof value !== "boolean") throw new Error(`WEB-06 raw ${name} must be a boolean`);
	return value;
}

export function snapshotKeyboardEvent(event: KeyboardEvent): Web06DomEventSnapshot {
	return {
		type: event.type === "keyup" ? "keyup" : "keydown",
		code: event.code,
		key: event.key,
		timeStamp: event.timeStamp,
		repeat: event.repeat,
		ctrlKey: event.getModifierState("Control"),
		metaKey: event.getModifierState("Meta"),
		altKey: event.getModifierState("Alt"),
		shiftKey: event.getModifierState("Shift"),
	};
}

export function snapshotWeb06ControlEvent(event: Web06ControlEventLike): Web06DomEventSnapshot {
	const source = event.nativeEvent ?? event;
	if (
		source.type !== "click"
		&& source.type !== "change"
		&& source.type !== "mousedown"
		&& source.type !== "touchstart"
	) {
		throw new Error(`Unsupported WEB-06 control event type: ${source.type}`);
	}
	return {
		type: source.type,
		code: "",
		key: "",
		timeStamp: source.timeStamp,
		repeat: false,
		ctrlKey: source.ctrlKey ?? false,
		metaKey: source.metaKey ?? false,
		altKey: source.altKey ?? false,
		shiftKey: source.shiftKey ?? false,
	};
}

export function mapWeb06KeyboardEvent(
	event: Web06DomEventSnapshot,
	context: Web06KeyboardMapContext,
): Web06EventMapResult {
	return event.type === "keydown"
		? mapKeyDown(event, context)
		: mapKeyUp(event, context);
}

export function web06FocusLossEvent(timeStamp: number): Web06DomEventSnapshot {
	return {
		type: "blur",
		code: "",
		key: "",
		timeStamp,
		repeat: false,
		ctrlKey: false,
		metaKey: false,
		altKey: false,
		shiftKey: false,
	};
}

function mapKeyDown(
	event: Web06DomEventSnapshot,
	context: Web06KeyboardMapContext,
): Web06EventMapResult {
	const canToggleAsciiMode = context.isInputFocused || context.currentHasComposition;
	if (isAsciiModeShiftTap(event) && canToggleAsciiMode) {
		const shiftEffect = context.pendingAsciiModeShift !== undefined
			&& event.code !== context.pendingAsciiModeShift
			? "mark-chorded"
			: !event.repeat && context.pendingAsciiModeShift === undefined
			? "start"
			: undefined;
		return frontendConsumed("ascii-mode-shift-keydown", true, shiftEffect);
	}

	const digitIndex = candidateIndexFromDigit(event, context.visibleCandidateCount);
	if (digitIndex !== undefined) {
		return mapped(
			"composition-digit-selection",
			true,
			mappedAction("selectCandidate", [digitIndex], "selection"),
			context.pendingAsciiModeShift === undefined ? undefined : "mark-chorded",
		);
	}

	const rimeKey = parseRimeKey(event, context);
	if (rimeKey !== undefined) {
		const contract = processKeyContract(event, rimeKey, false);
		return mapped(
			contract.supersedable ? "printable-key" : `rime-key:${contract.boundary}`,
			true,
			{
				name: "processKey",
				args: [`{${rimeKey}}`],
				...contract,
			},
			context.pendingAsciiModeShift === undefined ? undefined : "mark-chorded",
		);
	}

	return browserPassThrough(
		context.pendingAsciiModeShift === undefined
			? "unmapped-keydown"
			: "unmapped-keydown-chords-pending-shift",
		context.pendingAsciiModeShift === undefined ? undefined : "mark-chorded",
	);
}

function mapKeyUp(
	event: Web06DomEventSnapshot,
	context: Web06KeyboardMapContext,
): Web06EventMapResult {
	const canToggleAsciiMode = context.isInputFocused || context.currentHasComposition;
	if (isAsciiModeShiftTap(event) && context.pendingAsciiModeShift === event.code) {
		if (!context.pendingAsciiModeShiftWasChorded && canToggleAsciiMode) {
			return {
				classification: "mapped-action(s)",
				reason: "ascii-mode-shift-tap",
				preventDefault: true,
				actions: context.asciiModeToggleActions,
				shiftEffect: "finish-toggle",
			};
		}
		return frontendConsumed("ascii-mode-shift-release-without-toggle", true, "finish-without-toggle");
	}

	if (/^[0-9]$/.test(event.key) && context.currentHasComposition) {
		return frontendConsumed(
			"composition-digit-keyup-follows-keydown",
			false,
			context.pendingAsciiModeShift === undefined ? undefined : "mark-chorded",
		);
	}

	if (context.capturedHasComposition && isModifierRelease(event)) {
		const rimeKey = parseRimeKey(event, context);
		if (rimeKey !== undefined) {
			return mapped(
				"modifier-release",
				false,
				{
					name: "processKey",
					args: [`{Release+${rimeKey}}`],
					actionClass: "stateful-barrier",
					supersedable: false,
					boundary: "modifier-release",
				},
				context.pendingAsciiModeShift === undefined ? undefined : "mark-chorded",
			);
		}
	}

	return browserPassThrough(
		"unmapped-keyup",
		context.pendingAsciiModeShift === undefined ? undefined : "mark-chorded",
	);
}

function mapped(
	reason: string,
	preventDefault: boolean,
	action: Web06MappedAction,
	shiftEffect?: Web06EventMapResult["shiftEffect"],
): Web06EventMapResult {
	return {
		classification: "mapped-action(s)",
		reason,
		preventDefault,
		actions: [action],
		shiftEffect,
	};
}

function frontendConsumed(
	reason: string,
	preventDefault: boolean,
	shiftEffect?: Web06EventMapResult["shiftEffect"],
): Web06EventMapResult {
	return {
		classification: "frontend-consumed",
		reason,
		preventDefault,
		actions: [],
		shiftEffect,
	};
}

function browserPassThrough(
	reason: string,
	shiftEffect?: Web06EventMapResult["shiftEffect"],
): Web06EventMapResult {
	return {
		classification: "browser-pass-through",
		reason,
		preventDefault: false,
		actions: [],
		shiftEffect,
	};
}

function mappedAction(
	name: keyof Actions,
	args: unknown[],
	boundary: Web06BoundaryKind,
): Web06MappedAction {
	return {
		name,
		args,
		actionClass: "stateful-barrier",
		supersedable: false,
		boundary,
	};
}

function candidateIndexFromDigit(
	event: Web06DomEventSnapshot,
	visibleCandidateCount: number,
): number | undefined {
	if (
		event.ctrlKey
		|| event.metaKey
		|| event.altKey
		|| event.shiftKey
		|| !/^[0-9]$/.test(event.key)
	) {
		return undefined;
	}
	const index = event.key === "0" ? 9 : Number(event.key) - 1;
	return index >= 0 && index < visibleCandidateCount ? index : undefined;
}

function parseRimeKey(
	event: Web06DomEventSnapshot,
	context: Web06KeyboardMapContext,
): string | undefined {
	if (!context.capturedHasComposition && context.isAsciiMode && context.isInputFocused) {
		return undefined;
	}
	const shouldMap = context.capturedHasComposition || (
		context.isInputFocused
		&& (
			(!event.ctrlKey && (isPrintableKey(event.key) || (!event.shiftKey && event.key === "F4")))
			|| event.key === "`"
		)
		&& !event.metaKey
		&& !event.altKey
	);
	if (!shouldMap || event.code.length === 0) {
		return undefined;
	}

	const match = /^(Control|Meta|Alt|Shift)(Left|Right)$/.exec(event.code);
	const isNumpadKey = event.code.startsWith("Numpad");
	const modifiers = new Set<string>();
	if (event.ctrlKey) modifiers.add("Control");
	if (event.metaKey) modifiers.add("Meta");
	if (event.altKey) modifiers.add("Alt");
	if (event.shiftKey) modifiers.add("Shift");
	if (match) {
		modifiers.delete(match[1]);
		modifiers.add(`${match[1]}_${match[2]?.[0] ?? ""}`);
	}
	else {
		let rimeKey = isNumpadKey ? event.code.slice(6) : event.key;
		rimeKey = RIME_KEY_MAP[rimeKey] || rimeKey;
		modifiers.add(isNumpadKey ? `KP_${rimeKey}` : rimeKey);
	}
	return [...modifiers].join("+");
}

function processKeyContract(
	event: Web06DomEventSnapshot,
	rimeKey: string,
	isRelease: boolean,
): Pick<Web06MappedAction, "actionClass" | "supersedable" | "boundary"> {
	if (isRelease || rimeKey.startsWith("Release+")) {
		return { actionClass: "stateful-barrier", supersedable: false, boundary: "modifier-release" };
	}
	if (/^(?:Control|Meta|Alt|Shift)(?:Left|Right)$/.test(event.code)) {
		return { actionClass: "stateful-barrier", supersedable: false, boundary: "modifier-release" };
	}
	if (event.key === "Backspace" || event.key === "Delete") {
		return { actionClass: "stateful-barrier", supersedable: false, boundary: "correction" };
	}
	if (/^(?:Arrow|Page)/.test(event.key)) {
		return { actionClass: "stateful-barrier", supersedable: false, boundary: "paging" };
	}
	if (event.key === "Escape") {
		return { actionClass: "stateful-barrier", supersedable: false, boundary: "cancel" };
	}
	if (
		event.key === "Enter"
		|| event.key === " "
		|| (isPrintableKey(event.key) && !/^[\p{L}\p{N}]$/u.test(event.key))
	) {
		return { actionClass: "stateful-barrier", supersedable: false, boundary: "commit" };
	}
	const supersedable = isPrintableKey(event.key)
		&& !event.ctrlKey
		&& !event.metaKey
		&& !event.altKey
		&& !rimeKey.includes("+");
	return {
		actionClass: supersedable ? "native-key" : "stateful-barrier",
		supersedable,
		boundary: supersedable ? "none" : "commit",
	};
}

function isAsciiModeShiftTap(event: Web06DomEventSnapshot): boolean {
	return event.key === "Shift"
		&& /^Shift(Left|Right)$/.test(event.code)
		&& !event.ctrlKey
		&& !event.metaKey
		&& !event.altKey;
}

function isModifierRelease(event: Web06DomEventSnapshot): boolean {
	return /^(Control|Meta|Alt|Shift)(Left|Right)$/.test(event.code);
}

function isPrintableKey(key: string): boolean {
	// Keep the measurement classifier byte-for-byte aligned with the production
	// CandidatePanel `isPrintable` gate. Non-ASCII text input is browser-owned
	// while no RIME composition is active; the protocol must not invent an
	// engine action for it.
	return key.length === 1 && key >= " " && key <= "~";
}

export function web06StableDigest(value: unknown): string {
	const state = web06HashState();
	web06HashUnknown(state, value);
	return web06HashHex(state);
}

export function web06ValuesEqual(left: unknown, right: unknown): boolean {
	return web06StructuredEqual(left, right);
}

export function web06ActionIdentitiesEqual(
	left: Web06ActionIdentity,
	right: Web06ActionIdentity,
): boolean {
	return left.protocolVersion === right.protocolVersion
		&& left.actionId === right.actionId
		&& left.sequenceId === right.sequenceId
		&& left.eventId === right.eventId
		&& left.eventSequenceId === right.eventSequenceId
		&& left.eventActionIndex === right.eventActionIndex
		&& left.compositionEpochId === right.compositionEpochId
		&& left.supersessionSubRunId === right.supersessionSubRunId
		&& left.actionClass === right.actionClass
		&& left.supersedable === right.supersedable
		&& left.boundary === right.boundary
		&& left.originKind === right.originKind
		&& left.originReason === right.originReason
		&& left.originOwner === right.originOwner
		&& left.causedByActionId === right.causedByActionId
		&& left.causedBySequenceId === right.causedBySequenceId
		&& left.causedByEventId === right.causedByEventId
		&& left.causedByEventSequenceId === right.causedByEventSequenceId
		&& left.actionEnqueuedAt === right.actionEnqueuedAt
		&& left.mainQueueDepthAtEnqueue === right.mainQueueDepthAtEnqueue
		&& left.workerSentAt === right.workerSentAt
		&& left.workerDispatchDepth === right.workerDispatchDepth
		&& left.rawInputSequence.length === right.rawInputSequence.length
		&& left.rawInputSequence.every((value, index) => value === right.rawInputSequence[index]);
}

type Web06HashState = [number, number, number, number];

function web06HashState(): Web06HashState {
	return [0x811c9dc5, 0x9e3779b9, 0x85ebca6b, 0xc2b2ae35];
}

function web06HashToken(state: Web06HashState, token: string): void {
	for (let index = 0; index < token.length; index += 1) {
		const unit = token.charCodeAt(index);
		state[0] = Math.imul(state[0] ^ unit, 0x01000193);
		state[1] = Math.imul(state[1] ^ unit, 0x27d4eb2d);
		state[2] = Math.imul(state[2] ^ unit, 0x165667b1);
		state[3] = Math.imul(state[3] ^ unit, 0x85ebca77);
	}
}

function web06HashString(state: Web06HashState, value: string): void {
	web06HashToken(state, `s${value.length}:`);
	web06HashToken(state, value);
}

function web06HashNumber(state: Web06HashState, value: number): void {
	web06HashToken(state, `n${Object.is(value, -0) ? "-0" : String(value)};`);
}

function web06HashBoolean(state: Web06HashState, value: boolean): void {
	web06HashToken(state, value ? "b1;" : "b0;");
}

function web06HashUnknown(state: Web06HashState, value: unknown): void {
	if (value === null) {
		web06HashToken(state, "null;");
		return;
	}
	switch (typeof value) {
		case "undefined":
			web06HashToken(state, "undefined;");
			return;
		case "string":
			web06HashString(state, value);
			return;
		case "number":
			web06HashNumber(state, value);
			return;
		case "boolean":
			web06HashBoolean(state, value);
			return;
		case "bigint":
			web06HashToken(state, `i${String(value)};`);
			return;
		case "object":
			if (Array.isArray(value)) {
				web06HashToken(state, `a${value.length}:`);
				for (const item of value) web06HashUnknown(state, item);
				return;
			}
			{
				const record = value as Record<string, unknown>;
				const keys = Object.keys(record).sort();
				web06HashToken(state, `o${keys.length}:`);
				for (const key of keys) {
					web06HashString(state, key);
					web06HashUnknown(state, record[key]);
				}
				return;
			}
		default:
			web06HashToken(state, `${typeof value};`);
	}
}

function web06HashHex(state: Web06HashState): string {
	return state.map(hash => (hash >>> 0).toString(16).padStart(8, "0")).join("");
}

function web06StructuredEqual(left: unknown, right: unknown): boolean {
	if (Object.is(left, right)) return true;
	if (left === null || right === null || typeof left !== "object" || typeof right !== "object") {
		return false;
	}
	if (Array.isArray(left) || Array.isArray(right)) {
		return Array.isArray(left)
			&& Array.isArray(right)
			&& left.length === right.length
			&& left.every((value, index) => web06StructuredEqual(value, right[index]));
	}
	const leftRecord = left as Record<string, unknown>;
	const rightRecord = right as Record<string, unknown>;
	const leftKeys = Object.keys(leftRecord);
	const rightKeys = Object.keys(rightRecord);
	return leftKeys.length === rightKeys.length
		&& leftKeys.every(key => Object.hasOwn(rightRecord, key)
			&& web06StructuredEqual(leftRecord[key], rightRecord[key]));
}

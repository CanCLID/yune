import { OUTPUT_STANDARD_ENGINE_OPTIONS, RIME_KEY_MAP, outputOptionForStandard } from "../consts";

import type {
	Actions,
	Web06ActionIdentity,
	RimeSchemaId,
	Web06ActionClass,
	Web06BoundaryKind,
	Web06CollectionMode,
	Web06ControlEventLike,
	Web06DomEventSnapshot,
	Web06EventMapResult,
	Web06MappedAction,
} from "../types";
import type { OutputStandard } from "../consts";

export const WEB06_MODE_QUERY = "yuneWeb06Mode";
export const WEB06_IMPORT_CONTINUATION_EVENT = "yune-web06-import-enqueued";
export const WEB06_MINIMAL_RECEIPT_CAPACITY = 2_048;
export const WEB06_FULL_RECEIPT_CAPACITY = 8_192;
export const WEB06_REDACTED_USERDB_TEXT = "<web06-redacted:userdb-text>";
export const WEB06_REDACTED_CUSTOMIZE_VALUE = "<web06-redacted:customize-value>";
export const WEB06_REDACTED_DICTIONARY_EXCLUDE = "web06-redacted:dictionary-exclude";

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
		deferred: true,
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
	const raw = new URLSearchParams(search).get(WEB06_MODE_QUERY);
	if (raw === null || raw === "minimal") {
		return "minimal";
	}
	if (raw === "off" || raw === "full") {
		return raw;
	}
	throw new Error(`Invalid ${WEB06_MODE_QUERY}: ${raw}`);
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

export class BoundedReceiptMap<T extends { identity: { sequenceId: number } }> {
	readonly #capacity: number;
	readonly #values = new Map<number, T>();
	readonly #order: number[] = [];

	constructor(capacity: number) {
		if (!Number.isSafeInteger(capacity) || capacity < 0) {
			throw new Error(`Invalid receipt capacity: ${capacity}`);
		}
		this.#capacity = capacity;
	}

	get(sequenceId: number): T | undefined {
		return this.#values.get(sequenceId);
	}

	set(receipt: T): boolean {
		if (this.#capacity === 0) {
			return false;
		}
		const sequenceId = receipt.identity.sequenceId;
		let overflowed = false;
		if (!this.#values.has(sequenceId)) {
			this.#order.push(sequenceId);
		}
		this.#values.set(sequenceId, receipt);
		while (this.#order.length > this.#capacity) {
			overflowed = true;
			const evicted = this.#order.shift();
			if (evicted !== undefined) {
				this.#values.delete(evicted);
			}
		}
		return overflowed;
	}

	values(): T[] {
		return this.#order.flatMap(sequenceId => {
			const value = this.#values.get(sequenceId);
			return value === undefined ? [] : [value];
		});
	}

	clear(): void {
		this.#values.clear();
		this.#order.length = 0;
	}
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
	return [...key].length === 1;
}

export function web06StableDigest(value: unknown): string {
	const text = stableStringify(value);
	let hash = 0x811c9dc5;
	for (let index = 0; index < text.length; index += 1) {
		hash ^= text.charCodeAt(index);
		hash = Math.imul(hash, 0x01000193);
	}
	return (hash >>> 0).toString(16).padStart(8, "0");
}

export function web06ActionIdentitiesEqual(
	left: Web06ActionIdentity,
	right: Web06ActionIdentity,
): boolean {
	return stableStringify(left) === stableStringify(right);
}

function stableStringify(value: unknown): string {
	if (value === null || typeof value !== "object") {
		return JSON.stringify(value) ?? String(value);
	}
	if (Array.isArray(value)) {
		return `[${value.map(stableStringify).join(",")}]`;
	}
	const record = value as Record<string, unknown>;
	return `{${Object.keys(record)
		.sort()
		.map(key => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
		.join(",")}}`;
}

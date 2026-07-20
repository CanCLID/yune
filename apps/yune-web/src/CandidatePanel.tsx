import { useState, useEffect, useRef, useCallback, useLayoutEffect } from "react";

import Candidate from "./Candidate";
import CandidateInfo from "./CandidateInfo";
import CaretFollower from "./CaretFollower";
import { RIME_KEY_MAP } from "./consts";
import DictionaryPanel from "./DictionaryPanel";
import Rime, {
	canWeb06Supersede,
	declareWeb06ControlFanout,
	invalidateWeb06Measurement,
	recordWeb06DomEvent,
	recordWeb06PresentationOutcome,
	recordWeb06ResponseMapping,
	registerWeb06EventFanout,
	web06ActionIdentityFor,
	withWeb06OwnedAction,
	withWeb06ActionContext,
	withWeb06ControlEvent,
} from "./rime";
import { notify } from "./toast";
import { isPrintable } from "./utils";
import {
	mapWeb06KeyboardEvent,
	snapshotKeyboardEvent,
	web06AsciiModeToggleActions,
	web06FocusLossEvent,
	web06StableDigest,
	web06ControlAction,
} from "./yune-integration/private-protocol";
import {
	WEB06_ACTION_OWNER,
	web06SingleActionFanout,
} from "./yune-integration/web06-app-action-map";

import type {
	InputState,
	Preferences,
	RimeResult,
	Web06ActionContext,
	Web06ActionIdentity,
	Web06BoundaryKind,
	Web06DomEventIdentity,
	Web06MappedAction,
	Web06PresentationFingerprint,
	YuneInspectorDebug,
	YuneStatusSnapshot,
} from "./types";
import type { MouseEvent as ReactMouseEvent, TouchEvent as ReactTouchEvent } from "react";

interface ActionDiagnosticSnapshot {
	action?: string;
	input?: string;
	enqueuedAt?: number;
	sentAt?: number;
	receivedAt?: number;
	workerStartedAt?: number;
	workerFinishedAt?: number;
	queueWaitMs?: number;
	workerRoundtripMs?: number;
	workerMs?: number;
	workerBaseElapsedMs?: number;
	workerAmplificationMs?: number;
	workerActionMultiplier?: number;
	totalMs?: number;
}

interface PendingPerfDiagnostic {
	input: string;
	key?: string;
	keydownAt: number;
	workerQueuedAt: number;
	workerSentAt: number;
	workerStartedAt: number;
	workerFinishedAt: number;
	responseReceivedAt: number;
	responseMappingStartedAt: number;
	responseMappingFinishedAt: number;
	workerQueueWaitMs?: number;
	workerProcessMs?: number;
	workerRoundtripMs?: number;
	workerBaseElapsedMs?: number;
	workerAmplificationMs?: number;
	workerActionMultiplier?: number;
	responseMappingMs: number;
	totalWorkerActionMs?: number;
	wasmHeapBytes?: number;
	peakWasmHeapBytes?: number;
	candidateCount: number;
	totalCandidateCount: number;
	firstCandidateText?: string;
}

interface MetricUpdate {
	lookupMs?: number;
	aiMs?: number;
	wasmHeapBytes?: number;
	peakWasmHeapBytes?: number;
	candidateCount?: number;
	totalCandidateCount?: number;
	latestInput?: string;
}

interface PendingWeb06Presentation {
	identity: Web06ActionIdentity;
	stateUpdateScheduledAt: number;
	presentationExpected: Web06PresentationFingerprint;
	beforePresentationDigest: string;
	committed: boolean;
	resolved: boolean;
	stateCommittedAt?: number;
	firstRafAt?: number;
}

function nowMs() {
	return performance.timeOrigin + performance.now();
}

function web06Now() {
	return performance.now();
}

function renderedInput(state: InputState | undefined) {
	return state === undefined
		? undefined
		: state.inputBuffer.before + state.inputBuffer.active + state.inputBuffer.after;
}

function appendTypingDiagnostic(diagnostic: {
	action: "processKey";
	input?: string;
	totalMs: number;
	wasmHeapBytes?: number;
	peakWasmHeapBytes?: number;
}) {
	const existing = document.documentElement.dataset["yuneTypingDiagnostics"];
	const diagnostics = existing ? JSON.parse(existing) as typeof diagnostic[] : [];
	diagnostics.push(diagnostic);
	document.documentElement.dataset["yuneTypingDiagnostics"] = JSON.stringify(diagnostics.slice(-100));
}

function readLatestProcessKeyActionDiagnostic(input: string): ActionDiagnosticSnapshot | undefined {
	const existing = document.documentElement.dataset["yuneActionDiagnostics"];
	const diagnostics = existing ? JSON.parse(existing) as ActionDiagnosticSnapshot[] : [];
	return diagnostics
		.slice()
		.reverse()
		.find(diagnostic => diagnostic.action === "processKey" && diagnostic.input === input);
}

function appendPerfDiagnostic(diagnostic: PendingPerfDiagnostic & {
	renderedInput: string;
	renderRevision: number;
	stateAppliedAt: number;
	paintObservedAt: number;
	reactUpdateMs: number;
	paintProxyMs: number;
	totalKeydownToPaintMs: number;
}) {
	const existing = document.documentElement.dataset["yunePerfDiagnostics"];
	const diagnostics = existing ? JSON.parse(existing) as typeof diagnostic[] : [];
	diagnostics.push(diagnostic);
	document.documentElement.dataset["yunePerfDiagnostics"] = JSON.stringify(diagnostics.slice(-100));
}

function presentationFingerprint(
	state: InputState | undefined,
	prefs: Preferences,
	textArea: HTMLTextAreaElement,
	status: YuneStatusSnapshot | undefined,
): Web06PresentationFingerprint {
	return {
		input: renderedInput(state) ?? "",
		page: state?.page ?? 0,
		isLastPage: state?.isLastPage ?? true,
		highlightedIndex: state?.highlightedIndex ?? -1,
		candidates: (state?.candidates.slice(0, prefs.pageSize) ?? []).map(candidate => ({
			label: candidate.label,
			text: candidate.text,
			comment: visibleCandidateComment(candidate, prefs),
			source: candidate.source ?? "",
		})),
		status: statusFingerprint(status),
		textareaValue: textArea.value,
		selectionStart: textArea.selectionStart,
		selectionEnd: textArea.selectionEnd,
	};
}

function visibleCandidateComment(candidate: CandidateInfo, prefs: Preferences): string {
	const labels = candidate.matchedEntries?.flatMap(entry => entry.formattedLabels(prefs) ?? []) ?? [];
	const detailText = [candidate.note, ...labels.slice(0, 2)].filter(Boolean).join(" ");
	const definitions = candidate.inlineDefinitions(prefs);
	return definitions === undefined
		? detailText
		: `${definitions.map(([, , value]) => value).join("")}${detailText}`;
}

function statusFingerprint(status: YuneStatusSnapshot | undefined): Record<string, unknown> | null {
	return status === undefined
		? null
		: {
			schema_id: status.schema_id,
			schema_name: status.schema_name,
			is_disabled: status.is_disabled,
			is_composing: status.is_composing,
			is_ascii_mode: status.is_ascii_mode,
			is_full_shape: status.is_full_shape,
			is_simplified: status.is_simplified,
			is_traditional: status.is_traditional,
			is_ascii_punct: status.is_ascii_punct,
		};
}

function observedPresentationFingerprint(textArea: HTMLTextAreaElement): Web06PresentationFingerprint {
	const panel = document.querySelector<HTMLElement>(".candidate-panel");
	const marker = parseWeb06StateMarker(panel?.dataset["yuneWeb06State"]);
	const candidates = [...document.querySelectorAll<HTMLElement>(".candidate-panel .candidate-row")].map(row => ({
		label: normalizedText(row.querySelector(".candidate-index")),
		text: normalizedText(row.querySelector(".candidate-text")),
		comment: normalizedText(row.querySelector(".candidate-note")),
		source: row.dataset["source"] ?? "",
	}));
	return {
		input: normalizedText(panel?.querySelector(".candidate-preedit")),
		page: marker.page,
		isLastPage: marker.isLastPage,
		highlightedIndex: marker.highlightedIndex,
		candidates,
		status: observedStatusFingerprint(),
		textareaValue: textArea.value,
		selectionStart: textArea.selectionStart,
		selectionEnd: textArea.selectionEnd,
	};
}

function normalizedText(element: Element | null | undefined): string {
	return element?.textContent?.trim() ?? "";
}

function parseWeb06StateMarker(value: string | undefined): {
	page: number;
	isLastPage: boolean;
	highlightedIndex: number;
} {
	if (value === undefined) {
		return { page: 0, isLastPage: true, highlightedIndex: -1 };
	}
	const [version, , page, isLastPage, highlightedIndex] = value.split("|");
	if (version !== "web06-private-v1") {
		return { page: Number.NaN, isLastPage: false, highlightedIndex: Number.NaN };
	}
	return {
		page: Number(page),
		isLastPage: isLastPage === "1",
		highlightedIndex: Number(highlightedIndex),
	};
}

function observedStatusFingerprint(): Record<string, unknown> | null {
	const strip = document.querySelector<HTMLElement>("[data-yune-status]");
	if (strip === null) return null;
	const schema = strip.querySelector<HTMLElement>("[data-yune-status-schema]");
	return {
		schema_id: schema?.dataset["yuneStatusSchemaId"] ?? "",
		schema_name: schema?.dataset["yuneStatusSchemaName"] ?? "",
		is_disabled: booleanDataset(strip, "[data-yune-status-disabled]", "yuneStatusDisabled"),
		is_composing: booleanDataset(strip, "[data-yune-status-composing]", "yuneStatusComposing"),
		is_ascii_mode: booleanDataset(strip, "[data-yune-status-ascii]", "yuneStatusAscii"),
		is_full_shape: booleanDataset(strip, "[data-yune-status-full-shape]", "yuneStatusFullShape"),
		is_simplified: booleanDataset(strip, "[data-yune-status-simplified]", "yuneStatusSimplified"),
		is_traditional: booleanDataset(strip, "[data-yune-status-traditional]", "yuneStatusTraditional"),
		is_ascii_punct: booleanDataset(strip, "[data-yune-status-ascii-punct]", "yuneStatusAsciiPunct"),
	};
}

function booleanDataset(root: HTMLElement, selector: string, key: string): boolean {
	return root.querySelector<HTMLElement>(selector)?.dataset[key] === "true";
}

export default function CandidatePanel({
	textArea,
	prefs,
	deployStatus,
	aiStatus,
	onInspectorDebug,
	onInspectorCandidates,
	onStatus,
	onUserdbChange,
	onMetrics,
	onToggleAsciiMode,
}: {
	runAsyncTask(asyncTask: () => Promise<void>): void;
	textArea: HTMLTextAreaElement;
	prefs: Preferences;
	deployStatus: number;
	aiStatus: number;
	onInspectorDebug?(debug: YuneInspectorDebug | undefined): void;
	onInspectorCandidates?(candidates: Extract<RimeResult, { isComposing: true }>["candidates"]): void;
	onStatus?(status: YuneStatusSnapshot | undefined): void;
	onUserdbChange?(identity?: Web06ActionIdentity): void;
	onMetrics?(metrics: MetricUpdate): void;
	onToggleAsciiMode(): void;
}) {
	const [inputState, setInputState] = useState<InputState | undefined>();
	const inputStateRef = useRef<InputState | undefined>();
	const [showDictionaryIndex, setShowDictionaryIndex] = useState<number | undefined>();
	const candidateList = useRef<HTMLTableElement>(null);
	const dictionaryPanel = useRef<HTMLDivElement>(null);
	const pendingPerfDiagnostics = useRef<PendingPerfDiagnostic[]>([]);
	const pendingWeb06Presentations = useRef<PendingWeb06Presentation[]>([]);
	const [web06Render, setWeb06Render] = useState<PendingWeb06Presentation | undefined>();
	const committedRender = useRef<{ input: string | undefined; revision: number }>({ input: undefined, revision: 0 });
	const web06RenderRevision = useRef(0);
	const web06CompositionEpochId = useRef(1);
	const web06SupersessionSubRunId = useRef(1);
	const web06RawInputSequence = useRef<string[]>([]);
	const lastClassicStateRef = useRef<InputState | undefined>();
	const lastClassicActionIdentity = useRef<Web06ActionIdentity | undefined>();
	const pendingAsciiModeShift = useRef<string | undefined>();
	const pendingAsciiModeShiftWasChorded = useRef(false);

	const hideDictionary = useCallback(() => {
		setShowDictionaryIndex(undefined);
	}, [setShowDictionaryIndex]);

	const insert = useCallback((newText: string) => {
		const { selectionStart, selectionEnd } = textArea;
		textArea.value = textArea.value.slice(0, selectionStart) + newText + textArea.value.slice(selectionEnd);
		textArea.selectionStart = textArea.selectionEnd = selectionStart + newText.length;
	}, [textArea]);

	function advanceWeb06Boundary(boundary: Web06BoundaryKind) {
		if (boundary === "none") return;
		web06SupersessionSubRunId.current += 1;
		web06RawInputSequence.current = [];
		if (
			boundary === "commit"
			|| boundary === "cancel"
			|| boundary === "focus-loss"
			|| boundary === "schema"
			|| boundary === "option"
			|| boundary === "deploy"
			|| boundary === "persistence"
			|| boundary === "error"
		) {
			web06CompositionEpochId.current += 1;
		}
	}

	const handleRimeResult = useCallback((promise: Promise<RimeResult>, key?: string, keydownContext?: { input: string; key?: string; keydownAt: number }, metricKind?: "lookup" | "ai") => {
		const startedAt = performance.now();
		const web06Identity = web06ActionIdentityFor(promise);
		const beforePresentationDigest = web06StableDigest(observedPresentationFingerprint(textArea));
		if (web06Identity === undefined) {
			invalidateWeb06Measurement("MISSING_ACTION_IDENTITY", "RimeResult promise has no WEB-06 action identity");
		}
		void (async () => {
			let type: "warning" | "error" | undefined;
			try {
				const result = await promise;
				const responseReceivedAt = nowMs();
				const responseMappingStartedAt = web06Now();
				onInspectorDebug?.(result.isComposing ? result.debug : undefined);
				onInspectorCandidates?.(result.isComposing ? result.candidates : []);
				onStatus?.(result.status);
				if (!result.success) {
					type = "warning";
				}
				const state = result.isComposing
					? {
						inputBuffer: result.inputBuffer,
						page: result.page,
						isLastPage: result.isLastPage,
						highlightedIndex: result.highlightedIndex,
						candidates: result.candidates.map(
							({ label, text, comment, source }, i) => new CandidateInfo(label || `${(i + 1) % 10}.`, text, comment, source),
						),
						isPrevDisabled: !result.page,
						isNextDisabled: result.isLastPage,
					}
					: inputStateRef.current;
				const responseMappingFinishedAt = web06Now();
				if (web06Identity !== undefined) {
					recordWeb06ResponseMapping(
						web06Identity,
						responseMappingStartedAt,
						responseMappingFinishedAt,
					);
				}
				if (result.committed) {
					insert(result.committed);
					onUserdbChange?.(web06Identity);
				}
				else if (!state && key && isPrintable(key)) {
					insert(key);
				}
				const candidateCount = result.isComposing ? Math.min(result.candidates.length, prefs.pageSize) : undefined;
				const totalCandidateCount = result.isComposing ? result.candidates.length : undefined;
				if (keydownContext) {
					const actionDiagnostic = readLatestProcessKeyActionDiagnostic(keydownContext.input);
					const committedInput = renderedInput(result.isComposing ? state : undefined) ?? keydownContext.input;
					pendingPerfDiagnostics.current.push({
						input: committedInput,
						key: keydownContext.key,
						keydownAt: keydownContext.keydownAt,
						workerQueuedAt: actionDiagnostic?.enqueuedAt ?? keydownContext.keydownAt,
						workerSentAt: actionDiagnostic?.sentAt ?? responseReceivedAt,
						workerStartedAt: actionDiagnostic?.workerStartedAt ?? actionDiagnostic?.sentAt ?? responseReceivedAt,
						workerFinishedAt: actionDiagnostic?.workerFinishedAt ?? responseReceivedAt,
						responseReceivedAt: actionDiagnostic?.receivedAt ?? responseReceivedAt,
						responseMappingStartedAt,
						responseMappingFinishedAt,
						workerQueueWaitMs: actionDiagnostic?.queueWaitMs,
						workerProcessMs: actionDiagnostic?.workerMs,
						workerRoundtripMs: actionDiagnostic?.workerRoundtripMs,
						workerBaseElapsedMs: actionDiagnostic?.workerBaseElapsedMs,
						workerAmplificationMs: actionDiagnostic?.workerAmplificationMs,
						workerActionMultiplier: actionDiagnostic?.workerActionMultiplier,
						responseMappingMs: Math.round(responseMappingFinishedAt - responseMappingStartedAt),
						totalWorkerActionMs: actionDiagnostic?.totalMs,
						wasmHeapBytes: result.memory?.wasmHeapBytes,
						peakWasmHeapBytes: result.memory?.peakWasmHeapBytes,
						candidateCount: candidateCount ?? 0,
						totalCandidateCount: totalCandidateCount ?? 0,
						firstCandidateText: result.isComposing ? result.candidates[0]?.text : undefined,
					});
				}
				if (metricKind !== "ai") {
					lastClassicStateRef.current = result.isComposing ? state : undefined;
				}
				onMetrics?.({
					latestInput: result.isComposing ? result.inputBuffer.active || result.inputBuffer.before : undefined,
					candidateCount,
					totalCandidateCount,
					...(result.memory ? {
						wasmHeapBytes: result.memory.wasmHeapBytes,
						peakWasmHeapBytes: result.memory.peakWasmHeapBytes,
					} : {}),
					...(metricKind === "lookup" ? { lookupMs: Math.round(performance.now() - startedAt) } : {}),
					...(metricKind === "ai" ? { aiMs: Math.round(performance.now() - startedAt) } : {}),
				});
				if (web06Identity !== undefined) {
					const presentationExpected = presentationFingerprint(
						result.isComposing ? state : undefined,
						prefs,
						textArea,
						result.status,
					);
					const pending: PendingWeb06Presentation = {
						identity: web06Identity,
						stateUpdateScheduledAt: Number.NaN,
						presentationExpected,
						beforePresentationDigest,
						committed: result.committed !== undefined,
						resolved: false,
					};
					pendingWeb06Presentations.current.push(pending);
					pending.stateUpdateScheduledAt = web06Now();
					setWeb06Render(pending);
				}
				setInputState(result.isComposing ? state : undefined);
				if (result.committed !== undefined && web06Identity?.boundary !== "commit") {
					advanceWeb06Boundary("commit");
				}
				requestAnimationFrame(() => {
					appendTypingDiagnostic({
						action: "processKey",
						input: result.isComposing ? result.inputBuffer.active || result.inputBuffer.before : undefined,
						totalMs: Math.round(performance.now() - startedAt),
						wasmHeapBytes: result.memory?.wasmHeapBytes,
						peakWasmHeapBytes: result.memory?.peakWasmHeapBytes,
					});
				});
				hideDictionary();
			}
			catch (error) {
				type = "error";
				advanceWeb06Boundary("error");
				invalidateWeb06Measurement(
					"ACTION_RESULT_ERROR",
					error instanceof Error ? `${error.name}: ${error.message}` : String(error),
					web06Identity,
				);
			}
			if (type) {
				notify(type, "執行操作", "performing the operation");
			}
			textArea.focus();
		})();
	}, [hideDictionary, insert, onInspectorCandidates, onInspectorDebug, onMetrics, onStatus, onUserdbChange, prefs, textArea]);

	const processKey = useCallback((input: string, key?: string, keydownAt?: number, context?: Web06ActionContext) => {
		const classicResult = context === undefined
			? Rime.processKey(input)
			: withWeb06ActionContext(context, () => Rime.processKey(input));
		const classicIdentity = web06ActionIdentityFor(classicResult);
		lastClassicActionIdentity.current = classicIdentity;
		handleRimeResult(classicResult, key, keydownAt === undefined ? undefined : { input, key, keydownAt }, "lookup");
		if (prefs.enableAI) {
			void classicResult.then(result => {
				if (result.isComposing) {
					handleRimeResult(withWeb06OwnedAction(
						"stage-ai-background",
						"stageAi",
						[],
						"stage-ai-after-process-key",
						classicIdentity,
						() => Rime.stageAi(),
					), undefined, undefined, "ai");
				}
			}).catch(() => undefined);
		}
	}, [handleRimeResult, prefs.enableAI]);
	const flipPage = useCallback((backward: boolean, context?: Web06ActionContext) => {
		const promise = context === undefined
			? Rime.flipPage(backward)
			: withWeb06ActionContext(context, () => Rime.flipPage(backward));
		handleRimeResult(promise);
	}, [handleRimeResult]);
	const selectCandidate = useCallback((index: number, context?: Web06ActionContext) => {
		const promise = context === undefined
			? Rime.selectCandidate(index)
			: withWeb06ActionContext(context, () => Rime.selectCandidate(index));
		handleRimeResult(promise);
	}, [handleRimeResult]);
	const deleteCandidate = useCallback((index: number, context?: Web06ActionContext) => {
		const promise = context === undefined
			? Rime.deleteCandidate(index)
			: withWeb06ActionContext(context, () => Rime.deleteCandidate(index));
		handleRimeResult(promise);
	}, [handleRimeResult]);
	const flipPageFromControl = useCallback((event: ReactMouseEvent, backward: boolean) => {
		withWeb06ControlEvent(event, () => {
			declareWeb06ControlFanout(
				"candidate-page-button",
				web06SingleActionFanout(
					WEB06_ACTION_OWNER.control,
					web06ControlAction("flipPage", [backward]),
				),
			);
			const promise = withWeb06OwnedAction(
				WEB06_ACTION_OWNER.control,
				"flipPage",
				[backward],
				"candidate-page-button",
				undefined,
				() => Rime.flipPage(backward),
			);
			handleRimeResult(promise);
		});
	}, [handleRimeResult]);
	const selectCandidateFromControl = useCallback((event: ReactMouseEvent, index: number) => {
		withWeb06ControlEvent(event, () => {
			declareWeb06ControlFanout(
				"candidate-click-selection",
				web06SingleActionFanout(
					WEB06_ACTION_OWNER.control,
					web06ControlAction("selectCandidate", [index]),
				),
			);
			const promise = withWeb06OwnedAction(
				WEB06_ACTION_OWNER.control,
				"selectCandidate",
				[index],
				"candidate-click-selection",
				undefined,
				() => Rime.selectCandidate(index),
			);
			handleRimeResult(promise);
		});
	}, [handleRimeResult]);
	const prepareDeleteCandidateFromLongPress = useCallback((
		event: ReactMouseEvent | ReactTouchEvent,
		index: number,
	): Web06DomEventIdentity => {
		let identity: Web06DomEventIdentity | undefined;
		withWeb06ControlEvent(event, () => {
			identity = declareWeb06ControlFanout(
				"candidate-800ms-long-press-delete",
				web06SingleActionFanout(
					WEB06_ACTION_OWNER.longPressDelete,
					web06ControlAction("deleteCandidate", [index]),
				),
			);
		});
		if (identity === undefined) {
			throw new Error("WEB-06 long-press delete event identity was not declared");
		}
		return identity;
	}, []);
	const deleteCandidateFromLongPress = useCallback((index: number) => {
		const promise = withWeb06OwnedAction(
			WEB06_ACTION_OWNER.longPressDelete,
			"deleteCandidate",
			[index],
			"candidate-800ms-long-press-delete",
			undefined,
			() => Rime.deleteCandidate(index),
		);
		handleRimeResult(promise);
	}, [handleRimeResult]);
	const selectCandidateFromDigitKey = useCallback((event: KeyboardEvent, context?: Web06ActionContext) => {
		const state = inputStateRef.current;
		if (!state || event.ctrlKey || event.metaKey || event.altKey || event.shiftKey || !/^[0-9]$/.test(event.key)) {
			return false;
		}
		const index = event.key === "0" ? 9 : Number(event.key) - 1;
		if (index >= Math.min(state.candidates.length, prefs.pageSize)) {
			return false;
		}
		event.preventDefault();
		selectCandidate(index, context);
		return true;
	}, [prefs.pageSize, selectCandidate]);

	const parseKey = useCallback((event: KeyboardEvent) => {
		const { code, key } = event;
		const hasControl = event.getModifierState("Control");
		const hasMeta = event.getModifierState("Meta");
		const hasAlt = event.getModifierState("Alt");
		const hasShift = event.getModifierState("Shift");
		if (!inputState && prefs.isAsciiMode && document.activeElement === textArea) {
			return undefined;
		}
		if (
			(inputState || (
				document.activeElement === textArea
					&& (!hasControl && (isPrintable(key) || !hasShift && key === "F4") || key === "`")
					&& !hasMeta
					&& !hasAlt
			)) && code
		) {
			const match = /^(Control|Meta|Alt|Shift)(Left|Right)$/.exec(code);
			const isNumpadKey = code.startsWith("Numpad");
			const modifiers = new Set<string>();
			if (hasControl) modifiers.add("Control");
			if (hasMeta) modifiers.add("Meta");
			if (hasAlt) modifiers.add("Alt");
			if (hasShift) modifiers.add("Shift");
			if (match) {
				modifiers.delete(match[1]);
				modifiers.add(`${match[1]}_${match[2][0]}`);
			}
			else {
				let rimeKey = isNumpadKey ? code.slice(6) : key;
				rimeKey = RIME_KEY_MAP[rimeKey] || rimeKey;
				modifiers.add(isNumpadKey ? `KP_${rimeKey}` : rimeKey);
			}
			return [...modifiers].join("+");
		}
		return undefined;
	}, [inputState, prefs.isAsciiMode, textArea]);

	useEffect(() => {
		function mappedActionContext(
			event: Web06DomEventIdentity,
			action: Web06MappedAction,
			eventActionIndex: number,
			rawKey: string,
		): Web06ActionContext {
			if (action.supersedable) {
				web06RawInputSequence.current = [...web06RawInputSequence.current, rawKey];
			}
			return {
				event,
				eventActionIndex,
				compositionEpochId: web06CompositionEpochId.current,
				supersessionSubRunId: web06SupersessionSubRunId.current,
				actionClass: action.actionClass,
				supersedable: action.supersedable,
				boundary: action.boundary,
				rawInputSequence: [...web06RawInputSequence.current],
			};
		}

		function instrumentKeyboardEvent(event: KeyboardEvent, eventDeliveredAt: number) {
			const snapshot = snapshotKeyboardEvent(event);
			const currentState = inputStateRef.current;
			const mapping = mapWeb06KeyboardEvent(snapshot, {
				capturedHasComposition: inputState !== undefined,
				currentHasComposition: currentState !== undefined,
				isAsciiMode: prefs.isAsciiMode,
				isInputFocused: document.activeElement === textArea,
				visibleCandidateCount: Math.min(currentState?.candidates.length ?? 0, prefs.pageSize),
				pendingAsciiModeShift: pendingAsciiModeShift.current,
				pendingAsciiModeShiftWasChorded: pendingAsciiModeShiftWasChorded.current,
				asciiModeToggleActions: web06AsciiModeToggleActions({
					nextAsciiMode: !prefs.isAsciiMode,
					isFullShape: prefs.isFullShape,
					outputStandard: prefs.outputStandard,
					activeSchema: prefs.activeSchema,
					isExtendedCharset: prefs.isExtendedCharset,
					isDisabled: prefs.isDisabled,
				}),
			});
			const eventIdentity = recordWeb06DomEvent(
				snapshot,
				mapping,
				web06CompositionEpochId.current,
				web06SupersessionSubRunId.current,
				eventDeliveredAt,
			);
			return { mapping, eventIdentity };
		}

		function expectedContext(
			event: KeyboardEvent,
			eventIdentity: Web06DomEventIdentity,
			action: Web06MappedAction | undefined,
			eventActionIndex: number,
			name: Web06MappedAction["name"],
			args: unknown[],
		): Web06ActionContext | undefined {
			if (
				action === undefined
				|| action.name !== name
				|| web06StableDigest(action.args) !== web06StableDigest(args)
			) {
				invalidateWeb06Measurement(
					"EVENT_ACTION_MAP_MISMATCH",
					`${event.type}/${event.code} expected ${action?.name ?? "no action"}, dispatched ${name}`,
				);
				return undefined;
			}
			return mappedActionContext(eventIdentity, action, eventActionIndex, event.key);
		}

		function verifyActionCount(
			event: KeyboardEvent,
			expected: number,
			actual: number,
		) {
			if (expected !== actual) {
				invalidateWeb06Measurement(
					"EVENT_ACTION_CARDINALITY_MISMATCH",
					`${event.type}/${event.code} expected ${expected} actions, dispatched ${actual}`,
				);
			}
		}

		function isModifierRelease(event: KeyboardEvent) {
			return /^(Control|Meta|Alt|Shift)(Left|Right)$/.test(event.code);
		}

		function isAsciiModeShiftTap(event: KeyboardEvent) {
			return event.key === "Shift"
				&& /^Shift(Left|Right)$/.test(event.code)
				&& !event.ctrlKey
				&& !event.metaKey
				&& !event.altKey;
		}

		function canToggleAsciiModeFromKeyboard() {
			return document.activeElement === textArea || Boolean(inputStateRef.current);
		}

		function onKeyDown(event: KeyboardEvent) {
			const eventDeliveredAt = web06Now();
			const { mapping, eventIdentity } = instrumentKeyboardEvent(event, eventDeliveredAt);
			if (isAsciiModeShiftTap(event) && canToggleAsciiModeFromKeyboard()) {
				if (pendingAsciiModeShift.current && event.code !== pendingAsciiModeShift.current) {
					pendingAsciiModeShiftWasChorded.current = true;
				}
				else if (!event.repeat && !pendingAsciiModeShift.current) {
					pendingAsciiModeShift.current = event.code;
					pendingAsciiModeShiftWasChorded.current = false;
				}
				event.preventDefault();
				verifyActionCount(event, mapping.actions.length, 0);
				return;
			}
			if (pendingAsciiModeShift.current) {
				pendingAsciiModeShiftWasChorded.current = true;
			}
			const digitAction = mapping.actions[0];
			const digitContext = digitAction?.name === "selectCandidate"
				? expectedContext(
					event,
					eventIdentity,
					digitAction,
					0,
					"selectCandidate",
					digitAction.args,
				)
				: undefined;
			if (selectCandidateFromDigitKey(event, digitContext)) {
				verifyActionCount(event, mapping.actions.length, 1);
				advanceWeb06Boundary("selection");
				return;
			}
			const key = parseKey(event);
			if (key) {
				event.preventDefault();
				const input = `{${key}}`;
				const action = mapping.actions[0];
				const context = expectedContext(
					event,
					eventIdentity,
					action,
					0,
					"processKey",
					[input],
				);
				processKey(input, event.key, nowMs(), context);
				verifyActionCount(event, mapping.actions.length, 1);
				advanceWeb06Boundary(action?.boundary ?? "error");
				return;
			}
			verifyActionCount(event, mapping.actions.length, 0);
		}

		function onKeyUp(event: KeyboardEvent) {
			const eventDeliveredAt = web06Now();
			const { mapping, eventIdentity } = instrumentKeyboardEvent(event, eventDeliveredAt);
			if (isAsciiModeShiftTap(event) && pendingAsciiModeShift.current === event.code) {
				event.preventDefault();
				const shouldToggle = !pendingAsciiModeShiftWasChorded.current && canToggleAsciiModeFromKeyboard();
					pendingAsciiModeShift.current = undefined;
					pendingAsciiModeShiftWasChorded.current = false;
					if (shouldToggle) {
						registerWeb06EventFanout(
							eventIdentity,
							mapping.actions.map(action => ({ owner: "live-options-effect", action })),
						);
					onToggleAsciiMode();
					verifyActionCount(event, mapping.actions.length, 12);
					advanceWeb06Boundary("modifier-release");
				}
				else {
					verifyActionCount(event, mapping.actions.length, 0);
				}
				return;
			}
			if (pendingAsciiModeShift.current && !isAsciiModeShiftTap(event)) {
				pendingAsciiModeShiftWasChorded.current = true;
			}
			if (/^[0-9]$/.test(event.key) && inputStateRef.current) {
				verifyActionCount(event, mapping.actions.length, 0);
				return;
			}
			if (inputState && isModifierRelease(event)) {
				const key = parseKey(event);
				if (key) {
					const input = `{Release+${key}}`;
					const action = mapping.actions[0];
					const context = expectedContext(
						event,
						eventIdentity,
						action,
						0,
						"processKey",
						[input],
					);
					processKey(input, undefined, undefined, context);
					verifyActionCount(event, mapping.actions.length, 1);
					advanceWeb06Boundary("modifier-release");
					return;
				}
			}
			verifyActionCount(event, mapping.actions.length, 0);
		}

		function onFocusLoss(event: FocusEvent) {
			const eventDeliveredAt = web06Now();
			const snapshot = web06FocusLossEvent(event.timeStamp);
			recordWeb06DomEvent(
				snapshot,
				{
					classification: "frontend-consumed",
					reason: "focus-loss-boundary",
					preventDefault: false,
					actions: [],
				},
				web06CompositionEpochId.current,
				web06SupersessionSubRunId.current,
				eventDeliveredAt,
			);
			advanceWeb06Boundary("focus-loss");
		}

		document.addEventListener("keydown", onKeyDown);
		document.addEventListener("keyup", onKeyUp);
		textArea.addEventListener("blur", onFocusLoss);
		return () => {
			document.removeEventListener("keydown", onKeyDown);
			document.removeEventListener("keyup", onKeyUp);
			textArea.removeEventListener("blur", onFocusLoss);
		};
	}, [
		inputState,
		onToggleAsciiMode,
		parseKey,
		prefs.activeSchema,
		prefs.isAsciiMode,
		prefs.isDisabled,
		prefs.isExtendedCharset,
		prefs.isFullShape,
		prefs.outputStandard,
		prefs.pageSize,
		processKey,
		selectCandidateFromDigitKey,
		textArea,
	]);

	useEffect(() => {
		lastClassicStateRef.current = undefined;
		setInputState(undefined);
		hideDictionary();
	}, [deployStatus, setInputState, hideDictionary]);

	useLayoutEffect(() => {
		inputStateRef.current = inputState;
		const pending = pendingPerfDiagnostics.current.splice(0);
		const committedInput = renderedInput(inputState);
		const renderRevision = committedRender.current.revision + 1;
		committedRender.current = { input: committedInput, revision: renderRevision };
		if (committedInput === undefined) {
			return;
		}
		const stateAppliedAt = nowMs();
		for (const diagnostic of pending.filter(diagnostic => diagnostic.input === committedInput)) {
			requestAnimationFrame(() => {
				if (
					committedRender.current.revision !== renderRevision
					|| committedRender.current.input !== committedInput
				) {
					// This state was superseded before the frame that would paint it.
					// Leave the diagnostic absent so the gate fails closed.
					return;
				}
				// The first callback runs before its frame is painted. Observe from
				// the next frame so layout/paint of the applied candidate state is
				// included in the latency hard stop.
				requestAnimationFrame(() => {
					const paintObservedAt = nowMs();
					appendPerfDiagnostic({
						...diagnostic,
						renderedInput: committedInput,
						renderRevision,
						stateAppliedAt,
						paintObservedAt,
						reactUpdateMs: Math.round(stateAppliedAt - diagnostic.responseMappingFinishedAt),
						paintProxyMs: Math.round(paintObservedAt - stateAppliedAt),
						totalKeydownToPaintMs: Math.round(paintObservedAt - diagnostic.keydownAt),
					});
				});
			});
		}
	}, [inputState]);

	useLayoutEffect(() => {
		if (web06Render === undefined || web06Render.resolved) return;
		const renderRevision = ++web06RenderRevision.current;
		web06Render.stateCommittedAt = web06Now();
		requestAnimationFrame(() => {
			if (web06RenderRevision.current !== renderRevision) return;
			web06Render.firstRafAt = web06Now();
			requestAnimationFrame(() => {
				if (web06RenderRevision.current !== renderRevision || web06Render.resolved) return;
				const domObserved = observedPresentationFingerprint(textArea);
				const domDigest = web06StableDigest(domObserved);
				const expectedDigest = web06StableDigest(web06Render.presentationExpected);
				const terminalObservedAt = web06Now();
				const exactPresentation = domDigest === expectedDigest;

				for (const pending of pendingWeb06Presentations.current) {
					if (
						pending.resolved
						|| pending.identity.sequenceId >= web06Render.identity.sequenceId
					) {
						continue;
					}
					const renderedPrefix = web06Render.presentationExpected.input.startsWith(
						pending.presentationExpected.input,
					) && web06Render.presentationExpected.input.length > pending.presentationExpected.input.length;
					const canSupersede = renderedPrefix
						&& canWeb06Supersede(pending.identity, web06Render.identity);
					const outcome = canSupersede ? "superseded" : "failure";
					if (!canSupersede) {
						invalidateWeb06Measurement(
							pending.identity.supersedable
								? "INVALID_SUPERSESSION_LINK"
								: "BARRIER_SUPERSEDED",
							`Action ${pending.identity.actionId} did not reach an exact terminal state before ${web06Render.identity.actionId}`,
							pending.identity,
						);
					}
					recordWeb06PresentationOutcome({
						identity: pending.identity,
						outcome,
						stateUpdateScheduledAt: pending.stateUpdateScheduledAt,
						stateCommittedAt: pending.stateCommittedAt,
						firstRafAt: pending.firstRafAt,
						terminalObservedAt,
						presentationExpected: pending.presentationExpected,
						domObserved,
						presentationDigest: domDigest,
						...(canSupersede ? {
							supersededBySequenceId: web06Render.identity.sequenceId,
							supersessionSequenceLag:
								web06Render.identity.sequenceId - pending.identity.sequenceId,
						} : {}),
					});
					pending.resolved = true;
				}

				const visualChanged = web06Render.beforePresentationDigest !== domDigest;
				const outcome = !exactPresentation
					? "failure"
					: web06Render.committed
					? "committed"
					: web06Render.identity.actionClass === "stateful-barrier"
					? visualChanged ? "painted" : "barrier-completed"
					: visualChanged
					? "painted"
					: "processed-no-visual-change";
				if (!exactPresentation) {
					invalidateWeb06Measurement(
						"PRESENTATION_FINGERPRINT_MISMATCH",
						`Expected ${expectedDigest}, observed ${domDigest}`,
						web06Render.identity,
					);
				}
				recordWeb06PresentationOutcome({
					identity: web06Render.identity,
					outcome,
					stateUpdateScheduledAt: web06Render.stateUpdateScheduledAt,
					stateCommittedAt: web06Render.stateCommittedAt,
					firstRafAt: web06Render.firstRafAt,
					terminalObservedAt,
					presentationExpected: web06Render.presentationExpected,
					domObserved,
					presentationDigest: domDigest,
				});
				web06Render.resolved = true;
				pendingWeb06Presentations.current = pendingWeb06Presentations.current.filter(
					pending => !pending.resolved,
				);
				try {
					document.documentElement.dataset["yuneWeb06Terminal"] = [
						"web06-private-v1",
						web06Render.identity.sequenceId,
						web06Render.identity.compositionEpochId,
						web06Render.identity.supersessionSubRunId,
						outcome,
						domDigest,
					].join("|");
				}
				catch (error) {
					invalidateWeb06Measurement(
						"DOM_TERMINAL_MARKER_FAILURE",
						error instanceof Error ? `${error.name}: ${error.message}` : String(error),
						web06Render.identity,
					);
				}
			});
		});
	}, [inputState, textArea, web06Render]);

	useEffect(() => {
		if (!prefs.enableAI) {
			onMetrics?.({ aiMs: undefined });
			if (lastClassicStateRef.current && inputStateRef.current !== lastClassicStateRef.current) {
				inputStateRef.current = lastClassicStateRef.current;
				setInputState(lastClassicStateRef.current);
				hideDictionary();
			}
			return;
		}
		if (inputStateRef.current) {
			handleRimeResult(withWeb06OwnedAction(
				"stage-ai-background",
				"stageAi",
				[],
				"stage-ai-after-ai-setting",
				lastClassicActionIdentity.current,
				() => Rime.stageAi(),
			), undefined, undefined, "ai");
		}
	}, [aiStatus, handleRimeResult, hideDictionary, onMetrics, prefs.enableAI]);

	const hideDictionaryOnLeaveCandidate = useCallback(() => {
		function hideDictionaryOnLeaveDictionaryPanel() {
			if (!candidateList.current?.matches(":hover")) {
				hideDictionary();
			}
			dictionaryPanel.current?.removeEventListener("mouseleave", hideDictionaryOnLeaveDictionaryPanel);
			dictionaryPanel.current?.removeEventListener("touchend", hideDictionaryOnLeaveDictionaryPanel);
		}
		if (dictionaryPanel.current?.matches(":hover")) {
			dictionaryPanel.current.addEventListener("mouseleave", hideDictionaryOnLeaveDictionaryPanel);
			dictionaryPanel.current.addEventListener("touchend", hideDictionaryOnLeaveDictionaryPanel);
		}
		else if (!candidateList.current?.matches(":hover")) {
			hideDictionary();
		}
	}, [hideDictionary]);

	if (!inputState) return null;

	const visibleCandidates = inputState?.candidates.slice(0, prefs.pageSize) ?? [];
	const activeCandidateIndex = typeof showDictionaryIndex === "number" ? showDictionaryIndex : inputState.highlightedIndex;
	const dictionaryIndex = typeof showDictionaryIndex === "number"
		? showDictionaryIndex
		: prefs.showDictionaryByDefault
		? inputState.highlightedIndex
		: undefined;
	const dictionaryCandidate = typeof dictionaryIndex === "number" ? visibleCandidates[dictionaryIndex] : undefined;
	const hasDictionaryPanel = dictionaryCandidate?.hasDictionaryEntry(prefs) ?? false;
	const panelContent = <>
		<div className="candidate-list-pane">
			<div className="candidate-panel-header">
				<div className="candidate-preedit">
					{inputState.inputBuffer.before && <span>{inputState.inputBuffer.before}</span>}
					{inputState.inputBuffer.active && <span className="candidate-preedit-active">{inputState.inputBuffer.active}</span>}
					{inputState.inputBuffer.after && <span>{inputState.inputBuffer.after}</span>}
					<span className="candidate-caret" aria-hidden="true" />
				</div>
				<div className="candidate-nav">
					<button className="page-nav" disabled={inputState.isPrevDisabled} onClick={event => flipPageFromControl(event, true)}>
						<span>‹</span>
					</button>
					<button className="page-nav" disabled={inputState.isNextDisabled} onClick={event => flipPageFromControl(event, false)}>
						<span>›</span>
					</button>
				</div>
			</div>
			<table ref={candidateList} className="candidates">
				{visibleCandidates.map((candidate, index) =>
					<Candidate
						key={index}
						info={candidate}
						isHighlighted={index === activeCandidateIndex}
						selectCandidate={event => selectCandidateFromControl(event, index)}
						prepareDeleteCandidate={event => prepareDeleteCandidateFromLongPress(event, index)}
						deleteCandidate={() => deleteCandidateFromLongPress(index)}
						showDictionary={() => setShowDictionaryIndex(index)}
						hideDictionary={hideDictionaryOnLeaveCandidate}
						prefs={prefs} />
				)}
			</table>
		</div>
		{hasDictionaryPanel && dictionaryCandidate && <DictionaryPanel info={dictionaryCandidate} prefs={prefs} ref={dictionaryPanel} />}
	</>;
	const panelClassName = [
		"candidate-panel",
		`candidate-panel--${prefs.candidateMenuLayout}`,
		hasDictionaryPanel ? "candidate-panel--with-dictionary" : "candidate-panel--list-only",
		prefs.isCandidatePanelFixed ? "candidate-panel--fixed" : "",
	].filter(Boolean).join(" ");
	const web06StateMarker = [
		"web06-private-v1",
		web06Render?.identity.sequenceId ?? 0,
		inputState.page,
		inputState.isLastPage ? 1 : 0,
		inputState.highlightedIndex,
	].join("|");
	if (prefs.isCandidatePanelFixed) {
		return <div className={panelClassName} data-yune-web06-state={web06StateMarker}>{panelContent}</div>;
	}
	return <CaretFollower textArea={textArea} className={panelClassName} data-yune-web06-state={web06StateMarker}>{panelContent}</CaretFollower>;
}

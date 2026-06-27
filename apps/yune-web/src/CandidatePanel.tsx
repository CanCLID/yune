import { useState, useEffect, useRef, useCallback, useLayoutEffect } from "react";

import Candidate from "./Candidate";
import CandidateInfo from "./CandidateInfo";
import CaretFollower from "./CaretFollower";
import { RIME_KEY_MAP } from "./consts";
import DictionaryPanel from "./DictionaryPanel";
import Rime from "./rime";
import { notify } from "./toast";
import { isPrintable } from "./utils";

import type { InputState, Preferences, RimeResult, YuneInspectorDebug, YuneStatusSnapshot } from "./types";

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
	totalMs?: number;
}

interface PendingPerfDiagnostic {
	input: string;
	key?: string;
	keydownAt: number;
	workerQueuedAt: number;
	workerStartedAt: number;
	workerFinishedAt: number;
	responseReceivedAt: number;
	responseMappingFinishedAt: number;
	workerQueueWaitMs?: number;
	workerProcessMs?: number;
	workerRoundtripMs?: number;
	responseMappingMs: number;
	totalWorkerActionMs?: number;
	wasmHeapBytes?: number;
	peakWasmHeapBytes?: number;
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

function nowMs() {
	return performance.timeOrigin + performance.now();
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
	stateAppliedAt: number;
	paintObservedAt: number;
	candidateCount: number;
	totalCandidateCount: number;
	firstCandidateText?: string;
	reactUpdateMs: number;
	paintProxyMs: number;
	totalKeydownToPaintMs: number;
}) {
	const existing = document.documentElement.dataset["yunePerfDiagnostics"];
	const diagnostics = existing ? JSON.parse(existing) as typeof diagnostic[] : [];
	diagnostics.push(diagnostic);
	document.documentElement.dataset["yunePerfDiagnostics"] = JSON.stringify(diagnostics.slice(-100));
}

export default function CandidatePanel({
	textArea,
	prefs,
	deployStatus,
	aiStatus,
	onInspectorDebug,
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
	onStatus?(status: YuneStatusSnapshot | undefined): void;
	onUserdbChange?(): void;
	onMetrics?(metrics: MetricUpdate): void;
	onToggleAsciiMode(): void;
}) {
	const [inputState, setInputState] = useState<InputState | undefined>();
	const inputStateRef = useRef<InputState | undefined>();
	const [showDictionaryIndex, setShowDictionaryIndex] = useState<number | undefined>();
	const candidateList = useRef<HTMLTableElement>(null);
	const dictionaryPanel = useRef<HTMLDivElement>(null);
	const pendingPerfDiagnostics = useRef<PendingPerfDiagnostic[]>([]);
	const lastClassicStateRef = useRef<InputState | undefined>();
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

	const handleRimeResult = useCallback((promise: Promise<RimeResult>, key?: string, keydownContext?: { input: string; key?: string; keydownAt: number }, metricKind?: "lookup" | "ai") => {
		const startedAt = performance.now();
		void (async () => {
			let type: "warning" | "error" | undefined;
			try {
				const result = await promise;
				const responseReceivedAt = nowMs();
				const responseMappingStartedAt = nowMs();
				onInspectorDebug?.(result.isComposing ? result.debug : undefined);
				onStatus?.(result.status);
				if (!result.success) {
					type = "warning";
				}
				const state = result.isComposing
					? {
						inputBuffer: result.inputBuffer,
						highlightedIndex: result.highlightedIndex,
						candidates: result.candidates.map(
							({ label, text, comment, source }, i) => new CandidateInfo(label || `${(i + 1) % 10}.`, text, comment, source),
						),
						isPrevDisabled: !result.page,
						isNextDisabled: result.isLastPage,
					}
					: inputStateRef.current;
				const responseMappingFinishedAt = nowMs();
				if (result.committed) {
					insert(result.committed);
					onUserdbChange?.();
				}
				else if (!state && key && isPrintable(key)) {
					insert(key);
				}
				if (keydownContext) {
					const actionDiagnostic = readLatestProcessKeyActionDiagnostic(keydownContext.input);
					pendingPerfDiagnostics.current.push({
						input: result.isComposing ? result.inputBuffer.active || result.inputBuffer.before : keydownContext.input,
						key: keydownContext.key,
						keydownAt: keydownContext.keydownAt,
						workerQueuedAt: actionDiagnostic?.enqueuedAt ?? keydownContext.keydownAt,
						workerStartedAt: actionDiagnostic?.workerStartedAt ?? actionDiagnostic?.sentAt ?? responseReceivedAt,
						workerFinishedAt: actionDiagnostic?.workerFinishedAt ?? responseReceivedAt,
						responseReceivedAt: actionDiagnostic?.receivedAt ?? responseReceivedAt,
						responseMappingFinishedAt,
						workerQueueWaitMs: actionDiagnostic?.queueWaitMs,
						workerProcessMs: actionDiagnostic?.workerMs,
						workerRoundtripMs: actionDiagnostic?.workerRoundtripMs,
						responseMappingMs: Math.round(responseMappingFinishedAt - responseMappingStartedAt),
						totalWorkerActionMs: actionDiagnostic?.totalMs,
						wasmHeapBytes: result.memory?.wasmHeapBytes,
						peakWasmHeapBytes: result.memory?.peakWasmHeapBytes,
					});
				}
				const candidateCount = result.isComposing ? Math.min(result.candidates.length, prefs.pageSize) : undefined;
				const totalCandidateCount = result.isComposing ? result.candidates.length : undefined;
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
				setInputState(result.isComposing ? state : undefined);
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
			catch {
				type = "error";
			}
			if (type) {
				notify(type, "執行操作", "performing the operation");
			}
			textArea.focus();
		})();
	}, [hideDictionary, insert, onInspectorDebug, onMetrics, onStatus, onUserdbChange, prefs.pageSize, textArea]);

	const processKey = useCallback((input: string, key?: string, keydownAt?: number) => {
		const classicResult = Rime.processKey(input);
		handleRimeResult(classicResult, key, keydownAt === undefined ? undefined : { input, key, keydownAt }, "lookup");
		if (prefs.enableAI) {
			void classicResult.then(result => {
				if (result.isComposing) {
					handleRimeResult(Rime.stageAi(), undefined, undefined, "ai");
				}
			}).catch(() => undefined);
		}
	}, [handleRimeResult, prefs.enableAI]);
	const flipPage = useCallback((backward: boolean) => handleRimeResult(Rime.flipPage(backward)), [handleRimeResult]);
	const selectCandidate = useCallback((index: number) => handleRimeResult(Rime.selectCandidate(index)), [handleRimeResult]);
	const deleteCandidate = useCallback((index: number) => handleRimeResult(Rime.deleteCandidate(index)), [handleRimeResult]);
	const selectCandidateFromDigitKey = useCallback((event: KeyboardEvent) => {
		const state = inputStateRef.current;
		if (!state || event.ctrlKey || event.metaKey || event.altKey || event.shiftKey || !/^[0-9]$/.test(event.key)) {
			return false;
		}
		const index = event.key === "0" ? 9 : Number(event.key) - 1;
		if (index >= Math.min(state.candidates.length, prefs.pageSize)) {
			return false;
		}
		event.preventDefault();
		selectCandidate(index);
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
			if (hasControl) {
				modifiers.add("Control");
			}
			if (hasMeta) {
				modifiers.add("Meta");
			}
			if (hasAlt) {
				modifiers.add("Alt");
			}
			if (hasShift) {
				modifiers.add("Shift");
			}
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
			if (isAsciiModeShiftTap(event) && canToggleAsciiModeFromKeyboard()) {
				if (pendingAsciiModeShift.current && event.code !== pendingAsciiModeShift.current) {
					pendingAsciiModeShiftWasChorded.current = true;
				}
				else if (!event.repeat && !pendingAsciiModeShift.current) {
					pendingAsciiModeShift.current = event.code;
					pendingAsciiModeShiftWasChorded.current = false;
				}
				event.preventDefault();
				return;
			}
			if (pendingAsciiModeShift.current) {
				pendingAsciiModeShiftWasChorded.current = true;
			}
			if (selectCandidateFromDigitKey(event)) {
				return;
			}
			const key = parseKey(event);
			if (key) {
				event.preventDefault();
				processKey(`{${key}}`, event.key, nowMs());
			}
		}
		function onKeyUp(event: KeyboardEvent) {
			if (isAsciiModeShiftTap(event) && pendingAsciiModeShift.current === event.code) {
				event.preventDefault();
				const shouldToggle = !pendingAsciiModeShiftWasChorded.current && canToggleAsciiModeFromKeyboard();
				pendingAsciiModeShift.current = undefined;
				pendingAsciiModeShiftWasChorded.current = false;
				if (shouldToggle) {
					onToggleAsciiMode();
				}
				return;
			}
			if (pendingAsciiModeShift.current && !isAsciiModeShiftTap(event)) {
				pendingAsciiModeShiftWasChorded.current = true;
			}
			if (/^[0-9]$/.test(event.key) && inputStateRef.current) {
				return;
			}
			if (inputState && isModifierRelease(event)) {
				const key = parseKey(event);
				if (key) processKey(`{Release+${key}}`);
			}
		}
		document.addEventListener("keydown", onKeyDown);
		document.addEventListener("keyup", onKeyUp);
		return () => {
			document.removeEventListener("keydown", onKeyDown);
			document.removeEventListener("keyup", onKeyUp);
		};
	}, [inputState, onToggleAsciiMode, parseKey, processKey, selectCandidateFromDigitKey, textArea]);

	useEffect(() => {
		lastClassicStateRef.current = undefined;
		setInputState(undefined);
		hideDictionary();
	}, [deployStatus, setInputState, hideDictionary]);

	useLayoutEffect(() => {
		inputStateRef.current = inputState;
		const pending = pendingPerfDiagnostics.current.splice(0);
		if (pending.length === 0) {
			return;
		}
		const stateAppliedAt = nowMs();
		const renderedCandidates = inputState?.candidates.slice(0, prefs.pageSize) ?? [];
		const totalCandidateCount = inputState?.candidates.length ?? 0;
		for (const diagnostic of pending) {
			requestAnimationFrame(() => {
				const paintObservedAt = nowMs();
				appendPerfDiagnostic({
					...diagnostic,
					stateAppliedAt,
					paintObservedAt,
					candidateCount: renderedCandidates.length,
					totalCandidateCount,
					firstCandidateText: renderedCandidates[0]?.text,
					reactUpdateMs: Math.round(stateAppliedAt - diagnostic.responseMappingFinishedAt),
					paintProxyMs: Math.round(paintObservedAt - stateAppliedAt),
					totalKeydownToPaintMs: Math.round(paintObservedAt - diagnostic.keydownAt),
				});
			});
		}
	}, [inputState, prefs.pageSize]);

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
			handleRimeResult(Rime.stageAi(), undefined, undefined, "ai");
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
					<button className="page-nav" disabled={inputState.isPrevDisabled} onClick={() => flipPage(true)}>
						<span>‹</span>
					</button>
					<button className="page-nav" disabled={inputState.isNextDisabled} onClick={() => flipPage(false)}>
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
						selectCandidate={() => selectCandidate(index)}
						deleteCandidate={() => deleteCandidate(index)}
						showDictionary={() => setShowDictionaryIndex(index)}
						hideDictionary={hideDictionaryOnLeaveCandidate}
						prefs={prefs} />
				)}
			</table>
		</div>
		{dictionaryCandidate && <DictionaryPanel info={dictionaryCandidate} prefs={prefs} ref={dictionaryPanel} />}
	</>;
	const panelClassName = `candidate-panel candidate-panel--${prefs.candidateMenuLayout}${prefs.isCandidatePanelFixed ? " candidate-panel--fixed" : ""}`;
	if (prefs.isCandidatePanelFixed) {
		return <div className={panelClassName}>{panelContent}</div>;
	}
	return <CaretFollower textArea={textArea} className={panelClassName}>{panelContent}</CaretFollower>;
}

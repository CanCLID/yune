// @vitest-environment jsdom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

import { DEFAULT_PREFERENCES } from "../src/consts.js";
import {
	web06AdapterProjectionFingerprint,
	web06AdapterProjectionFingerprintsEqual,
	web06EngineRawAdapterProjection,
	web06EngineRawFingerprint,
	web06PresentationStateDigest,
	web06StableDigest,
} from "../src/yune-integration/private-protocol.js";

import type { Preferences, RimeResult, Web06WorkerReceipt } from "../src/types.js";

type WorkerMessage = Record<string, any>;

class ComponentWorker {
	static latest: ComponentWorker;
	readonly sent: WorkerMessage[] = [];
	readonly url: string;
	#messageListener?: (event: { data: WorkerMessage }) => void;

	constructor(url: string | URL) {
		this.url = String(url);
		ComponentWorker.latest = this;
	}

	addEventListener(type: string, listener: (event: { data: WorkerMessage }) => void) {
		if (type === "message") this.#messageListener = listener;
	}

	postMessage(message: WorkerMessage) {
		this.sent.push(message);
	}

	emit(data: WorkerMessage) {
		this.#messageListener?.({ data });
	}

	actions() {
		return this.sent.filter(message => message.kind === "action");
	}
}

interface MountedPanel {
	root: Root;
	container: HTMLDivElement;
	textArea: HTMLTextAreaElement;
	worker: ComponentWorker;
	debug: {
		snapshot(): Record<string, any>;
	};
	flushAllRafs(): Promise<void>;
}

let mounted: MountedPanel | undefined;

function composingResult(input: string, page = 0, isLastPage = true): RimeResult {
	return {
		isComposing: true,
		success: true,
		inputBuffer: { before: "", active: input, after: "" },
		page,
		isLastPage,
		highlightedIndex: -1,
		candidates: [],
	};
}

function rawResponse(input: string, page = 0, isLastPage = true): string {
	return JSON.stringify({
		handled: true,
		commits: [],
		context: {
			input,
			preedit: input,
			caret: input.length,
			highlighted: -1,
			page_size: 6,
			page_no: page,
			is_last_page: isLastPage,
			select_keys: null,
			select_labels: [],
			candidates: [],
		},
		status: null,
	});
}

function successReceipt(envelope: WorkerMessage, result: RimeResult, rawJson: string): Web06WorkerReceipt {
	const at = performance.now();
	const rawFingerprint = web06EngineRawFingerprint(envelope.name, envelope.name, rawJson);
	const rawProjection = web06EngineRawAdapterProjection(rawFingerprint);
	const adapterProjection = web06AdapterProjectionFingerprint(result);
	return {
		workerMessageReceivedAt: at,
		workerActionStartedAt: at,
		workerFinishedAt: at,
		runtimeSpans: [],
		adapterSpans: [],
		persistenceSpans: [],
		collectorSpans: [],
		engineRawJson: rawJson,
		engineRawOperation: envelope.name,
		engineRaw: {
			availability: "captured",
			action: envelope.name,
			operation: envelope.name,
			jsonDigest: web06StableDigest(rawJson),
			rawFingerprintDigest: web06StableDigest(rawFingerprint),
			rawProjectionDigest: web06StableDigest(rawProjection),
			adapterProjectionDigest: web06StableDigest(adapterProjection),
			projectionMatches: web06AdapterProjectionFingerprintsEqual(rawProjection, adapterProjection),
			rawFingerprint,
			adapterProjection,
		},
		observerFailures: [],
		lifecycleEffects: [],
		resultSummary: {
			kind: "rime-result",
			resultDigest: web06StableDigest({
				success: result.success,
				isComposing: result.isComposing,
			}),
			success: true,
			persistenceCompleted: false,
		},
	};
}

async function emitSuccess(worker: ComponentWorker, envelope: WorkerMessage, result: RimeResult) {
	const rawJson = rawResponse(
		result.isComposing ? result.inputBuffer.active : "",
		result.isComposing ? result.page : 0,
		result.isComposing ? result.isLastPage : true,
	);
	await act(async () => {
		worker.emit({
			protocolVersion: "web06-private-v1",
			kind: "action-result",
			mode: envelope.mode,
			modeProvenance: envelope.modeProvenance,
			identity: structuredClone(envelope.identity),
			resultType: "success",
			result,
			receipt: successReceipt(envelope, result, rawJson),
		});
		await Promise.resolve();
	});
}

function timedEvent<T extends Event>(event: T): T {
	Object.defineProperty(event, "timeStamp", { configurable: true, value: performance.now() });
	return event;
}

async function mountCandidatePanel(
	callbacks: {
		onInspectorCandidates?: (candidates: unknown[]) => void;
	} = {},
): Promise<MountedPanel> {
	vi.resetModules();
	window.history.replaceState({}, "", "/?yuneWeb06Mode=full");
	vi.stubGlobal("Worker", ComponentWorker);
	(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

	const rafCallbacks: FrameRequestCallback[] = [];
	vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
		rafCallbacks.push(callback);
		return rafCallbacks.length;
	});
	const [{ default: CandidatePanel }] = await Promise.all([
		import("../src/CandidatePanel.js"),
	]);
	const debug = (window as typeof window & {
		__YUNE_WEB06__: MountedPanel["debug"];
	}).__YUNE_WEB06__;
	const textArea = document.createElement("textarea");
	const container = document.createElement("div");
	document.body.append(textArea, container);
	textArea.focus();
	const root = createRoot(container);
	const prefs: Preferences = {
		...DEFAULT_PREFERENCES,
		displayLanguages: new Set(DEFAULT_PREFERENCES.displayLanguages),
		isCandidatePanelFixed: true,
		showDictionaryByDefault: false,
	};
	await act(async () => {
		root.render(<CandidatePanel
			runAsyncTask={() => undefined}
			textArea={textArea}
			prefs={prefs}
			deployStatus={0}
			aiStatus={0}
			onInspectorCandidates={callbacks.onInspectorCandidates}
			onClaimWeb06LiveOptions={() => undefined}
			onToggleAsciiMode={() => undefined} />);
		await Promise.resolve();
	});
	const panel: MountedPanel = {
		root,
		container,
		textArea,
		worker: ComponentWorker.latest,
		debug,
		async flushAllRafs() {
			let count = 0;
			while (rafCallbacks.length > 0) {
				if (++count > 64) throw new Error("CandidatePanel rAF queue did not settle");
				const callback = rafCallbacks.shift()!;
				await act(async () => {
					callback(performance.now());
					await Promise.resolve();
				});
			}
		},
	};
	mounted = panel;
	return panel;
}

async function dispatchKey(key: string) {
	await act(async () => {
		document.dispatchEvent(timedEvent(new KeyboardEvent("keydown", {
			bubbles: true,
			cancelable: true,
			key,
			code: `Key${key.toUpperCase()}`,
		})));
		await Promise.resolve();
	});
}

afterEach(async () => {
	if (mounted !== undefined) {
		await act(async () => mounted?.root.unmount());
		mounted.container.remove();
		mounted.textArea.remove();
		mounted = undefined;
	}
	delete (window as typeof window & { __YUNE_WEB06__?: unknown }).__YUNE_WEB06__;
	vi.unstubAllGlobals();
	vi.restoreAllMocks();
	vi.resetModules();
});

describe("WEB-06 CandidatePanel protocol integration", () => {
	it("reads the second queued result's before-state from the first committed DOM", async () => {
		const panel = await mountCandidatePanel();
		await dispatchKey("n");
		await dispatchKey("i");
		expect(panel.worker.actions()).toHaveLength(1);

		const firstEnvelope = panel.worker.actions()[0]!;
		await emitSuccess(panel.worker, firstEnvelope, composingResult("n"));
		await panel.flushAllRafs();
		expect(panel.worker.actions()).toHaveLength(2);

		const secondEnvelope = panel.worker.actions()[1]!;
		await emitSuccess(panel.worker, secondEnvelope, composingResult("ni"));
		await panel.flushAllRafs();

		const snapshot = panel.debug.snapshot();
		expect(snapshot.invalidations, JSON.stringify(snapshot.invalidations, null, 2)).toEqual([]);
		expect(snapshot.actions).toHaveLength(2);
		const [first, second] = snapshot.actions;
		expect(first.presentation.outcome).toBe("painted");
		expect(second.presentation.outcome).toBe("painted");
		expect(second.presentation.beforePresentationDigest).toBe(
			web06PresentationStateDigest(first.presentation.domObserved),
		);
		expect(second.presentation.domObserved.input).toBe("ni");
	});

	it("maps one real page-control click to one invocation, one application, and one terminal", async () => {
		const applied = vi.fn();
		const panel = await mountCandidatePanel({ onInspectorCandidates: applied });
		await dispatchKey("n");
		await emitSuccess(panel.worker, panel.worker.actions()[0]!, composingResult("n", 0, false));
		await panel.flushAllRafs();
		const appliedBeforeClick = applied.mock.calls.length;

		const next = panel.container.querySelectorAll<HTMLButtonElement>(".page-nav")[1]!;
		expect(next.disabled).toBe(false);
		await act(async () => {
			next.dispatchEvent(timedEvent(new MouseEvent("click", { bubbles: true, cancelable: true })));
			await Promise.resolve();
		});
		const pageEnvelopes = panel.worker.actions().filter(message => message.name === "flipPage");
		expect(pageEnvelopes).toHaveLength(1);
		expect(pageEnvelopes[0].args).toEqual([false]);

		await emitSuccess(panel.worker, pageEnvelopes[0], composingResult("n", 1, true));
		await panel.flushAllRafs();
		const snapshot = panel.debug.snapshot();
		const pageReceipts = snapshot.actions.filter((action: any) => action.name === "flipPage");
		expect(pageReceipts).toHaveLength(1);
		expect(pageReceipts[0].presentation).toMatchObject({
			outcome: "painted",
			presentationExpected: { page: 1 },
			domObserved: { page: 1 },
		});
		expect(Number.isFinite(pageReceipts[0].presentation.stateUpdateScheduledAt)).toBe(true);
		expect(applied.mock.calls.length - appliedBeforeClick).toBe(1);
		expect(snapshot.invalidations, JSON.stringify(snapshot.invalidations, null, 2)).toEqual([]);
	});
});

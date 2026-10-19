// @vitest-environment jsdom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

import { DEFAULT_PREFERENCES } from "../src/consts.js";
import { engineActionToastType } from "../src/yune-integration/action-toast.js";
import { translateResponse } from "../src/yune-integration/response.js";

import type { Preferences, RimeResult } from "../src/types.js";

interface WorkerMessage {
	kind: string;
	protocolVersion?: string;
	mode?: string;
	modeProvenance?: string;
	identity?: unknown;
	name?: string;
	args?: unknown[];
}

// What the adapter hands the panel when the engine reports `handled: false`
// with nothing composed (Space or a digit on an empty composition).
const passThroughResult = translateResponse({ handled: false, commits: [], context: null, status: null });

class ToastWorker {
	static latest: ToastWorker;
	readonly sent: WorkerMessage[] = [];
	#messageListener?: (event: { data: unknown }) => void;

	constructor() {
		ToastWorker.latest = this;
	}

	addEventListener(type: string, listener: (event: { data: unknown }) => void) {
		if (type === "message") this.#messageListener = listener;
	}

	postMessage(message: WorkerMessage) {
		this.sent.push(message);
	}

	actions() {
		return this.sent.filter(message => message.kind === "action");
	}

	async settle(envelope: WorkerMessage, outcome: { result: RimeResult } | { error: unknown }) {
		await act(async () => {
			this.#messageListener?.({
				data: {
					protocolVersion: envelope.protocolVersion,
					kind: "action-result",
					mode: envelope.mode,
					modeProvenance: envelope.modeProvenance,
					identity: structuredClone(envelope.identity),
					...("result" in outcome
						? { resultType: "success", result: outcome.result }
						: { resultType: "error", error: outcome.error }),
					// Only what the main-thread receipt ledger reads; WEB-06 timing
					// evidence is out of scope for the toast policy.
					receipt: {
						runtimeSpans: [],
						adapterSpans: [],
						persistenceSpans: [],
						collectorSpans: [],
						observerFailures: [],
						lifecycleEffects: [],
					},
				},
			});
			// Let the panel's async result handler run to completion.
			await new Promise(resolve => setTimeout(resolve, 0));
		});
	}
}

interface MountedPanel {
	root: Root;
	container: HTMLDivElement;
	textArea: HTMLTextAreaElement;
	worker: ToastWorker;
	toast(): HTMLElement | null;
}

let mounted: MountedPanel | undefined;

async function mountPanelWithToasts(): Promise<MountedPanel> {
	vi.resetModules();
	window.history.replaceState({}, "", "/");
	vi.stubGlobal("Worker", ToastWorker);
	vi.stubGlobal("requestAnimationFrame", () => 0);
	vi.spyOn(console, "log").mockImplementation(() => undefined);
	(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
	const [{ default: CandidatePanel }, { ToastViewport }] = await Promise.all([
		import("../src/CandidatePanel.js"),
		import("../src/toast.js"),
	]);
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
		root.render(<>
			<CandidatePanel
				runAsyncTask={() => undefined}
				textArea={textArea}
				prefs={prefs}
				deployStatus={0}
				aiStatus={0}
				onClaimWeb06LiveOptions={() => undefined}
				onToggleAsciiMode={() => undefined} />
			<ToastViewport />
		</>);
		await Promise.resolve();
	});
	mounted = {
		root,
		container,
		textArea,
		worker: ToastWorker.latest,
		toast: () => container.querySelector<HTMLElement>(".yd-toast"),
	};
	return mounted;
}

async function pressKey(key: string, code: string, repeat = false) {
	await act(async () => {
		document.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, key, code, repeat }));
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

describe("engineActionToastType", () => {
	it("does not toast a key the engine passed through", () => {
		expect(passThroughResult).toEqual({ isComposing: false, success: false });
		expect(engineActionToastType({ status: "fulfilled", value: passThroughResult })).toBeUndefined();
	});

	it("does not toast a handled result", () => {
		const committed: RimeResult = { isComposing: false, success: true, committed: "你" };
		expect(engineActionToastType({ status: "fulfilled", value: committed })).toBeUndefined();
	});

	it("shows the error toast for a rejected action", () => {
		expect(engineActionToastType({ status: "rejected", reason: new Error("worker failed") })).toBe("error");
		expect(engineActionToastType({ status: "rejected", reason: "worker failed" })).toBe("error");
	});
});

describe("CandidatePanel action toasts", () => {
	it("inserts unhandled Space, repeated Space, and digits on an empty composition without a toast", async () => {
		const panel = await mountPanelWithToasts();
		const keys = [
			{ key: " ", code: "Space", input: "{space}" },
			{ key: " ", code: "Space", input: "{space}", repeat: true },
			{ key: "1", code: "Digit1", input: "{1}" },
		];
		for (const [index, { key, code, input, repeat }] of keys.entries()) {
			await pressKey(key, code, repeat);
			const envelope = panel.worker.actions()[index]!;
			expect(envelope).toMatchObject({ name: "processKey", args: [input] });
			await panel.worker.settle(envelope, { result: passThroughResult });
		}
		// Enter on an empty composition is left to the browser and never reaches
		// the engine.
		await pressKey("Enter", "Enter");

		expect(panel.worker.actions()).toHaveLength(keys.length);
		expect(panel.textArea.value).toBe("  1");
		expect(panel.toast()).toBeNull();
	});

	it("still shows the error toast when the worker rejects the action", async () => {
		const panel = await mountPanelWithToasts();
		const consoleError = vi.spyOn(console, "error").mockImplementation(() => undefined);
		await pressKey(" ", "Space");
		await panel.worker.settle(panel.worker.actions()[0]!, { error: new Error("worker failed") });

		expect(consoleError).toHaveBeenCalledWith("YUNE_WORKER_ACTION_ERROR", expect.objectContaining({ action: "processKey" }));
		const toast = panel.toast();
		expect(toast).not.toBeNull();
		expect(toast!.classList.contains("yd-toast-error")).toBe(true);
		expect(toast!.textContent).toContain("執行操作時發生錯誤");
		expect(panel.textArea.value).toBe("");
	});
});

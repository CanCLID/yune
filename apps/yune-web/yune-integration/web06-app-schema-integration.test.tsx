// @vitest-environment jsdom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

import { DEFAULT_PREFERENCES, OUTPUT_STANDARD_ENGINE_OPTIONS } from "../src/consts.js";
import {
	web06DeployCacheSnapshotDigest,
	web06InjectedAssetManifestDigest,
	web06AsciiModeToggleActions,
	web06RetainedMappedAction,
	web06StableDigest,
	web06UserdbSnapshotDigest,
	web06ValuesEqual,
} from "../src/yune-integration/private-protocol.js";
import {
	WEB06_ACTION_OWNER,
	web06DeployPreferenceFanout,
	web06LiveOptionFanout,
	web06SchemaChangeFanout,
} from "../src/yune-integration/web06-app-action-map.js";

import type {
	Actions,
	Web06WorkerLifecycleEffect,
	Web06WorkerReceipt,
	Web06WorkerResultSummary,
	YuneDeployCacheSnapshot,
	YuneInjectedAssetManifest,
	YuneWebUserdbSnapshot,
} from "../src/types.js";

type WorkerMessage = Record<string, any>;

class AppWorker {
	static latest: AppWorker;
	readonly sent: WorkerMessage[] = [];
	#messageListener?: (event: { data: WorkerMessage }) => void;

	constructor(_url: string | URL) {
		AppWorker.latest = this;
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

interface AppRealm {
	root: Root;
	container: HTMLDivElement;
	worker: AppWorker;
	debug: {
		status(): Record<string, any>;
		snapshot(): Record<string, any>;
		resetReceipts(): void;
	};
	flushRafs(): Promise<void>;
}

interface AppActionLedgerRow {
	name: keyof Actions;
	args: unknown[];
	originKind: "dom-event" | "background";
	originReason: string;
	originOwner: string;
	eventActionIndex?: number;
	causedBy?: {
		name: keyof Actions;
		eventActionIndex: number;
		originOwner: string;
	};
}

let realm: AppRealm | undefined;

function projectAppActionLedger(actions: any[]): AppActionLedgerRow[] {
	const byId = new Map(actions.map(action => [action.identity.actionId, action]));
	return actions.map(action => {
		const identity = action.identity;
		const cause = identity.causedByActionId === undefined
			? undefined
			: byId.get(identity.causedByActionId);
		if (identity.originKind === "background" && cause === undefined) {
			throw new Error(`Background action ${identity.actionId} lacks an in-window causal action`);
		}
		if (cause !== undefined && (
			identity.causedBySequenceId !== cause.identity.sequenceId
			|| identity.causedByEventId !== cause.identity.eventId
			|| identity.causedByEventSequenceId !== cause.identity.eventSequenceId
		)) {
			throw new Error(`Background action ${identity.actionId} has an incoherent causal identity`);
		}
		return {
			name: action.name,
			args: action.args,
			originKind: identity.originKind,
			originReason: identity.originReason,
			originOwner: identity.originOwner,
			...(identity.eventActionIndex === undefined ? {} : {
				eventActionIndex: identity.eventActionIndex,
			}),
			...(cause === undefined ? {} : {
				causedBy: {
					name: cause.name,
					eventActionIndex: cause.identity.eventActionIndex,
					originOwner: cause.identity.originOwner,
				},
			}),
		};
	});
}

function assertExactAppActionLedger(
	actual: AppActionLedgerRow[],
	expected: AppActionLedgerRow[],
): void {
	if (actual.length !== expected.length) {
		throw new Error(`App action ledger count mismatch: expected ${expected.length}, got ${actual.length}`);
	}
	for (let index = 0; index < expected.length; index += 1) {
		if (!web06ValuesEqual(actual[index], expected[index])) {
			throw new Error(`App action ledger mismatch at index ${index}`);
		}
	}
}

const userdbSnapshot: YuneWebUserdbSnapshot = {
	schemaId: "jyut6ping3",
	dictionaryId: "jyut6ping3",
	path: "/rime/jyut6ping3.userdb.txt",
	exists: false,
	bytes: 0,
	updatedAt: null,
	rows: [],
	rawText: "",
	parseErrors: [],
};

const cacheSnapshot: YuneDeployCacheSnapshot = {
	schemaId: "jyut6ping3",
	dictionaryId: "jyut6ping3",
	cacheFresh: true,
	deployedSchemaExists: true,
	actualStamp: {
		version: 1,
		assetVersion: "web06-test",
		schemaId: "jyut6ping3",
		dictionaryId: "jyut6ping3",
		assetSignature: "asset",
		customConfigSignature: "custom",
	},
	expectedStamp: {
		version: 1,
		assetVersion: "web06-test",
		schemaId: "jyut6ping3",
		dictionaryId: "jyut6ping3",
		assetSignature: "asset",
		customConfigSignature: "custom",
	},
};

const assetManifest: YuneInjectedAssetManifest = {
	schemaId: "jyut6ping3",
	assets: [],
};

function timedEvent<T extends Event>(event: T): T {
	Object.defineProperty(event, "timeStamp", { configurable: true, value: performance.now() });
	return event;
}

function semanticEffect(name: keyof Actions, args: unknown[]): Exclude<Web06WorkerLifecycleEffect, { kind: "listener" }>["kind"] | undefined {
	switch (name) {
		case "setOption":
		case "selectSchema":
			return "engine-state";
		case "customize":
			return args[0] !== null
				&& typeof args[0] === "object"
				&& Object.keys(args[0] as object).some(key => key !== "enableAI")
				? "engine-persistence"
				: "engine-state";
		case "deploy":
		case "importUserdb":
		case "customizeValue":
			return "engine-persistence";
		case "invalidateDeployCache":
			return "cache-invalidation";
		case "getUserdbSnapshot":
		case "deployCacheSnapshot":
		case "injectedAssetsManifest":
			return "snapshot-read";
		default:
			return undefined;
	}
}

function actionResult(envelope: WorkerMessage): unknown {
	switch (envelope.name as keyof Actions) {
		case "setOption":
			return undefined;
		case "selectSchema":
		case "customize":
		case "customizeValue":
		case "deploy":
			return true;
		case "getUserdbSnapshot":
		case "importUserdb":
			return {
				...userdbSnapshot,
				schemaId: document.documentElement.dataset["yuneActiveSchema"] ?? "jyut6ping3",
			};
		case "deployCacheSnapshot":
		case "invalidateDeployCache":
			return {
				...cacheSnapshot,
				schemaId: document.documentElement.dataset["yuneActiveSchema"] ?? "jyut6ping3",
			};
		case "injectedAssetsManifest":
			return {
				...assetManifest,
				schemaId: document.documentElement.dataset["yuneActiveSchema"] ?? "jyut6ping3",
			};
		default:
			throw new Error(`Unexpected App integration action ${envelope.name}`);
	}
}

function resultSummary(name: keyof Actions, args: unknown[], result: unknown): Web06WorkerResultSummary {
	const effect = semanticEffect(name, args);
	const persistenceCompleted = effect === "engine-persistence";
	if (typeof result === "boolean") {
		return { kind: "boolean", resultDigest: web06StableDigest(result), success: result, persistenceCompleted };
	}
	if (result === undefined) {
		return { kind: "empty", resultDigest: web06StableDigest(null), success: true, persistenceCompleted };
	}
	if (name === "getUserdbSnapshot" || name === "importUserdb") {
		const snapshot = result as YuneWebUserdbSnapshot;
		const userdbDigest = web06UserdbSnapshotDigest(snapshot);
		return {
			kind: "userdb-snapshot",
			resultDigest: userdbDigest,
			success: true,
			persistenceCompleted,
			userdbDigest,
			userdbRowCount: snapshot.rows.length,
			userdbBytes: snapshot.bytes,
		};
	}
	if (name === "deployCacheSnapshot" || name === "invalidateDeployCache") {
		return {
			kind: "deploy-cache-snapshot",
			resultDigest: web06DeployCacheSnapshotDigest(result as YuneDeployCacheSnapshot),
			success: true,
			persistenceCompleted,
		};
	}
	return {
		kind: "asset-manifest",
		resultDigest: web06InjectedAssetManifestDigest(result as YuneInjectedAssetManifest),
		success: true,
		persistenceCompleted,
	};
}

function actionListeners(envelope: WorkerMessage): Array<{ name: string; args: unknown[] }> {
	switch (envelope.name as keyof Actions) {
		case "setOption":
			return [{ name: "optionChanged", args: envelope.args }];
		case "selectSchema":
			return [{ name: "schemaChanged", args: [envelope.args[0], `Schema ${envelope.args[0]}`] }];
		case "deploy":
			return [{ name: "deployStatusChanged", args: ["success"] }];
		default:
			return [];
	}
}

async function respond(worker: AppWorker, envelope: WorkerMessage) {
	const listeners = actionListeners(envelope);
	const result = actionResult(envelope);
	const summary = resultSummary(envelope.name, envelope.args, result);
	const at = performance.now();
	const lifecycleEffects: Web06WorkerLifecycleEffect[] = listeners.map(listener => ({
		kind: "listener",
		name: listener.name as never,
		argsDigest: web06StableDigest(listener.args),
		args: listener.args as never,
		recordedAt: at,
	}));
	const effect = semanticEffect(envelope.name, envelope.args);
	if (effect !== undefined) {
		lifecycleEffects.push({
			kind: effect,
			name: envelope.name,
			resultDigest: summary.resultDigest,
			recordedAt: at,
		});
	}
	const receipt: Web06WorkerReceipt = {
		workerMessageReceivedAt: at,
		workerActionStartedAt: at,
		workerFinishedAt: at,
		runtimeSpans: [],
		adapterSpans: [],
		persistenceSpans: [],
		collectorSpans: [],
		engineRaw: {
			availability: "not-applicable",
			action: envelope.name,
			reason: "action-has-no-runtime-response",
		},
		observerFailures: [],
		lifecycleEffects,
		resultSummary: summary,
	};
	await act(async () => {
		for (const listener of listeners) {
			worker.emit({
				type: "listener",
				name: listener.name,
				args: listener.args,
				web06: {
					protocolVersion: "web06-private-v1",
					actionId: envelope.identity.actionId,
					sequenceId: envelope.identity.sequenceId,
				},
			});
		}
		worker.emit({
			protocolVersion: "web06-private-v1",
			kind: "action-result",
			mode: envelope.mode,
			modeProvenance: envelope.modeProvenance,
			identity: structuredClone(envelope.identity),
			resultType: "success",
			result,
			receipt,
		});
		for (let index = 0; index < 4; index += 1) await Promise.resolve();
	});
}

async function settleApp(app: AppRealm, startIndex: number): Promise<number> {
	let processed = startIndex;
	let idleTurns = 0;
	for (let turn = 0; turn < 600; turn += 1) {
		const next = app.worker.actions()[processed];
		if (next !== undefined) {
			await respond(app.worker, next);
			processed += 1;
			idleTurns = 0;
			await app.flushRafs();
			continue;
		}
		await act(async () => {
			for (let index = 0; index < 6; index += 1) await Promise.resolve();
		});
		await app.flushRafs();
		if (
			app.worker.actions()[processed] === undefined
			&& app.debug.status().queueDepth === 0
			&& app.debug.status().pendingTerminalActions === 0
		) {
			idleTurns += 1;
			if (idleTurns >= 3) return processed;
		}
		else {
			idleTurns = 0;
		}
	}
	throw new Error(`App did not settle after ${processed - startIndex} actions`);
}

async function mountApp(): Promise<AppRealm> {
	vi.resetModules();
	window.history.replaceState({}, "", "/?yuneWeb06Mode=full");
	window.localStorage.clear();
	for (const key of Object.keys(document.documentElement.dataset)) {
		delete document.documentElement.dataset[key];
	}
	vi.stubGlobal("Worker", AppWorker);
	vi.stubGlobal("matchMedia", (query: string) => ({
		matches: false,
		media: query,
		onchange: null,
		addEventListener: () => undefined,
		removeEventListener: () => undefined,
		addListener: () => undefined,
		removeListener: () => undefined,
		dispatchEvent: () => false,
	}));
	Object.defineProperty(window, "matchMedia", { configurable: true, value: globalThis.matchMedia });
	(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
	const rafCallbacks: FrameRequestCallback[] = [];
	vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
		rafCallbacks.push(callback);
		return rafCallbacks.length;
	});
	const { default: App } = await import("../src/App.js");
	const container = document.createElement("div");
	document.body.append(container);
	const root = createRoot(container);
	await act(async () => {
		root.render(<App />);
		await Promise.resolve();
	});
	const mounted: AppRealm = {
		root,
		container,
		worker: AppWorker.latest,
		debug: (window as typeof window & { __YUNE_WEB06__: AppRealm["debug"] }).__YUNE_WEB06__,
		async flushRafs() {
			let count = 0;
			while (rafCallbacks.length > 0) {
				if (++count > 256) throw new Error("App rAF queue did not settle");
				const callback = rafCallbacks.shift()!;
				await act(async () => {
					callback(performance.now());
					await Promise.resolve();
				});
			}
		},
	};
	realm = mounted;
	return mounted;
}

async function prepareAppMeasurement(): Promise<{ app: AppRealm; processed: number }> {
	const app = await mountApp();
	await act(async () => {
		app.worker.emit({ type: "listener", name: "schemaChanged", args: ["jyut6ping3", "Jyutping"] });
		app.worker.emit({ type: "listener", name: "initialized", args: [true, undefined] });
		for (let index = 0; index < 4; index += 1) await Promise.resolve();
	});
	const processed = await settleApp(app, 0);
	const startup = app.debug.snapshot();
	expect(startup.invalidations, JSON.stringify(startup.invalidations, null, 2)).toEqual([]);
	app.debug.resetReceipts();
	expect(app.debug.snapshot().actions).toEqual([]);
	return { app, processed };
}

function assertExactControlFanout(
	app: AppRealm,
	snapshot: Record<string, any>,
	postResetActionStart: number,
	reason: string,
	expected: Array<{ owner: string; action: any }>,
): void {
	const event = snapshot.events.find((candidate: any) => candidate.identity.reason === reason);
	if (event === undefined) throw new Error(`Missing ${reason} event`);
	const retainedExpected = expected.map(item => web06RetainedMappedAction("full", item.action));
	expect(event.mappedActions.map((action: any) => ({
		name: action.name,
		args: action.args,
		boundary: action.boundary,
	}))).toEqual(retainedExpected.map(action => ({
		name: action.name,
		args: action.args,
		boundary: action.boundary,
	})));
	expect(event.linkedActionIds).toHaveLength(expected.length);
	expect(new Set(event.linkedActionIds).size).toBe(expected.length);
	const receiptsById = new Map(snapshot.actions.map((action: any) => [action.identity.actionId, action]));
	const envelopes = app.worker.actions().slice(postResetActionStart);
	const envelopesById = new Map(envelopes.map(action => [action.identity.actionId, action]));
	expect(envelopes.map(action => action.identity.actionId)).toEqual(
		snapshot.actions.map((action: any) => action.identity.actionId),
	);
	expect(event.linkedActionIds.map((actionId: string) => {
		const receipt: any = receiptsById.get(actionId);
		const envelope = envelopesById.get(actionId);
		return {
			name: receipt?.name,
			retainedArgs: receipt?.args,
			workerArgs: envelope?.args,
			originOwner: receipt?.identity.originOwner,
			originReason: receipt?.identity.originReason,
			lifecycle: receipt?.lifecycle?.outcome,
		};
	})).toEqual(expected.map((item, index) => ({
		name: item.action.name,
		retainedArgs: retainedExpected[index]!.args,
		workerArgs: item.action.args,
		originOwner: item.owner,
		originReason: reason,
		lifecycle: "barrier-completed",
	})));
	expect(snapshot.actions.every((action: any) =>
		Number(action.presentation !== undefined) + Number(action.lifecycle !== undefined) === 1
		&& action.presentation?.outcome !== "failure"
		&& action.lifecycle?.outcome !== "failure"
	)).toBe(true);
	expect(snapshot.status, JSON.stringify(snapshot.invalidations, null, 2)).toMatchObject({
		valid: true,
		queueDepth: 0,
		pendingFanoutActions: 0,
		pendingTerminalActions: 0,
	});
	expect(snapshot.invalidations, JSON.stringify(snapshot.invalidations, null, 2)).toEqual([]);
}

afterEach(async () => {
	if (realm !== undefined) {
		await act(async () => realm?.root.unmount());
		realm.container.remove();
		realm = undefined;
	}
	delete (window as typeof window & { __YUNE_WEB06__?: unknown }).__YUNE_WEB06__;
	vi.unstubAllGlobals();
	vi.restoreAllMocks();
	vi.resetModules();
});

describe("WEB-06 App control integration", () => {
	it("dispatches the frozen cross-effect schema/deploy fanout once in exact order with complete terminals", async () => {
		const prepared = await prepareAppMeasurement();
		const { app } = prepared;
		let { processed } = prepared;
		const postResetActionStart = processed;

		const expected = web06SchemaChangeFanout({
			nextSchema: "luna_pinyin",
			deployPreferences: {
				pageSize: DEFAULT_PREFERENCES.pageSize,
				enableCompletion: DEFAULT_PREFERENCES.enableCompletion,
				enableCorrection: DEFAULT_PREFERENCES.enableCorrection,
				enableSentence: DEFAULT_PREFERENCES.enableSentence,
				enableLearning: DEFAULT_PREFERENCES.enableLearning,
				combineCandidates: DEFAULT_PREFERENCES.combineCandidates,
				predictionNeverFirst: DEFAULT_PREFERENCES.predictionNeverFirst,
				predictionThreshold: DEFAULT_PREFERENCES.predictionThreshold,
				dictionaryExclude: DEFAULT_PREFERENCES.dictionaryExclude,
				isCangjie5: DEFAULT_PREFERENCES.isCangjie5,
			},
			liveOptions: {
				isAsciiMode: DEFAULT_PREFERENCES.isAsciiMode,
				isFullShape: DEFAULT_PREFERENCES.isFullShape,
				outputStandard: DEFAULT_PREFERENCES.outputStandard,
				activeSchema: "luna_pinyin",
				isExtendedCharset: DEFAULT_PREFERENCES.isExtendedCharset,
				isDisabled: DEFAULT_PREFERENCES.isDisabled,
			},
			applyDeployPreferences: true,
		});
		expect(expected).toHaveLength(3 + OUTPUT_STANDARD_ENGINE_OPTIONS.length + 6);
		const schemaSelect = app.container.querySelector<HTMLSelectElement>("#yune-schema-select")!;
		schemaSelect.value = "luna_pinyin";
		await act(async () => {
			schemaSelect.dispatchEvent(timedEvent(new Event("change", { bubbles: true, cancelable: true })));
			for (let index = 0; index < 4; index += 1) await Promise.resolve();
		});
		processed = await settleApp(app, processed);
		const snapshot = app.debug.snapshot();
		const schemaEvent = snapshot.events.find((event: any) => event.identity.reason === "schema-select-change");
		expect(schemaEvent).toBeDefined();
		const retainedExpected = expected.map(item => web06RetainedMappedAction("full", item.action));
		expect(schemaEvent.mappedActions.map((action: any) => ({
			name: action.name,
			args: action.args,
			boundary: action.boundary,
		}))).toEqual(retainedExpected.map(action => ({
			name: action.name,
			args: action.args,
			boundary: action.boundary,
		})));
		expect(schemaEvent.linkedActionIds).toHaveLength(expected.length);
		expect(new Set(schemaEvent.linkedActionIds).size).toBe(expected.length);
		const byId = new Map(snapshot.actions.map((action: any) => [action.identity.actionId, action]));
		const linked = schemaEvent.linkedActionIds.map((actionId: string) => byId.get(actionId));
		expect(snapshot.invalidations, JSON.stringify(snapshot.invalidations, null, 2)).toEqual([]);
		expect(linked.map((action: any) => ({ name: action.name, args: action.args }))).toEqual(
			retainedExpected.map(action => ({ name: action.name, args: action.args })),
		);
		const envelopeById = new Map(app.worker.actions().map(action => [action.identity.actionId, action]));
		expect(schemaEvent.linkedActionIds.map((actionId: string) => {
			const action = envelopeById.get(actionId);
			return { name: action.name, args: action.args };
		})).toEqual(expected.map(item => ({ name: item.action.name, args: item.action.args })));
		expect(linked.map((action: any) => ({
			name: action?.name,
			presentation: action?.presentation?.outcome,
			lifecycle: action?.lifecycle?.outcome,
		}))).toEqual(expected.map(item => ({
			name: item.action.name,
			presentation: undefined,
			lifecycle: "barrier-completed",
		})));

		const postResetEnvelopes = app.worker.actions().slice(postResetActionStart);
		expect(postResetEnvelopes.map(action => action.identity.actionId)).toEqual(
			snapshot.actions.map((action: any) => action.identity.actionId),
		);
		const plannedRow = (eventActionIndex: number): AppActionLedgerRow => ({
			name: retainedExpected[eventActionIndex]!.name,
			args: retainedExpected[eventActionIndex]!.args,
			originKind: "dom-event",
			originReason: "schema-select-change",
			originOwner: expected[eventActionIndex]!.owner,
			eventActionIndex,
		});
		const backgroundRow = (
			name: keyof Actions,
			args: unknown[],
			originOwner: string,
			originReason: string,
			causeEventActionIndex: number,
		): AppActionLedgerRow => ({
			name,
			args,
			originKind: "background",
			originReason,
			originOwner,
			causedBy: {
				name: retainedExpected[causeEventActionIndex]!.name,
				eventActionIndex: causeEventActionIndex,
				originOwner: expected[causeEventActionIndex]!.owner,
			},
		});
		const liveEventIndexes = [2, ...Array.from({ length: 11 }, (_, index) => index + 4)];
		const backgroundLiveRow = (ordinal: number, causeEventActionIndex: number) => {
			const eventActionIndex = liveEventIndexes[ordinal]!;
			const action = retainedExpected[eventActionIndex]!;
			return backgroundRow(
				action.name,
				action.args,
				`${WEB06_ACTION_OWNER.liveOptions}:background`,
				"live-options-effect",
				causeEventActionIndex,
			);
		};
		const exactLedger: AppActionLedgerRow[] = [
			plannedRow(0),
			plannedRow(1),
			plannedRow(2),
			backgroundRow(
				"getUserdbSnapshot",
				[],
				WEB06_ACTION_OWNER.userdb,
				"causal-userdb-refresh",
				0,
			),
			backgroundRow("deployCacheSnapshot", [], "control-snapshot-background", "control-snapshot-refresh", 0),
			backgroundRow("injectedAssetsManifest", [], "control-snapshot-background", "control-snapshot-refresh", 0),
			backgroundRow("setOption", ["ascii_punct", false], "rime-option:ascii_punct", "rime-option-effect", 0),
			backgroundLiveRow(0, 0),
			plannedRow(3),
			plannedRow(4),
			backgroundLiveRow(1, 0),
			backgroundRow("deployCacheSnapshot", [], "control-snapshot-background", "control-snapshot-refresh", 3),
			backgroundRow("injectedAssetsManifest", [], "control-snapshot-background", "control-snapshot-refresh", 3),
			backgroundRow("setOption", ["ascii_punct", false], "rime-option:ascii_punct", "rime-option-effect", 3),
			backgroundLiveRow(0, 3),
		];
		for (let ordinal = 2; ordinal < liveEventIndexes.length; ordinal += 1) {
			exactLedger.push(
				plannedRow(liveEventIndexes[ordinal]!),
				backgroundLiveRow(ordinal, 0),
				backgroundLiveRow(ordinal - 1, 3),
			);
		}
		exactLedger.push(
			backgroundLiveRow(liveEventIndexes.length - 1, 3),
			backgroundRow(
				"getUserdbSnapshot",
				[],
				WEB06_ACTION_OWNER.userdb,
				"causal-userdb-refresh",
				3,
			),
		);
		expect(exactLedger).toHaveLength(47);
		const actualLedger = projectAppActionLedger(snapshot.actions);
		assertExactAppActionLedger(actualLedger, exactLedger);
		for (const duplicateIndex of [
			actualLedger.findIndex(row => row.originReason === "live-options-effect"),
			actualLedger.findIndex(row => row.name === "deployCacheSnapshot"),
			actualLedger.findIndex(row => row.name === "getUserdbSnapshot"),
		]) {
			const mutated = structuredClone(actualLedger);
			mutated.splice(duplicateIndex, 0, structuredClone(mutated[duplicateIndex]!));
			expect(() => assertExactAppActionLedger(mutated, exactLedger)).toThrow("action ledger count mismatch");
		}
		expect(snapshot.actions.every((action: any) =>
			Number(action.presentation !== undefined) + Number(action.lifecycle !== undefined) === 1
			&& action.presentation?.outcome !== "failure"
			&& action.lifecycle?.outcome !== "failure"
		)).toBe(true);
		expect(snapshot.status).toMatchObject({
			valid: true,
			queueDepth: 0,
			pendingFanoutActions: 0,
			pendingTerminalActions: 0,
		});
		expect(snapshot.invalidations, JSON.stringify(snapshot.invalidations, null, 2)).toEqual([]);
	});

	it("consumes a real keyboard Shift live-option fanout exactly once", async () => {
		const { app, processed } = await prepareAppMeasurement();
		const textArea = app.container.querySelector<HTMLTextAreaElement>("textarea")!;
		textArea.focus();
		await act(async () => {
			document.dispatchEvent(timedEvent(new KeyboardEvent("keydown", {
				key: "Shift",
				code: "ShiftLeft",
				shiftKey: true,
				bubbles: true,
				cancelable: true,
			})));
			document.dispatchEvent(timedEvent(new KeyboardEvent("keyup", {
				key: "Shift",
				code: "ShiftLeft",
				bubbles: true,
				cancelable: true,
			})));
			for (let index = 0; index < 4; index += 1) await Promise.resolve();
		});
		await settleApp(app, processed);
		const expected = web06AsciiModeToggleActions({
			nextAsciiMode: true,
			isFullShape: DEFAULT_PREFERENCES.isFullShape,
			outputStandard: DEFAULT_PREFERENCES.outputStandard,
			activeSchema: DEFAULT_PREFERENCES.activeSchema,
			isExtendedCharset: DEFAULT_PREFERENCES.isExtendedCharset,
			isDisabled: DEFAULT_PREFERENCES.isDisabled,
		}).map(action => ({ owner: WEB06_ACTION_OWNER.liveOptions, action }));
		const snapshot = app.debug.snapshot();
		assertExactControlFanout(app, snapshot, processed, "ascii-mode-shift-tap", expected);
		expect(snapshot.actions).toHaveLength(expected.length + 1);
		const shiftEvent = snapshot.events.find((event: any) => event.identity.reason === "ascii-mode-shift-tap");
		const linkedIds = new Set(shiftEvent.linkedActionIds);
		const background = snapshot.actions.filter((action: any) => !linkedIds.has(action.identity.actionId));
		expect(background).toHaveLength(1);
		expect(background[0]).toMatchObject({
			name: "getUserdbSnapshot",
			identity: {
				originKind: "background",
				originOwner: WEB06_ACTION_OWNER.userdb,
				originReason: "causal-userdb-refresh",
				causedByActionId: shiftEvent.linkedActionIds.at(-1),
			},
			lifecycle: { outcome: "barrier-completed" },
		});
	});

	it("consumes a real Cangjie deploy-preference fanout exactly once", async () => {
		const { app, processed } = await prepareAppMeasurement();
		const cangjie3 = app.container.querySelector<HTMLInputElement>(
			'input[name="cangjieVersion"][value="false"]',
		)!;
		await act(async () => {
			const timestampDescriptor = Object.getOwnPropertyDescriptor(Event.prototype, "timeStamp");
			Object.defineProperty(Event.prototype, "timeStamp", {
				configurable: true,
				get: () => performance.now(),
			});
			try {
				cangjie3.click();
			}
			finally {
				if (timestampDescriptor === undefined) delete (Event.prototype as { timeStamp?: number }).timeStamp;
				else Object.defineProperty(Event.prototype, "timeStamp", timestampDescriptor);
			}
			for (let index = 0; index < 4; index += 1) await Promise.resolve();
		});
		await settleApp(app, processed);
		const expected = web06DeployPreferenceFanout({
			pageSize: DEFAULT_PREFERENCES.pageSize,
			enableCompletion: DEFAULT_PREFERENCES.enableCompletion,
			enableCorrection: DEFAULT_PREFERENCES.enableCorrection,
			enableSentence: DEFAULT_PREFERENCES.enableSentence,
			enableLearning: DEFAULT_PREFERENCES.enableLearning,
			combineCandidates: DEFAULT_PREFERENCES.combineCandidates,
			predictionNeverFirst: DEFAULT_PREFERENCES.predictionNeverFirst,
			predictionThreshold: DEFAULT_PREFERENCES.predictionThreshold,
			dictionaryExclude: DEFAULT_PREFERENCES.dictionaryExclude,
			isCangjie5: false,
		});
		const snapshot = app.debug.snapshot();
		assertExactControlFanout(app, snapshot, processed, "toolbar-cangjie-version-change", expected);
		const retainedExpected = expected.map(item => web06RetainedMappedAction("full", item.action));
		const plannedRow = (eventActionIndex: number): AppActionLedgerRow => ({
			name: retainedExpected[eventActionIndex]!.name,
			args: retainedExpected[eventActionIndex]!.args,
			originKind: "dom-event",
			originReason: "toolbar-cangjie-version-change",
			originOwner: expected[eventActionIndex]!.owner,
			eventActionIndex,
		});
		const backgroundRow = (
			name: keyof Actions,
			args: unknown[],
			originOwner: string,
			originReason: string,
		): AppActionLedgerRow => ({
			name,
			args,
			originKind: "background",
			originReason,
			originOwner,
			causedBy: {
				name: retainedExpected[1]!.name,
				eventActionIndex: 1,
				originOwner: expected[1]!.owner,
			},
		});
		const retainedLiveOptions = web06LiveOptionFanout({
			isAsciiMode: DEFAULT_PREFERENCES.isAsciiMode,
			isFullShape: DEFAULT_PREFERENCES.isFullShape,
			outputStandard: DEFAULT_PREFERENCES.outputStandard,
			activeSchema: DEFAULT_PREFERENCES.activeSchema,
			isExtendedCharset: DEFAULT_PREFERENCES.isExtendedCharset,
			isDisabled: DEFAULT_PREFERENCES.isDisabled,
		}).map(item => web06RetainedMappedAction("full", item.action));
		const liveOptionRow = (index: number) => backgroundRow(
			retainedLiveOptions[index]!.name,
			retainedLiveOptions[index]!.args,
			`${WEB06_ACTION_OWNER.liveOptions}:background`,
			"live-options-effect",
		);
		const exactLedger: AppActionLedgerRow[] = [
			plannedRow(0),
			plannedRow(1),
			backgroundRow("deployCacheSnapshot", [], "control-snapshot-background", "control-snapshot-refresh"),
			backgroundRow("injectedAssetsManifest", [], "control-snapshot-background", "control-snapshot-refresh"),
			backgroundRow("setOption", ["ascii_punct", false], "rime-option:ascii_punct", "rime-option-effect"),
			liveOptionRow(0),
			backgroundRow("getUserdbSnapshot", [], WEB06_ACTION_OWNER.userdb, "causal-userdb-refresh"),
			...retainedLiveOptions.slice(1).map((_, index) => liveOptionRow(index + 1)),
			backgroundRow("getUserdbSnapshot", [], WEB06_ACTION_OWNER.userdb, "causal-userdb-refresh"),
		];
		expect(exactLedger).toHaveLength(19);
		const actualLedger = projectAppActionLedger(snapshot.actions);
		assertExactAppActionLedger(actualLedger, exactLedger);
		for (const duplicateIndex of [0, 2, 5, 6]) {
			const mutated = structuredClone(actualLedger);
			mutated.splice(duplicateIndex, 0, structuredClone(mutated[duplicateIndex]!));
			expect(() => assertExactAppActionLedger(mutated, exactLedger)).toThrow("action ledger count mismatch");
		}
	});
});

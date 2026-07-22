import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { FakeYuneWebFilesystem } from "../../../packages/yune-web-runtime/test/fake-filesystem.js";
import { FakeYuneWebModule } from "../../../packages/yune-web-runtime/test/fake-module.js";
import {
	WEB06_INJECTED_ERROR_CONFIG_ID,
	WEB06_INJECTED_ERROR_KEY,
	WEB06_INJECTED_ERROR_VALUE,
} from "../src/yune-integration/private-protocol.js";
import {
	createWeb06WorkerProtocol,
	invokeWeb06PublicAction,
} from "../src/yune-integration/web06-worker-protocol.js";
import {
	logWeb06ActionError,
	logWeb06NativeMessage,
	logWeb06PersistenceDiagnostic,
	logWeb06RuntimeInitializationFailure,
	shouldLogWeb06ProductDebug,
	type Web06ProductConsoleLane,
} from "../src/yune-integration/web06-product-console.js";

import type {
	Actions,
	Web06ActionIdentity,
	Web06CollectionMode,
	Web06CollectionModeProvenance,
	Web06WorkerActionEnvelope,
} from "../src/types.js";

vi.mock("@yune-ime/yune-web-runtime", async () => {
	return await import("../../../packages/yune-web-runtime/src/index.ts");
});

const assets = {
	defaultYaml: "config_version: yune-web\nschema_list:\n  - schema: luna_pinyin\n",
	schemaYaml: "schema:\n  schema_id: luna_pinyin\ntranslator:\n  dictionary: luna_pinyin\n",
	dictionaryYaml: "---\nname: luna_pinyin\n...\nni\t你\t1\n",
};

const initOptions = {
	sharedDataDir: "/usr/share/rime-data",
	userDataDir: "/rime",
	schemaId: "luna_pinyin",
};

function identity(sequenceId: number, boundary: Web06ActionIdentity["boundary"]): Web06ActionIdentity {
	return {
		protocolVersion: "web06-private-v1",
		actionId: `web06-action-${String(sequenceId).padStart(8, "0")}`,
		sequenceId,
		compositionEpochId: sequenceId,
		supersessionSubRunId: sequenceId,
		actionClass: "stateful-barrier",
		supersedable: false,
		boundary,
		rawInputSequence: [],
		originKind: "background",
		originReason: "worker-real-path-test",
		actionEnqueuedAt: sequenceId,
		mainQueueDepthAtEnqueue: 1,
		workerSentAt: sequenceId,
		workerDispatchDepth: 1,
	};
}

function envelope(
	mode: Web06CollectionMode,
	modeProvenance: Web06CollectionModeProvenance,
	sequenceId: number,
	name: keyof Actions,
	args: unknown[],
	boundary: Web06ActionIdentity["boundary"],
): Web06WorkerActionEnvelope {
	return {
		protocolVersion: "web06-private-v1",
		kind: "action",
		mode,
		modeProvenance,
		identity: identity(sequenceId, boundary),
		name,
		args,
	};
}

afterEach(async () => {
	const { cleanupYuneRuntime } = await import("../src/yune-integration/adapter.js");
	cleanupYuneRuntime();
	vi.restoreAllMocks();
});

beforeEach(() => {
	vi.spyOn(console, "info").mockImplementation(() => undefined);
	vi.spyOn(console, "warn").mockImplementation(() => undefined);
	vi.spyOn(console, "error").mockImplementation(() => undefined);
	vi.spyOn(console, "log").mockImplementation(() => undefined);
});

describe("WEB-06 worker/adapter action seam", () => {
	it("preserves the exact product persistence console lane in every instrumented mode", async () => {
		const adapter = await import("../src/yune-integration/adapter.js");
		const observations: Array<{ label: string; result: boolean; persistenceLogs: string[] }> = [];
		for (const lane of [
			{ label: "product/no-probe", mode: undefined, provenance: undefined },
			{ label: "selector-omitted minimal", mode: "minimal", provenance: "instrumented-default-minimal" },
			{ label: "explicit minimal", mode: "minimal", provenance: "instrumented-explicit-minimal" },
			{ label: "explicit full", mode: "full", provenance: "instrumented-explicit-full" },
		] as const) {
			adapter.cleanupYuneRuntime();
			let now = 10;
			const protocol = lane.mode === undefined ? undefined : createWeb06WorkerProtocol({
				mode: lane.mode,
				modeProvenance: lane.provenance,
				now: () => now++,
				postDiagnostic: () => undefined,
			});
			const info = vi.spyOn(console, "info").mockImplementation(() => undefined);
			await adapter.initYuneRuntime(
				new FakeYuneWebModule(),
				new FakeYuneWebFilesystem(),
				initOptions,
				assets,
				"luna_pinyin",
				[],
				false,
				"web06-worker-persistence-console-test",
				protocol?.adapterObservation,
			);
			info.mockClear();
			const result = await adapter.customizeValue(
				"luna_pinyin",
				"translator/enable_completion",
				"true",
			);
			observations.push({
				label: lane.label,
				result,
				persistenceLogs: info.mock.calls
					.map(args => String(args[0]))
					.filter(message => message.startsWith("YUNE_PERSISTENCE ")),
			});
			info.mockRestore();
		}

		expect(observations.map(({ label, result }) => ({ label, result }))).toEqual([
			{ label: "product/no-probe", result: true },
			{ label: "selector-omitted minimal", result: true },
			{ label: "explicit minimal", result: true },
			{ label: "explicit full", result: true },
		]);
		expect(observations.every(observation => observation.persistenceLogs.length === 2)).toBe(true);
		const normalize = (logs: string[]) => logs.map(message => {
			const diagnostic = JSON.parse(message.slice("YUNE_PERSISTENCE ".length)) as Record<string, unknown>;
			delete diagnostic["timestamp"];
			return diagnostic;
		});
		const productLogs = normalize(observations[0]!.persistenceLogs);
		expect(observations.slice(1).map(observation => normalize(observation.persistenceLogs))).toEqual([
			productLogs,
			productLogs,
			productLogs,
		]);
	});

	it("keeps every product console call exact across PRODUCT and all instrumented lanes while the collector adds none", () => {
		const observations: Array<{
			label: string;
			collectorCalls: number;
			debugDecisions: boolean[];
			calls: Array<{ method: string; args: unknown[] }>;
		}> = [];
		for (const lane of [
			{ label: "product/no-probe", consoleLane: "product/no-probe", mode: undefined, provenance: undefined },
			{ label: "selector-omitted minimal", consoleLane: "minimal", mode: "minimal", provenance: "instrumented-default-minimal" },
			{ label: "explicit minimal", consoleLane: "minimal", mode: "minimal", provenance: "instrumented-explicit-minimal" },
			{ label: "explicit full", consoleLane: "full", mode: "full", provenance: "instrumented-explicit-full" },
		] as const satisfies ReadonlyArray<{
			label: string;
			consoleLane: Web06ProductConsoleLane;
			mode: Web06CollectionMode | undefined;
			provenance: Web06CollectionModeProvenance | undefined;
		}>) {
			const info = vi.spyOn(console, "info").mockImplementation(() => undefined);
			const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
			const error = vi.spyOn(console, "error").mockImplementation(() => undefined);
			const log = vi.spyOn(console, "log").mockImplementation(() => undefined);
			let now = 10;
			if (lane.mode !== undefined && lane.provenance !== undefined) {
				const protocol = createWeb06WorkerProtocol({
					mode: lane.mode,
					modeProvenance: lane.provenance,
					now: () => now++,
					postDiagnostic: () => undefined,
				});
				const session = protocol.beginAction(
					envelope(lane.mode, lane.provenance, 1, "setOption", ["yune_inspector", true], "option"),
					now++,
					now++,
				);
				protocol.finalizeSuccess(session, undefined);
				protocol.finishAction(session, now++);
			}
			const collectorCalls = info.mock.calls.length + warn.mock.calls.length + error.mock.calls.length + log.mock.calls.length;

			const persistence = { phase: "sync:pass", reason: "customize-value", timestamp: "frozen" };
			const startupError = new Error("frozen startup failure");
			const actionError = { action: "setOption", args: ["ascii_mode", true], error: { name: "Error", message: "frozen action failure" } };
			logWeb06PersistenceDiagnostic(lane.consoleLane, persistence);
			logWeb06RuntimeInitializationFailure(lane.consoleLane, startupError);
			logWeb06NativeMessage(lane.consoleLane, "I0722 00:00:00 worker.cc:1 native info", false, "");
			logWeb06NativeMessage(lane.consoleLane, "W0722 00:00:00 worker.cc:1 native warning", false, "");
			logWeb06NativeMessage(lane.consoleLane, "E0722 00:00:00 worker.cc:1 native error", false, "");
			logWeb06NativeMessage(lane.consoleLane, "raw native error", false, "");
			logWeb06NativeMessage(lane.consoleLane, "W0722 00:00:00 worker.cc:1 hidden public warning", true, "");
			const debugDecisions = [
				shouldLogWeb06ProductDebug(lane.consoleLane, false, ""),
				shouldLogWeb06ProductDebug(lane.consoleLane, false, "?debug"),
				shouldLogWeb06ProductDebug(lane.consoleLane, true, ""),
			];
			if (debugDecisions[1]) {
				console.log("post", JSON.stringify({ name: "setOption", args: ["ascii_mode", true] }));
				console.log("receive", JSON.stringify({ type: "success" }));
				console.info("diagnostic", JSON.stringify({ source: "yune" }));
			}
			logWeb06ActionError(lane.consoleLane, actionError);

			const normalize = (value: unknown): unknown => value instanceof Error
				? { name: value.name, message: value.message }
				: value;
			const calls = [
				...info.mock.calls.map((args, index) => ({ method: "info", args: args.map(normalize), order: info.mock.invocationCallOrder[index]! })),
				...warn.mock.calls.map((args, index) => ({ method: "warn", args: args.map(normalize), order: warn.mock.invocationCallOrder[index]! })),
				...error.mock.calls.map((args, index) => ({ method: "error", args: args.map(normalize), order: error.mock.invocationCallOrder[index]! })),
				...log.mock.calls.map((args, index) => ({ method: "log", args: args.map(normalize), order: log.mock.invocationCallOrder[index]! })),
			]
				.sort((left, right) => left.order - right.order)
				.map(({ method, args }) => ({ method, args }));
			observations.push({ label: lane.label, collectorCalls, debugDecisions, calls });
			info.mockRestore();
			warn.mockRestore();
			error.mockRestore();
			log.mockRestore();
		}

		expect(observations.every(observation => observation.collectorCalls === 0)).toBe(true);
		expect(observations.map(observation => observation.debugDecisions)).toEqual([
			[false, true, true],
			[false, true, true],
			[false, true, true],
			[false, true, true],
		]);
		const productCalls = observations[0]!.calls;
		expect(productCalls).toEqual([
			{ method: "info", args: ["YUNE_PERSISTENCE {\"phase\":\"sync:pass\",\"reason\":\"customize-value\",\"timestamp\":\"frozen\"}"] },
			{ method: "error", args: ["Yune runtime initialization failed", { name: "Error", message: "frozen startup failure" }] },
			{ method: "info", args: ["[native info"] },
			{ method: "warn", args: ["[native warning"] },
			{ method: "error", args: ["[native error"] },
			{ method: "error", args: ["raw native error"] },
			{ method: "log", args: ["post", "{\"name\":\"setOption\",\"args\":[\"ascii_mode\",true]}"] },
			{ method: "log", args: ["receive", "{\"type\":\"success\"}"] },
			{ method: "info", args: ["diagnostic", "{\"source\":\"yune\"}"] },
			{ method: "error", args: ["YUNE_WORKER_ACTION_ERROR", { action: "setOption", args: ["ascii_mode", true], error: { name: "Error", message: "frozen action failure" } }] },
		]);
		expect(observations.slice(1).map(observation => observation.calls)).toEqual([
			productCalls,
			productCalls,
			productCalls,
		]);
	});

	it("keeps the real frozen validation error and FIFO recovery identical in product, minimal, and full modes", async () => {
		const adapter = await import("../src/yune-integration/adapter.js");
		const observations: unknown[] = [];
		for (const lane of [
			{ label: "product/no-probe", mode: undefined, provenance: undefined },
			{ label: "minimal", mode: "minimal", provenance: "instrumented-explicit-minimal" },
			{ label: "full", mode: "full", provenance: "instrumented-explicit-full" },
		] as const) {
			adapter.cleanupYuneRuntime();
			let now = 10;
			const protocol = lane.mode === undefined ? undefined : createWeb06WorkerProtocol({
				mode: lane.mode,
				modeProvenance: lane.provenance,
				now: () => now++,
				postDiagnostic: () => undefined,
			});
			const module = new FakeYuneWebModule();
			const fs = new FakeYuneWebFilesystem();
			await adapter.initYuneRuntime(
				module,
				fs,
				initOptions,
				assets,
				"luna_pinyin",
				[],
				false,
				"web06-worker-real-path-test",
				protocol?.adapterObservation,
			);
			const publicCalls: Array<{ name: keyof Actions; args: unknown[] }> = [];
			const actions = {
				async customizeValue(configId: string, key: string, value: string) {
					publicCalls.push({ name: "customizeValue", args: [configId, key, value] });
					return adapter.customizeValue(configId, key, value);
				},
				async setOption(option: string, value: boolean) {
					publicCalls.push({ name: "setOption", args: [option, value] });
					return adapter.setOption(option, value);
				},
			} as unknown as Actions;
			const nativeBefore = module.callTrace();
			const freesBefore = module.freedResponses();
			const results: Array<Record<string, unknown>> = [];

			const run = async (request: Web06WorkerActionEnvelope) => {
				const session = protocol?.beginAction(request, now++, now++);
				try {
					const result = await invokeWeb06PublicAction(actions, request.name, request.args);
					if (session !== undefined) protocol.finalizeSuccess(session, result);
					results.push({ name: request.name, resultType: "success", result });
				}
				catch (error) {
					if (session !== undefined) protocol?.finalizeError(session);
					results.push({
						name: request.name,
						resultType: "error",
						error: error instanceof Error ? { name: error.name, message: error.message } : error,
					});
				}
				finally {
					if (session !== undefined) protocol?.finishAction(session, now++);
				}
				return session?.receipt;
			};

			const failureRequest = envelope(
				lane.mode ?? "minimal",
				lane.provenance ?? "instrumented-default-minimal",
				1,
				"customizeValue",
				[WEB06_INJECTED_ERROR_CONFIG_ID, WEB06_INJECTED_ERROR_KEY, WEB06_INJECTED_ERROR_VALUE],
				"option",
			);
			const failureReceipt = await run(failureRequest);
			expect(module.callTrace()).toEqual(nativeBefore);
			expect(module.freedResponses()).toEqual(freesBefore);

			const recoveryRequest = envelope(
				lane.mode ?? "minimal",
				lane.provenance ?? "instrumented-default-minimal",
				2,
				"setOption",
				["ascii_mode", false],
				"option",
			);
			const recoveryReceipt = await run(recoveryRequest);
			const nativeDelta = module.callTrace().slice(nativeBefore.length);
			expect(nativeDelta).toEqual([
				{ symbol: "yune_web_set_option", args: [1, "ascii_mode", 0] },
			]);
			expect(module.freedResponses()).toEqual(freesBefore);
			observations.push({
				label: lane.label,
				publicCalls,
				results,
				failureRaw: failureReceipt?.engineRaw,
				recoverySummary: recoveryReceipt?.resultSummary,
				failureObserverFailures: failureReceipt?.observerFailures ?? [],
				recoveryObserverFailures: recoveryReceipt?.observerFailures ?? [],
			});
		}

		const publicContract = (observation: any) => ({
			publicCalls: observation.publicCalls,
			results: observation.results,
		});
		expect(observations.map(publicContract)).toEqual(new Array(3).fill({
			publicCalls: [
				{ name: "customizeValue", args: ["", "", WEB06_INJECTED_ERROR_VALUE] },
				{ name: "setOption", args: ["ascii_mode", false] },
			],
			results: [
				{
					name: "customizeValue",
					resultType: "error",
					error: { name: "Error", message: "Yune customizeValue requires a config ID and key" },
				},
				{ name: "setOption", resultType: "success", result: undefined },
			],
		}));
		expect(observations.slice(1).every((observation: any) =>
			observation.failureObserverFailures.length === 0
			&& observation.recoveryObserverFailures.length === 0
			&& observation.recoverySummary?.kind === "empty"
			&& observation.recoverySummary?.success === true
		)).toBe(true);
		expect((observations[1] as any).failureRaw).toMatchObject({
			availability: "not-collected",
			action: "customizeValue",
		});
		expect((observations[2] as any).failureRaw).toEqual({
			availability: "not-applicable",
			action: "customizeValue",
			reason: "action-failed-before-runtime-response",
		});
	});
});

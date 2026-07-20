import { useCallback, useEffect, useReducer, useRef, useState } from "react";

import CandidatePanel from "./CandidatePanel";
import { NO_AUTO_FILL, OUTPUT_STANDARD_ENGINE_OPTIONS, normalizeOutputStandard, outputOptionForStandard } from "./consts";
import { useLoading, usePreferences, useRimeOption } from "./hooks";
import Preferences from "./Preferences";
import Rime, {
	declareWeb06ControlFanout,
	subscribe,
	withWeb06ControlEvent,
	withWeb06OwnedAction,
} from "./rime";
import ThemeSwitcher from "./ThemeSwitcher";
import Toolbar from "./Toolbar";
import { notify, setToastLanguage, ToastViewport } from "./toast";
import UiLanguageSwitcher from "./UiLanguageSwitcher";
import { schemaText, uiText } from "./uiText";
import YuneControlSurface from "./YuneControlSurface";
import YuneInspector from "./YuneInspector";
import YuneStatusStrip from "./YuneStatusStrip";
import YuneUserdbViewer from "./YuneUserdbViewer";
import {
	WEB06_ACTION_OWNER,
	web06DeployPreferenceFanout,
	web06SchemaChangeFanout,
	web06SingleActionFanout,
} from "./yune-integration/web06-app-action-map";
import { web06ControlAction } from "./yune-integration/private-protocol";

import type {
	RimePreferences,
	GrammarModelDiagnostic,
	YuneWebUserdbSnapshot,
	YuneInspectorDebug,
	YuneStatusSnapshot,
	RimeDeployStatus,
	RimeResult,
	RimeSchemaId,
	Web06ActionIdentity,
	Web06FanoutAction,
} from "./types";
import type { UiLanguage } from "./uiText";

interface YuneMetrics {
	lookupMs?: number;
	aiMs?: number;
	wasmHeapBytes?: number;
	peakWasmHeapBytes?: number;
	candidateCount?: number;
	totalCandidateCount?: number;
	latestInput?: string;
}

type EngineStartupState = "starting" | "ready" | "failed";
type InspectorCandidate = Extract<RimeResult, { isComposing: true }>["candidates"][number];
type DeployPreferenceSet = Pick<
	RimePreferences,
	| "pageSize"
	| "enableCompletion"
	| "enableCorrection"
	| "enableSentence"
	| "enableLearning"
	| "combineCandidates"
	| "predictionNeverFirst"
	| "predictionThreshold"
	| "dictionaryExclude"
	| "isCangjie5"
>;

function metricValue(value: number | undefined, unit: string, emptyLabel: string) {
	if (value === undefined) {
		return <span className="yd-metric-empty">{emptyLabel}</span>;
	}
	return <>
		{value.toFixed(value < 10 ? 1 : 0)}
		<span>{unit}</span>
	</>;
}

function byteMetricValue(value: number | undefined, emptyLabel: string) {
	if (value === undefined) {
		return <span className="yd-metric-empty">{emptyLabel}</span>;
	}
	if (value < 1024) {
		return <>
			{value}
			<span>B</span>
		</>;
	}
	if (value < 1024 * 1024) {
		return <>
			{(value / 1024).toFixed(1)}
			<span>KiB</span>
		</>;
	}
	return <>
		{(value / (1024 * 1024)).toFixed(1)}
		<span>MiB</span>
	</>;
}

function YuneInspectorMetrics({
	metrics,
	userdbSnapshot,
	grammarDiagnostic,
	uiLanguage,
}: {
	metrics: YuneMetrics;
	userdbSnapshot?: YuneWebUserdbSnapshot;
	grammarDiagnostic?: GrammarModelDiagnostic;
	uiLanguage: UiLanguage;
}) {
	const text = uiText[uiLanguage].metrics;
	return <div className="yd-inspector-metrics" aria-label={text.aria} data-yune-metrics>
		<div className="yd-metric">
			<div className="yd-metric-label">{text.lookup}</div>
			<div className="yd-metric-value" data-yune-metric-lookup>{metricValue(metrics.lookupMs, "ms", text.na)}</div>
		</div>
		<div className="yd-metric">
			<div className="yd-metric-label">{text.wasmHeap}</div>
			<div className="yd-metric-value" data-yune-metric-wasm-heap>
				{byteMetricValue(metrics.wasmHeapBytes, text.na)}
			</div>
		</div>
		<div className="yd-metric">
			<div className="yd-metric-label">{text.peakWasmHeap}</div>
			<div className="yd-metric-value" data-yune-metric-peak-wasm-heap>
				{byteMetricValue(metrics.peakWasmHeapBytes, text.na)}
			</div>
		</div>
		<div className="yd-metric">
			<div className="yd-metric-label">{text.aiRerank}</div>
			<div className="yd-metric-value yd-metric-accent" data-yune-metric-ai>
				{metrics.aiMs === undefined ? text.off : metricValue(metrics.aiMs, "ms", text.na)}
			</div>
		</div>
		<div className="yd-metric">
			<div className="yd-metric-label">{text.candidates}</div>
			<div className="yd-metric-value" data-yune-metric-candidates>
				{metrics.candidateCount === undefined
					? <span className="yd-metric-empty">{text.na}</span>
					: <>
						{metrics.candidateCount}
						<span>/{metrics.totalCandidateCount ?? metrics.candidateCount}</span>
					</>}
			</div>
		</div>
		<div className="yd-metric">
			<div className="yd-metric-label">{text.userdb}</div>
			<div className="yd-metric-value" data-yune-metric-userdb>
				{userdbSnapshot
					? <>
						{userdbSnapshot.rows.length}
						<span> {text.rows}</span>
					</>
					: <span className="yd-metric-empty">{text.na}</span>}
			</div>
		</div>
		<div
			className="yd-metric"
			data-yune-grammar-diagnostic
			data-delivered={grammarDiagnostic?.delivered ?? false}
			data-model-id={grammarDiagnostic?.modelId ?? ""}
			data-checksum={grammarDiagnostic?.actualSha256 ?? ""}
			data-expected-checksum={grammarDiagnostic?.expectedSha256 ?? ""}
			data-memory-delta-bytes={grammarDiagnostic?.memoryDeltaBytes ?? ""}>
			<div className="yd-metric-label">{text.octagram}</div>
			<div className="yd-metric-value" data-yune-metric-grammar title={grammarDiagnostic?.actualSha256 ?? grammarDiagnostic?.reason ?? ""}>
				{grammarDiagnostic?.delivered
					? <>
						{grammarDiagnostic.modelId}
						<span> {grammarDiagnostic.actualSha256?.slice(0, 8)}</span>
					</>
					: <span className="yd-metric-empty">{grammarDiagnostic?.fallback ? text.fallback : text.off}</span>}
			</div>
		</div>
	</div>;
}

export default function App() {
	const [textArea, setTextArea] = useState<HTMLTextAreaElement | null>(null);
	const [loading, runAsyncTask, startAsyncTask] = useLoading();
	const [debouncedLoading, setDebouncedLoading] = useState(loading);
	const loadingIndicatorTimeout = useRef<ReturnType<typeof setTimeout>>();
	const [engineStartupState, setEngineStartupState] = useState<EngineStartupState>("starting");
	const [deployStatusLabel, setDeployStatusLabel] = useState<RimeDeployStatus | "idle">("idle");
	const isEngineReady = engineStartupState === "ready";

	useEffect(() => {
		document.documentElement.dataset["yuneLoading"] = String(loading);
	}, [loading]);

	useEffect(() => {
		function clear() {
			if (typeof loadingIndicatorTimeout.current !== "undefined") {
				clearTimeout(loadingIndicatorTimeout.current);
				loadingIndicatorTimeout.current = undefined;
			}
		}
		if (loading) {
			loadingIndicatorTimeout.current = setTimeout(() => setDebouncedLoading(true), 200);
			return clear;
		}
		setDebouncedLoading(false);
		return clear();
	}, [loading]);

	useEffect(() => {
		const { resolve } = startAsyncTask();
		return subscribe("initialized", (success, memory) => {
			setEngineStartupState(success ? "ready" : "failed");
			if (memory) {
				updateMetrics({
					wasmHeapBytes: memory.wasmHeapBytes,
					peakWasmHeapBytes: memory.peakWasmHeapBytes,
				});
			}
			if (!success) {
				notify(
					"error",
					"啟動輸入法引擎",
					"initializing the input method engine",
				);
			}
			resolve();
		});
	}, [startAsyncTask]);

	useEffect(() => {
		let pending: PromiseWithResolvers<void> | undefined;
		return subscribe("deployStatusChanged", (status) => {
			setDeployStatusLabel(status);
			switch (status) {
				case "start":
					pending?.resolve();
					pending = startAsyncTask();
					break;
				case "success":
					pending?.resolve();
					pending = undefined;
					break;
				case "failure":
					notify("warning", "重新整理", "refreshing");
					pending?.reject();
					pending = undefined;
					break;
			}
		});
	}, [startAsyncTask]);

	const [deployStatus, updateDeployStatus] = useReducer(
		(n: number) => n + 1,
		0,
	);
	const [aiStatus, updateAiStatus] = useReducer((n: number) => n + 1, 0);
	const [isInspectorEnabled, setIsInspectorEnabled] = useState(false);
	const [inspectorDebug, setInspectorDebug] = useState<
		YuneInspectorDebug | undefined
	>();
	const [inspectorCandidates, setInspectorCandidates] = useState<InspectorCandidate[]>([]);
	const [engineStatus, setEngineStatus] = useState<
		YuneStatusSnapshot | undefined
	>();
	const [metrics, setMetrics] = useState<YuneMetrics>({});
	const [grammarDiagnostic, setGrammarDiagnostic] = useState<GrammarModelDiagnostic | undefined>();
	const updateMetrics = useCallback((next: YuneMetrics) => {
		setMetrics(current => {
			const merged = { ...current, ...next };
			return current.lookupMs === merged.lookupMs
				&& current.aiMs === merged.aiMs
				&& current.wasmHeapBytes === merged.wasmHeapBytes
				&& current.peakWasmHeapBytes === merged.peakWasmHeapBytes
				&& current.candidateCount === merged.candidateCount
				&& current.totalCandidateCount === merged.totalCandidateCount
				&& current.latestInput === merged.latestInput
				? current
				: merged;
		});
	}, []);
	const [userdbRefreshStatus, bumpUserdbRefreshStatus] = useReducer(
		(n: number) => n + 1,
		0,
	);
	const userdbRefreshCause = useRef<Web06ActionIdentity>();
	const refreshUserdbAfterCommit = useCallback((identity?: Web06ActionIdentity) => {
		userdbRefreshCause.current = identity;
		bumpUserdbRefreshStatus();
	}, []);
	const [userdbSnapshot, setUserdbSnapshot] = useState<
		YuneWebUserdbSnapshot | undefined
	>();
	const [isUserdbLoading, setIsUserdbLoading] = useState(false);
	const [userdbError, setUserdbError] = useState<string | undefined>();
	const preferences = usePreferences();
	const {
		uiLanguage,
		pageSize,
		enableCompletion,
		enableCorrection,
		enableSentence,
		enableLearning,
		enableAI,
		combineCandidates,
		predictionNeverFirst,
		predictionThreshold,
		activeSchema,
		isExtendedCharset,
		isDisabled,
		dictionaryExclude,
		isAsciiMode,
		setIsAsciiMode,
		isFullShape,
		outputStandard,
		isCangjie5,
		chineseTypeface,
	} = preferences;
	const text = uiText[uiLanguage];
	const outputStandardValue = normalizeOutputStandard(outputStandard, "hong_kong_traditional");
	const composePlaceholder = schemaText[uiLanguage][activeSchema].label;
	const [isAsciiPunct, toggleAsciiPunct] = useRimeOption("ascii_punct", false, deployStatus, "isAsciiPunct", isEngineReady);
	const setIsAsciiPunct = useCallback((checked: boolean) => {
		if (checked !== isAsciiPunct) {
			toggleAsciiPunct();
		}
	}, [isAsciiPunct, toggleAsciiPunct]);
	const toggleAsciiMode = useCallback(() => {
		setIsAsciiMode(value => !value);
	}, [setIsAsciiMode]);
	const didRunSchemaEffect = useRef(false);
	const didRunDeployPreferencesEffect = useRef(false);
	const lastDeployPreferenceKey = useRef<string | undefined>(undefined);
	const lastDeployPreferenceSchema = useRef<string | undefined>(undefined);
	const restoreTextAreaFocusAfterLiveOptions = useRef(false);

	useEffect(() => {
		document.documentElement.lang = uiLanguage === "yue" ? "zh-HK" : "en";
		setToastLanguage(uiLanguage);
	}, [uiLanguage]);

	useEffect(() => {
		if (!isEngineReady) {
			return;
		}
		runAsyncTask(async () => {
			if (!didRunSchemaEffect.current) {
				didRunSchemaEffect.current = true;
				if (document.documentElement.dataset["yuneActiveSchema"] === activeSchema) {
					return;
				}
			}
			let type: "warning" | "error" | undefined;
			try {
					if (!(await withWeb06OwnedAction(
						WEB06_ACTION_OWNER.schema,
						"selectSchema",
						[activeSchema],
						"schema-effect",
						undefined,
						() => Rime.selectSchema(activeSchema),
					))) {
					type = "warning";
				}
				setInspectorDebug(undefined);
				setInspectorCandidates([]);
				setEngineStatus(undefined);
				setMetrics(current => ({
					wasmHeapBytes: current.wasmHeapBytes,
					peakWasmHeapBytes: current.peakWasmHeapBytes,
				}));
				updateDeployStatus();
			} catch {
				type = "error";
			}
			if (type) {
				notify(type, "切換方案", "switching the schema");
			}
		});
	}, [activeSchema, isEngineReady, runAsyncTask]);

	useEffect(() =>
		subscribe("grammarDiagnosticChanged", setGrammarDiagnostic),
	[]);

	useEffect(() => {
		if (!isEngineReady) {
			return;
		}
		runAsyncTask(async () => {
			const deployPreferences: DeployPreferenceSet = {
				pageSize,
				enableCompletion,
				enableCorrection,
				enableSentence,
				enableLearning,
				combineCandidates,
				predictionNeverFirst,
				predictionThreshold,
				dictionaryExclude,
				isCangjie5,
			};
			const deployPreferenceKey = deployPreferenceSetKey(deployPreferences);
			const firstRun = !didRunDeployPreferencesEffect.current;
			const previousDeployPreferenceKey = lastDeployPreferenceKey.current;
			const previousDeployPreferenceSchema = lastDeployPreferenceSchema.current;
			const deployPreferencesChanged =
				previousDeployPreferenceKey !== undefined
				&& previousDeployPreferenceKey !== deployPreferenceKey;
			const schemaChanged =
				previousDeployPreferenceSchema !== undefined
				&& previousDeployPreferenceSchema !== activeSchema;
			didRunDeployPreferencesEffect.current = true;
			lastDeployPreferenceKey.current = deployPreferenceKey;
			lastDeployPreferenceSchema.current = activeSchema;

			if (!firstRun && !deployPreferencesChanged && !schemaChanged) {
				return;
			}
			if (
				isDefaultDeployPreferenceSet(deployPreferences)
				&& (
					firstRun
					|| (schemaChanged && !deployPreferencesChanged && activeSchema === "jyut6ping3")
				)
			) {
				return;
			}
			let type: "warning" | "error" | undefined;
			try {
					const success = await withWeb06OwnedAction(
						WEB06_ACTION_OWNER.deployPreferences,
						"customize",
						[deployPreferences],
						"deploy-preferences-effect",
						undefined,
						() => Rime.customize(deployPreferences),
					);
					const deployed = await withWeb06OwnedAction(
						WEB06_ACTION_OWNER.deployPreferences,
						"deploy",
						[],
						"deploy-preferences-effect",
						undefined,
						() => Rime.deploy(),
					);
					if (!(deployed && success)) {
					type = "warning";
				}
			} catch {
				type = "error";
			}
			if (type) {
				notify(type, "套用設定", "applying the settings");
			}
			updateDeployStatus();
		});
	}, [
		activeSchema,
		pageSize,
		enableCompletion,
		enableCorrection,
		enableSentence,
		enableLearning,
		combineCandidates,
		predictionNeverFirst,
		predictionThreshold,
		dictionaryExclude,
		isCangjie5,
		isEngineReady,
		updateDeployStatus,
		runAsyncTask,
	]);

	useEffect(() => {
		if (!isEngineReady) {
			return;
		}
		if (textArea !== null && document.activeElement === textArea) {
			restoreTextAreaFocusAfterLiveOptions.current = true;
		}
		runAsyncTask(async () => {
			let type: "warning" | "error" | undefined;
			try {
					const setLiveOption = (option: string, value: boolean) => withWeb06OwnedAction(
						WEB06_ACTION_OWNER.liveOptions,
						"setOption",
						[option, value],
						"live-options-effect",
						undefined,
						() => Rime.setOption(option, value),
					);
					await setLiveOption("soft_cursor", true);
					await setLiveOption("ascii_mode", isAsciiMode);
					await setLiveOption("full_shape", isFullShape);
					await setLiveOption("traditionalization", false);
				const activeOutputOption = outputOptionForStandard(outputStandardValue, activeSchema);
				document.documentElement.dataset["yuneActiveOutputOption"] = activeOutputOption ?? "none";
				const appliedOutputOptions: string[] = [];
				for (const optionName of OUTPUT_STANDARD_ENGINE_OPTIONS) {
						await setLiveOption(optionName, optionName === activeOutputOption);
					appliedOutputOptions.push(`${optionName}:${optionName === activeOutputOption}`);
				}
				document.documentElement.dataset["yuneAppliedOutputOptions"] = appliedOutputOptions.join(",");
					await setLiveOption("extended_charset", isExtendedCharset);
					await setLiveOption("disabled", isDisabled);
			} catch {
				type = "error";
			}
			if (type) {
				notify(type, "套用即時選項", "applying the live options");
			}
		});
	}, [
		activeSchema,
		deployStatus,
		isAsciiMode,
		isFullShape,
		outputStandardValue,
		isExtendedCharset,
		isDisabled,
		isEngineReady,
		runAsyncTask,
		textArea,
	]);

	useEffect(() => {
		if (
			!loading
			&& restoreTextAreaFocusAfterLiveOptions.current
			&& textArea !== null
		) {
			restoreTextAreaFocusAfterLiveOptions.current = false;
			if (
				textArea.isConnected
				&& !textArea.disabled
				&& document.activeElement === document.body
			) {
				textArea.focus({ preventScroll: true });
			}
		}
	}, [loading, textArea]);

	useEffect(() => {
		if (!isEngineReady) {
			return;
		}
		runAsyncTask(async () => {
			let type: "error" | undefined;
			try {
					await withWeb06OwnedAction(
						WEB06_ACTION_OWNER.inspector,
						"setOption",
						["yune_inspector", isInspectorEnabled],
						"inspector-effect",
						undefined,
						() => Rime.setOption("yune_inspector", isInspectorEnabled),
					);
				if (!isInspectorEnabled) {
					setInspectorDebug(undefined);
				}
			} catch {
				type = "error";
			}
			if (type) {
				notify(type, "套用檢視設定", "applying the inspector setting");
			}
		});
	}, [isEngineReady, isInspectorEnabled, runAsyncTask]);

	useEffect(() => {
		if (!isEngineReady) {
			return;
		}
		let cancelled = false;
		async function applyAiSettings() {
			let type: "warning" | "error" | undefined;
			try {
					if (!(await withWeb06OwnedAction(
						WEB06_ACTION_OWNER.aiSettings,
						"customize",
						[{ enableAI }],
						"ai-settings-effect",
						undefined,
						() => Rime.customize({ enableAI }),
					))) {
					type = "warning";
				}
			} catch {
				type = "error";
			}
			if (type) {
				notify(type, "套用智能設定", "applying the AI settings");
			}
			if (!cancelled) {
				updateAiStatus();
			}
		}
		void applyAiSettings();
		return () => {
			cancelled = true;
		};
	}, [enableAI, isEngineReady, updateAiStatus]);

	const refreshUserdbSnapshot = useCallback(async () => {
		setIsUserdbLoading(true);
		setUserdbError(undefined);
		try {
			const cause = userdbRefreshCause.current;
			userdbRefreshCause.current = undefined;
			setUserdbSnapshot(await withWeb06OwnedAction(
				WEB06_ACTION_OWNER.userdb,
				"getUserdbSnapshot",
				[],
				cause === undefined ? "userdb-refresh-effect" : "commit-userdb-refresh",
				cause,
				() => Rime.getUserdbSnapshot(),
			));
		} catch (error) {
			setUserdbError(
				error instanceof Error
					? `${error.name}: ${error.message}`
					: String(error),
			);
		} finally {
			setIsUserdbLoading(false);
		}
	}, []);

	const importUserdbSnapshot = useCallback(async (rawText: string) => {
		setIsUserdbLoading(true);
		setUserdbError(undefined);
		try {
			setUserdbSnapshot(await withWeb06OwnedAction(
				WEB06_ACTION_OWNER.userdb,
				"importUserdb",
				[rawText],
				"userdb-import",
				undefined,
				() => Rime.importUserdb(rawText),
			));
		} catch (error) {
			setUserdbError(
				error instanceof Error
					? `${error.name}: ${error.message}`
					: String(error),
			);
			throw error;
		} finally {
			setIsUserdbLoading(false);
		}
	}, []);

	useEffect(() => {
		if (!isEngineReady || loading) {
			return;
		}
		void refreshUserdbSnapshot();
	}, [
		isEngineReady,
		loading,
		activeSchema,
		deployStatus,
		userdbRefreshStatus,
		refreshUserdbSnapshot,
	]);

	const isInputDisabled = loading || !isEngineReady;
	const showInputOverlay = debouncedLoading || engineStartupState !== "ready";
	const inputOverlayMessage = engineStartupState === "failed"
		? text.compose.startupFailed
		: text.compose.loading;
	const web06DeployPreferencePlan = useCallback((patch: Partial<DeployPreferenceSet>): Web06FanoutAction[] =>
		web06DeployPreferenceFanout({
			pageSize,
			enableCompletion,
			enableCorrection,
			enableSentence,
			enableLearning,
			combineCandidates,
			predictionNeverFirst,
			predictionThreshold,
			dictionaryExclude,
			isCangjie5,
			...patch,
		}), [
		pageSize,
		enableCompletion,
		enableCorrection,
		enableSentence,
		enableLearning,
		combineCandidates,
		predictionNeverFirst,
		predictionThreshold,
		dictionaryExclude,
		isCangjie5,
	]);
	const web06SchemaChangePlan = useCallback((nextSchema: RimeSchemaId): Web06FanoutAction[] => {
		const deployPreferences: DeployPreferenceSet = {
			pageSize,
			enableCompletion,
			enableCorrection,
			enableSentence,
			enableLearning,
			combineCandidates,
			predictionNeverFirst,
			predictionThreshold,
			dictionaryExclude,
			isCangjie5,
		};
		return web06SchemaChangeFanout({
			nextSchema,
			deployPreferences,
			liveOptions: {
				isAsciiMode,
				isFullShape,
				outputStandard: outputStandardValue,
				activeSchema: nextSchema,
				isExtendedCharset,
				isDisabled,
			},
			applyDeployPreferences: !(
				isDefaultDeployPreferenceSet(deployPreferences)
				&& nextSchema === "jyut6ping3"
			),
		});
	}, [
		pageSize,
		enableCompletion,
		enableCorrection,
		enableSentence,
		enableLearning,
		combineCandidates,
		predictionNeverFirst,
		predictionThreshold,
		dictionaryExclude,
		isCangjie5,
		isAsciiMode,
		isFullShape,
		outputStandardValue,
		isExtendedCharset,
		isDisabled,
	]);

	return (
		<div
			className="yd-app-shell"
			lang={uiLanguage === "yue" ? "zh-HK" : "en"}
			data-yune-ui-language={uiLanguage}
			data-chinese-typeface={chineseTypeface}
			data-yune-output-standard={outputStandardValue}
			data-yune-output-option-count={OUTPUT_STANDARD_ENGINE_OPTIONS.length}>
			<header className="yd-app-header">
				<div className="yd-app-header-inner">
					<a className="yd-brand" href="/" aria-label={text.header.home}>
						<span className="yd-brand-mark">韻</span>
						<span className="yd-brand-copy">
							<span className="yd-brand-product">{text.header.product}</span>
							<span className="yd-brand-title">{text.header.title}</span>
						</span>
					</a>
					<div className="yd-header-status">
						<UiLanguageSwitcher
							uiLanguage={uiLanguage}
							setUiLanguage={preferences.setUiLanguage}
						/>
						<ThemeSwitcher uiLanguage={uiLanguage} />
					</div>
				</div>
			</header>
			<main className="yd-app-main">
				<section data-yune-playground-content className="yd-playground">
					<Toolbar
						isAsciiMode={preferences.isAsciiMode}
						setIsAsciiMode={preferences.setIsAsciiMode}
						outputStandard={outputStandardValue}
						setOutputStandard={preferences.setOutputStandard}
						isFullShape={preferences.isFullShape}
						setIsFullShape={preferences.setIsFullShape}
						activeSchema={preferences.activeSchema}
						setActiveSchema={preferences.setActiveSchema}
							isCangjie5={preferences.isCangjie5}
							setIsCangjie5={preferences.setIsCangjie5}
							isExtendedCharset={preferences.isExtendedCharset}
							isDisabled={preferences.isDisabled}
							web06SchemaChangePlan={web06SchemaChangePlan}
							web06DeployPreferencePlan={web06DeployPreferencePlan}
						uiLanguage={uiLanguage}
					/>
					<div className="yd-playground-grid">
						<section className="yd-compose-panel" aria-label={text.compose.panelAria}>
							<div className="yd-panel-heading">
								<span>{text.compose.title}</span>
								<label className="yd-compose-panel-toggle">
									<span>{text.compose.fixedFloating}</span>
									<input
										type="checkbox"
										className="yd-check yd-toggle"
										{...NO_AUTO_FILL}
										checked={preferences.isCandidatePanelFixed}
										onChange={event => preferences.setIsCandidatePanelFixed(event.target.checked)}
										aria-label={text.compose.fixedFloating} />
								</label>
							</div>
							<div className="yd-input-frame" aria-busy={isInputDisabled} data-yune-input-frame>
								<textarea
									className="yd-input-area"
									data-chinese-typeface={chineseTypeface}
									ref={setTextArea}
									aria-label={text.compose.inputAria}
									placeholder={composePlaceholder}
									disabled={isInputDisabled}
									{...NO_AUTO_FILL}
								/>
								{showInputOverlay && <div className="yd-input-loading" data-yune-loading-indicator role="status" aria-live="polite">
									{engineStartupState !== "failed" && <span className="yd-spinner" aria-hidden="true" />}
									<span>{inputOverlayMessage}</span>
								</div>}
							</div>
							{textArea && isEngineReady && (
								<CandidatePanel
									runAsyncTask={runAsyncTask}
									textArea={textArea}
									prefs={preferences}
									deployStatus={deployStatus}
									aiStatus={aiStatus}
									onInspectorDebug={setInspectorDebug}
									onInspectorCandidates={setInspectorCandidates}
									onStatus={setEngineStatus}
									onUserdbChange={refreshUserdbAfterCommit}
									onMetrics={updateMetrics}
									onToggleAsciiMode={toggleAsciiMode}
								/>
							)}
						</section>
						<YuneUserdbViewer
							snapshot={userdbSnapshot}
							isLoading={isUserdbLoading}
							error={userdbError}
							onRefresh={refreshUserdbSnapshot}
							onImport={importUserdbSnapshot}
							uiLanguage={uiLanguage} />
					</div>
					<YuneStatusStrip status={engineStatus} outputStandard={outputStandardValue} uiLanguage={uiLanguage} />
					<section className="yd-inspector-gate">
						<div className="yd-inspector-gate-header">
							<div>
								<span>{text.inspector.gateTitle}</span>
							</div>
							<YuneInspectorMetrics metrics={metrics} userdbSnapshot={userdbSnapshot} grammarDiagnostic={grammarDiagnostic} uiLanguage={uiLanguage} />
							<label className="yd-inspector-enable" data-yune-inspector-toggle>
								<input
									type="checkbox"
									className="yd-check yd-toggle"
									checked={isInspectorEnabled}
									aria-label={text.inspector.traceAria}
									onChange={event => withWeb06ControlEvent(event, () => {
										declareWeb06ControlFanout(
											"inspector-toggle",
											web06SingleActionFanout(
												WEB06_ACTION_OWNER.inspector,
												web06ControlAction("setOption", ["yune_inspector", event.currentTarget.checked]),
											),
										);
										setIsInspectorEnabled(event.currentTarget.checked);
									})}
								/>
								<span>{isInspectorEnabled ? text.inspector.traceOn : text.inspector.traceOff}</span>
							</label>
						</div>
						{isInspectorEnabled && (
							<YuneInspector
								debug={inspectorDebug}
								candidates={inspectorCandidates}
								uiLanguage={uiLanguage}
							/>
						)}
					</section>
					<YuneControlSurface
						uiLanguage={uiLanguage}
						isEngineReady={isEngineReady}
						deployStatus={deployStatusLabel}
						refreshSignal={deployStatus}
						onDeployMutation={updateDeployStatus}
					/>
				</section>
				<Preferences
					{...preferences}
					isAsciiPunct={isAsciiPunct}
					setIsAsciiPunct={setIsAsciiPunct} />
			</main>
			<footer className="yd-app-footer">
				<span>{text.header.footer}</span>
				<a
					className="yd-anchor"
					href="/PROVENANCE.md"
					target="_blank"
					rel="noreferrer"
				>
					{text.header.provenance}
				</a>
			</footer>
			<ToastViewport />
		</div>
	);
}

function deployPreferenceSetKey(preferences: DeployPreferenceSet): string {
	return JSON.stringify({
		pageSize: preferences.pageSize,
		enableCompletion: preferences.enableCompletion,
		enableCorrection: preferences.enableCorrection,
		enableSentence: preferences.enableSentence,
		enableLearning: preferences.enableLearning,
		combineCandidates: preferences.combineCandidates,
		predictionNeverFirst: preferences.predictionNeverFirst,
		predictionThreshold: preferences.predictionThreshold,
		dictionaryExclude: preferences.dictionaryExclude,
		isCangjie5: preferences.isCangjie5,
	});
}

function isDefaultDeployPreferenceSet(preferences: DeployPreferenceSet) {
	return preferences.pageSize === 6
		&& preferences.enableCompletion
		&& !preferences.enableCorrection
		&& preferences.enableSentence
		&& preferences.enableLearning
		&& preferences.combineCandidates
		&& preferences.predictionNeverFirst
		&& preferences.predictionThreshold === 0
		&& preferences.dictionaryExclude.length === 0
		&& preferences.isCangjie5;
}

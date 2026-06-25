import { useCallback, useEffect, useReducer, useRef, useState } from "react";

import CandidatePanel from "./CandidatePanel";
import { NO_AUTO_FILL } from "./consts";
import { useLoading, usePreferences } from "./hooks";
import Preferences from "./Preferences";
import Rime, { subscribe } from "./rime";
import ThemeSwitcher from "./ThemeSwitcher";
import Toolbar from "./Toolbar";
import { notify, ToastViewport } from "./toast";
import YuneInspector from "./YuneInspector";
import YuneStatusStrip from "./YuneStatusStrip";
import YuneUserdbViewer from "./YuneUserdbViewer";

import type {
	YuneWebUserdbSnapshot,
	YuneInspectorDebug,
	YuneStatusSnapshot,
} from "./types";

interface YuneMetrics {
	lookupMs?: number;
	aiMs?: number;
	candidateCount?: number;
	totalCandidateCount?: number;
	latestInput?: string;
}

function metricValue(value: number | undefined, unit: string) {
	if (value === undefined) {
		return <span className="yd-metric-empty">N/A</span>;
	}
	return <>
		{value.toFixed(value < 10 ? 1 : 0)}
		<span>{unit}</span>
	</>;
}

function YuneMetricsStrip({
	metrics,
	userdbSnapshot,
}: {
	metrics: YuneMetrics;
	userdbSnapshot?: YuneWebUserdbSnapshot;
}) {
	return <section className="yd-metrics" aria-label="Live engine metrics" data-yune-metrics>
		<div className="yd-metric">
			<div className="yd-metric-label">LOOKUP</div>
			<div className="yd-metric-value" data-yune-metric-lookup>{metricValue(metrics.lookupMs, "ms")}</div>
		</div>
		<div className="yd-metric">
			<div className="yd-metric-label">AI RERANK</div>
			<div className="yd-metric-value yd-metric-accent" data-yune-metric-ai>
				{metrics.aiMs === undefined ? "off" : metricValue(metrics.aiMs, "ms")}
			</div>
		</div>
		<div className="yd-metric">
			<div className="yd-metric-label">CANDIDATES</div>
			<div className="yd-metric-value" data-yune-metric-candidates>
				{metrics.candidateCount === undefined
					? <span className="yd-metric-empty">N/A</span>
					: <>
						{metrics.candidateCount}
						<span>/{metrics.totalCandidateCount ?? metrics.candidateCount}</span>
					</>}
			</div>
		</div>
		<div className="yd-metric">
			<div className="yd-metric-label">USERDB</div>
			<div className="yd-metric-value" data-yune-metric-userdb>
				{userdbSnapshot
					? <>
						{userdbSnapshot.rows.length}
						<span> rows</span>
					</>
					: <span className="yd-metric-empty">N/A</span>}
			</div>
		</div>
	</section>;
}

export default function App() {
	const [textArea, setTextArea] = useState<HTMLTextAreaElement | null>(null);
	const [loading, runAsyncTask, startAsyncTask] = useLoading();
	const [debouncedLoading, setDebouncedLoading] = useState(loading);
	const loadingIndicatorTimeout = useRef<ReturnType<typeof setTimeout>>();
	const [isEngineReady, setIsEngineReady] = useState(false);

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
		return subscribe("initialized", (success) => {
			setIsEngineReady(success);
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
	const [engineStatus, setEngineStatus] = useState<
		YuneStatusSnapshot | undefined
	>();
	const [metrics, setMetrics] = useState<YuneMetrics>({});
	const updateMetrics = useCallback((next: YuneMetrics) => {
		setMetrics(current => {
			const merged = { ...current, ...next };
			return current.lookupMs === merged.lookupMs
				&& current.aiMs === merged.aiMs
				&& current.candidateCount === merged.candidateCount
				&& current.totalCandidateCount === merged.totalCandidateCount
				&& current.latestInput === merged.latestInput
				? current
				: merged;
		});
	}, []);
	const [userdbRefreshStatus, refreshUserdbAfterCommit] = useReducer(
		(n: number) => n + 1,
		0,
	);
	const [userdbSnapshot, setUserdbSnapshot] = useState<
		YuneWebUserdbSnapshot | undefined
	>();
	const [isUserdbLoading, setIsUserdbLoading] = useState(false);
	const [userdbError, setUserdbError] = useState<string | undefined>();
	const preferences = usePreferences();
	const {
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
		isFullShape,
		isSimplification,
		isCangjie5,
		chineseTypeface,
	} = preferences;
	const didRunSchemaEffect = useRef(false);
	useEffect(
		() =>
			runAsyncTask(async () => {
				if (!didRunSchemaEffect.current) {
					didRunSchemaEffect.current = true;
					if (activeSchema === "jyut6ping3_mobile") {
						return;
					}
				}
				let type: "warning" | "error" | undefined;
				try {
					if (!(await Rime.selectSchema(activeSchema))) {
						type = "warning";
					}
					setInspectorDebug(undefined);
					setEngineStatus(undefined);
					setMetrics({});
					updateDeployStatus();
				} catch {
					type = "error";
				}
				if (type) {
					notify(type, "Switch schema", "switching the schema");
				}
			}),
		[activeSchema, runAsyncTask],
	);

	useEffect(
		() =>
			runAsyncTask(async () => {
				let type: "warning" | "error" | undefined;
				try {
					const success = await Rime.customize({
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
					});
					if (!((await Rime.deploy()) && success)) {
						type = "warning";
					}
				} catch {
					type = "error";
				}
				if (type) {
					notify(type, "套用設定", "applying the settings");
				}
				updateDeployStatus();
			}),
		[
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
			updateDeployStatus,
			runAsyncTask,
		],
	);

	useEffect(
		() =>
			runAsyncTask(async () => {
				let type: "warning" | "error" | undefined;
				try {
					await Rime.setOption("soft_cursor", true);
					await Rime.setOption("ascii_mode", isAsciiMode);
					await Rime.setOption("full_shape", isFullShape);
					await Rime.setOption("simplification", isSimplification);
					await Rime.setOption("traditionalization", false);
					await Rime.setOption("extended_charset", isExtendedCharset);
					await Rime.setOption("disabled", isDisabled);
				} catch {
					type = "error";
				}
				if (type) {
					notify(type, "Apply live options", "applying the live options");
				}
			}),
		[
			activeSchema,
			isAsciiMode,
			isFullShape,
			isSimplification,
			isExtendedCharset,
			isDisabled,
			runAsyncTask,
		],
	);

	useEffect(
		() =>
			runAsyncTask(async () => {
				let type: "error" | undefined;
				try {
					await Rime.setOption("yune_inspector", isInspectorEnabled);
					if (!isInspectorEnabled) {
						setInspectorDebug(undefined);
					}
				} catch {
					type = "error";
				}
				if (type) {
					notify(type, "Apply inspector", "applying the inspector setting");
				}
			}),
		[isInspectorEnabled, runAsyncTask],
	);

	useEffect(() => {
		let cancelled = false;
		async function applyAiSettings() {
			let type: "warning" | "error" | undefined;
			try {
				if (!(await Rime.customize({ enableAI }))) {
					type = "warning";
				}
			} catch {
				type = "error";
			}
			if (type) {
				notify(type, "Apply AI settings", "applying the AI settings");
			}
			if (!cancelled) {
				updateAiStatus();
			}
		}
		void applyAiSettings();
		return () => {
			cancelled = true;
		};
	}, [enableAI, updateAiStatus]);

	const refreshUserdbSnapshot = useCallback(async () => {
		setIsUserdbLoading(true);
		setUserdbError(undefined);
		try {
			setUserdbSnapshot(await Rime.getUserdbSnapshot());
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

	return (
		<div className="yd-app-shell" data-chinese-typeface={chineseTypeface}>
			<header className="yd-app-header">
				<div className="yd-app-header-inner">
					<a className="yd-brand" href="/" aria-label="yune-web home">
						<span className="yd-brand-mark">韻</span>
						<span className="yd-brand-copy">
							<span className="yd-brand-title">新韻輸入法引擎 <span>yune-web</span></span>
						</span>
					</a>
					<div className="yd-header-status">
						<span className="yd-version-chip"><span />PUBLIC</span>
						<ThemeSwitcher />
					</div>
				</div>
			</header>
			<main className="yd-app-main">
				<section data-yune-playground-content className="yd-playground">
					<Toolbar
						isAsciiMode={preferences.isAsciiMode}
						setIsAsciiMode={preferences.setIsAsciiMode}
						isSimplification={preferences.isSimplification}
						setIsSimplification={preferences.setIsSimplification}
						isFullShape={preferences.isFullShape}
						setIsFullShape={preferences.setIsFullShape}
						activeSchema={preferences.activeSchema}
						setActiveSchema={preferences.setActiveSchema}
						isCangjie5={preferences.isCangjie5}
						setIsCangjie5={preferences.setIsCangjie5}
					/>
					<div className="yd-playground-grid">
						<section className="yd-compose-panel" aria-label="IME playground">
							<div className="yd-panel-heading">
								<span>輸入測試</span>
								<label className="yd-compose-panel-toggle">
									<span>固定浮窗</span>
									<input
										type="checkbox"
										className="yd-check yd-toggle"
										{...NO_AUTO_FILL}
										checked={preferences.isCandidatePanelFixed}
										onChange={event => preferences.setIsCandidatePanelFixed(event.target.checked)}
										aria-label="固定浮窗" />
								</label>
							</div>
							<div className="yd-input-frame" aria-busy={loading} data-yune-input-frame>
								<textarea
									className="yd-input-area"
									data-chinese-typeface={chineseTypeface}
									ref={setTextArea}
									aria-label="yune-web composing input"
									placeholder="粵語拼音"
									disabled={loading}
									{...NO_AUTO_FILL}
								/>
								{debouncedLoading && <div className="yd-input-loading" data-yune-loading-indicator role="status" aria-live="polite">
									<span className="yd-spinner" aria-hidden="true" />
									<span>載入中 Loading...</span>
								</div>}
							</div>
							{textArea && (
								<CandidatePanel
									runAsyncTask={runAsyncTask}
									textArea={textArea}
									prefs={preferences}
									deployStatus={deployStatus}
									aiStatus={aiStatus}
									onInspectorDebug={setInspectorDebug}
									onStatus={setEngineStatus}
									onUserdbChange={refreshUserdbAfterCommit}
									onMetrics={updateMetrics}
								/>
							)}
						</section>
						<YuneUserdbViewer
							snapshot={userdbSnapshot}
							isLoading={isUserdbLoading}
							error={userdbError}
							onRefresh={refreshUserdbSnapshot} />
					</div>
					<YuneMetricsStrip metrics={metrics} userdbSnapshot={userdbSnapshot} />
					<YuneStatusStrip status={engineStatus} />
					<section className="yd-inspector-gate">
						<div className="yd-inspector-gate-header">
							<div>
								<span>引擎檢視</span>
								<span>Yune inspector</span>
							</div>
							<label className="yd-inspector-enable" data-yune-inspector-toggle>
								<input
									type="checkbox"
									className="yd-check yd-toggle"
									checked={isInspectorEnabled}
									aria-label="Yune inspector"
									onChange={(event) =>
										setIsInspectorEnabled(event.currentTarget.checked)
									}
								/>
								<span>{isInspectorEnabled ? "TRACE ON" : "TRACE OFF"}</span>
							</label>
						</div>
						{isInspectorEnabled && (
							<YuneInspector
								debug={inspectorDebug}
							/>
						)}
					</section>
				</section>
				<Preferences {...preferences} />
			</main>
			<footer className="yd-app-footer">
				<span>yune-web public demo. Yune and upstream-derived assets are licensed under their upstream terms.</span>
				<a
					className="yd-anchor"
					href="/PROVENANCE.md"
					target="_blank"
					rel="noreferrer"
				>
					Provenance
				</a>
			</footer>
			<ToastViewport />
		</div>
	);
}

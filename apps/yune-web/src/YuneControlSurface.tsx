import { useCallback, useEffect, useState } from "react";

import { IS_PUBLIC_DEMO, NO_AUTO_FILL } from "./consts";
import Rime from "./rime";
import { uiText } from "./uiText";

import type {
	YuneDeployCacheSnapshot,
	YuneInjectedAssetManifest,
	RimeDeployStatus,
} from "./types";
import type { UiLanguage } from "./uiText";

type DiagnosticPayload = {
	source?: string;
	marker?: {
		phase?: string;
		reason?: string;
		schemaId?: string;
		timestamp?: string;
		persistedConfig?: { path?: string; bytes?: number; settings?: Record<string, string | null> };
		deployedConfig?: { path?: string; bytes?: number; settings?: Record<string, string | null> };
	};
};

type ActionError = {
	action?: string;
	input?: string;
	error?: {
		name?: string;
		message?: string;
		value?: string;
	};
};

interface DiagnosticsSnapshot {
	persistenceDiagnostics: DiagnosticPayload[];
	actionErrors: ActionError[];
	lastActionResult: string;
}

function parseDatasetJson<T>(key: string, fallback: T): T {
	const raw = document.documentElement.dataset[key];
	if (!raw) {
		return fallback;
	}
	try {
		return JSON.parse(raw) as T;
	}
	catch {
		return fallback;
	}
}

function readDiagnosticsSnapshot(): DiagnosticsSnapshot {
	return {
		persistenceDiagnostics: parseDatasetJson<DiagnosticPayload[]>("yunePersistenceDiagnostics", []),
		actionErrors: parseDatasetJson<ActionError[]>("yuneActionErrors", []),
		lastActionResult: document.documentElement.dataset["yuneLastActionResult"] ?? "",
	};
}

function formatDiagnostic(diagnostic: DiagnosticPayload): string {
	const marker = diagnostic.marker;
	return [
		diagnostic.source,
		marker?.phase,
		marker?.reason,
		marker?.schemaId,
		marker?.persistedConfig?.path,
	].filter(Boolean).join(" | ");
}

function formatActionError(error: ActionError): string {
	return [
		error.action,
		error.input,
		error.error?.name,
		error.error?.message,
		error.error?.value,
	].filter(Boolean).join(" | ");
}

export default function YuneControlSurface({
	uiLanguage,
	isEngineReady,
	deployStatus,
	refreshSignal,
	onDeployMutation,
}: {
	uiLanguage: UiLanguage;
	isEngineReady: boolean;
	deployStatus: RimeDeployStatus | "idle";
	refreshSignal: number;
	onDeployMutation(): void;
}) {
	const text = uiText[uiLanguage].controlSurface;
	const [cache, setCache] = useState<YuneDeployCacheSnapshot | undefined>();
	const [assets, setAssets] = useState<YuneInjectedAssetManifest | undefined>();
	const [diagnostics, setDiagnostics] = useState<DiagnosticsSnapshot>(() => ({
		persistenceDiagnostics: [],
		actionErrors: [],
		lastActionResult: "",
	}));
	const [optionName, setOptionName] = useState("ascii_mode");
	const [optionValue, setOptionValue] = useState("true");
	const [customizeConfigId, setCustomizeConfigId] = useState("jyut6ping3.schema");
	const [customizeKey, setCustomizeKey] = useState("translator/enable_completion");
	const [customizeValue, setCustomizeValue] = useState("true");

	const refreshDiagnostics = useCallback(() => {
		setDiagnostics(readDiagnosticsSnapshot());
	}, []);

	const refreshRemoteState = useCallback(async () => {
		if (!isEngineReady) {
			return;
		}
		if (IS_PUBLIC_DEMO) {
			refreshDiagnostics();
			return;
		}
		try {
			const [nextCache, nextAssets] = await Promise.all([
				Rime.deployCacheSnapshot(),
				Rime.injectedAssetsManifest(),
			]);
			setCache(nextCache);
			setAssets(nextAssets);
			document.documentElement.dataset["yuneDeployCacheFresh"] = String(nextCache.cacheFresh);
		}
		catch {
			// The diagnostics panel should not destabilize the input path.
		}
		refreshDiagnostics();
	}, [isEngineReady, refreshDiagnostics]);

	useEffect(() => {
		void refreshRemoteState();
	}, [refreshRemoteState, refreshSignal]);

	useEffect(() => {
		const timer = window.setInterval(refreshDiagnostics, 500);
		return () => window.clearInterval(timer);
	}, [refreshDiagnostics]);

	async function redeployNow() {
		try {
			await Rime.deploy();
		}
		catch {
			// Error details are recorded by rime.ts diagnostics.
		}
		onDeployMutation();
		await refreshRemoteState();
	}

	async function invalidateCache() {
		try {
			const snapshot = await Rime.invalidateDeployCache();
			setCache(snapshot);
			document.documentElement.dataset["yuneDeployCacheFresh"] = String(snapshot.cacheFresh);
			onDeployMutation();
		}
		catch {
			// Error details are recorded by rime.ts diagnostics.
		}
		refreshDiagnostics();
	}

	async function applyFreeformOption() {
		try {
			await Rime.setOption(optionName.trim(), optionValue === "true");
		}
		catch {
			// Error details are recorded by rime.ts diagnostics.
		}
		refreshDiagnostics();
	}

	async function applyFreeformCustomize() {
		try {
			await Rime.customizeValue(
				customizeConfigId.trim(),
				customizeKey.trim(),
				customizeValue,
			);
		}
		catch {
			// Error details are recorded by rime.ts diagnostics.
		}
		refreshDiagnostics();
	}

	const latestDiagnostics = diagnostics.persistenceDiagnostics.slice(-6).reverse();
	const latestErrors = diagnostics.actionErrors.slice(-6).reverse();
	const debugUrls = [
		"?schema=jyut6ping3",
		"?schema=luna_pinyin",
		"?schema=luna_pinyin_octagram",
		"?debug",
		"?wasmAttributionFamily=luna-core",
	];

	return <section className="yd-control-surface" data-yune-control-surface>
		<header className="yd-control-surface-header">
			<h3>{text.title}</h3>
			<div className="yd-chip-row">
				<span
					className="yd-status-pill"
					data-status={deployStatus || "idle"}
					data-yune-deploy-status-view>
					<span className="yd-status-dot" aria-hidden="true" />
					{text.deployStatus}: {deployStatus || text.idle}
				</span>
				{cache && <span
					className="yd-status-pill"
					data-status={cache.cacheFresh ? "fresh" : "stale"}
					data-yune-deploy-cache-view>
					<span className="yd-status-dot" aria-hidden="true" />
					{text.cacheFresh}: {cache.cacheFresh ? text.trueValue : text.falseValue}
				</span>}
			</div>
		</header>

		{!IS_PUBLIC_DEMO && <div className="yd-cockpit">
			<div className="yd-cockpit-actionbar">
				<div className="yd-actionbar-actions">
					<span className="yd-actionbar-label">{text.deployTitle}</span>
					<div className="yd-button-row">
						<button
							type="button"
							className="yd-button"
							data-yune-control-redeploy
							disabled={!isEngineReady}
							onClick={() => void redeployNow()}>
							{text.redeploy}
						</button>
						<button
							type="button"
							className="yd-button yd-button-danger"
							data-yune-control-invalidate-deploy-cache
							disabled={!isEngineReady}
							onClick={() => void invalidateCache()}>
							{text.invalidateCache}
						</button>
					</div>
				</div>
				<dl className="yd-fact-strip">
					<div>
						<dt>{text.schema}</dt>
						<dd>{cache?.schemaId ?? "—"}</dd>
					</div>
					<div>
						<dt>{text.dictionary}</dt>
						<dd>{cache?.dictionaryId ?? "—"}</dd>
					</div>
					<div>
						<dt>{text.deployCache}</dt>
						<dd>{cache?.actualStamp?.assetVersion ?? "—"}</dd>
					</div>
				</dl>
			</div>

			<div className="yd-cockpit-body">
				<div className="yd-cockpit-col">
					<p className="yd-cockpit-group-title">{text.configGroup}</p>

					<section className="yd-panel" data-yune-freeform-set-option>
						<header className="yd-panel-header">
							<h4>{text.setOptionTitle}</h4>
						</header>
						<div className="yd-panel-body yd-inline-form">
							<label className="yd-field">
								<span className="yd-field-label">{text.setOptionName}</span>
								<input
									className="yd-text-input"
									data-yune-freeform-set-option-name
									{...NO_AUTO_FILL}
									value={optionName}
									onChange={event => setOptionName(event.currentTarget.value)} />
							</label>
							<label className="yd-field">
								<span className="yd-field-label">{text.setOptionValue}</span>
								<select
									className="yd-schema-select"
									data-yune-freeform-set-option-value
									value={optionValue}
									onChange={event => setOptionValue(event.currentTarget.value)}>
									<option value="true">{text.trueValue}</option>
									<option value="false">{text.falseValue}</option>
								</select>
							</label>
							<button
								type="button"
								className="yd-button yd-inline-form-submit"
								data-yune-freeform-set-option-submit
								disabled={!isEngineReady}
								onClick={() => void applyFreeformOption()}>
								{text.apply}
							</button>
						</div>
					</section>

					<section className="yd-panel" data-yune-freeform-customize>
						<header className="yd-panel-header">
							<h4>{text.customizeTitle}</h4>
						</header>
						<div className="yd-panel-body">
							<p className="yd-control-warning" data-yune-freeform-customize-warning>
								{text.customizeWarning}
							</p>
							<label className="yd-field">
								<span className="yd-field-label">{text.customizeConfigId}</span>
								<input
									className="yd-text-input"
									{...NO_AUTO_FILL}
									value={customizeConfigId}
									onChange={event => setCustomizeConfigId(event.currentTarget.value)} />
							</label>
							<label className="yd-field">
								<span className="yd-field-label">{text.customizeKey}</span>
								<input
									className="yd-text-input"
									{...NO_AUTO_FILL}
									value={customizeKey}
									onChange={event => setCustomizeKey(event.currentTarget.value)} />
							</label>
							<label className="yd-field">
								<span className="yd-field-label">{text.customizeValue}</span>
								<input
									className="yd-text-input"
									{...NO_AUTO_FILL}
									value={customizeValue}
									onChange={event => setCustomizeValue(event.currentTarget.value)} />
							</label>
							<button
								type="button"
								className="yd-button"
								data-yune-freeform-customize-submit
								disabled={!isEngineReady}
								onClick={() => void applyFreeformCustomize()}>
								{text.apply}
							</button>
						</div>
					</section>

					<section className="yd-panel" data-yune-debug-url-reference>
						<header className="yd-panel-header">
							<h4>{text.debugUrlsTitle}</h4>
						</header>
						<div className="yd-panel-body">
							<ul className="yd-url-chips">
								{debugUrls.map(url => <li key={url}><code>{url}</code></li>)}
							</ul>
						</div>
					</section>
				</div>

				<div className="yd-cockpit-col">
					<p className="yd-cockpit-group-title">{text.inspectorGroup}</p>

					<section className="yd-panel" data-yune-persistence-diagnostics-panel>
						<header className="yd-panel-header">
							<h4>{text.persistenceTitle}</h4>
							{latestDiagnostics.length > 0
								&& <span className="yd-count-badge">{latestDiagnostics.length}</span>}
						</header>
						<div className="yd-panel-body">
							{latestDiagnostics.length
								? <ol className="yd-diagnostic-list">
									{latestDiagnostics.map((diagnostic, index) =>
										<li key={`${diagnostic.source}-${diagnostic.marker?.phase}-${index}`}>
											{formatDiagnostic(diagnostic)}
										</li>)}
								</ol>
								: <p className="yd-muted">{text.noDiagnostics}</p>}
						</div>
					</section>

					<section className="yd-panel" data-yune-injected-assets>
						<header className="yd-panel-header">
							<h4>{text.injectedAssetsTitle}</h4>
							{assets?.assets.length
								? <span className="yd-count-badge">{assets.assets.length}</span>
								: null}
						</header>
						<div className="yd-panel-body">
							{assets?.assets.length
								? <ol className="yd-diagnostic-list">
									{assets.assets.map((asset) =>
										<li key={asset.path} data-yune-injected-asset>
											<span className="yd-asset-path">{asset.path}</span>
											<span className="yd-asset-bytes">{asset.bytes} B</span>
										</li>)}
								</ol>
								: <p className="yd-muted">{text.noAssets}</p>}
						</div>
					</section>

					<section className="yd-panel" data-yune-raw-response-viewer>
						<header className="yd-panel-header">
							<h4>{text.rawResponseTitle}</h4>
						</header>
						<div className="yd-panel-body">
							<pre className="yd-raw-inline">{diagnostics.lastActionResult || text.rawResponseEmpty}</pre>
						</div>
					</section>

					<section className="yd-panel yd-panel--danger" data-yune-action-error-history>
						<header className="yd-panel-header">
							<h4>{text.errorHistoryTitle}</h4>
							{latestErrors.length > 0
								&& <span className="yd-count-badge yd-count-badge--danger">{latestErrors.length}</span>}
						</header>
						<div className="yd-panel-body">
							{latestErrors.length
								? <ol className="yd-diagnostic-list">
									{latestErrors.map((error, index) =>
										<li key={`${error.action}-${index}`}>{formatActionError(error)}</li>)}
								</ol>
								: <p className="yd-muted">{text.noErrors}</p>}
						</div>
					</section>
				</div>
			</div>
		</div>}
	</section>;
}

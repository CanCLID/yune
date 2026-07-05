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
				<span className="yd-square-chip" data-yune-deploy-status-view>
					{text.deployStatus}: {deployStatus || text.idle}
				</span>
				{cache && <span className="yd-square-chip" data-yune-deploy-cache-view>
					{text.cacheFresh}: {cache.cacheFresh ? text.trueValue : text.falseValue}
				</span>}
			</div>
		</header>

		<div className="yd-control-grid">
			{!IS_PUBLIC_DEMO && <>
				<section className="yd-control-card">
					<h4>{text.deployTitle}</h4>
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
					{cache && <dl className="yd-compact-facts">
						<div>
							<dt>{text.schema}</dt>
							<dd>{cache.schemaId}</dd>
						</div>
						<div>
							<dt>{text.dictionary}</dt>
							<dd>{cache.dictionaryId}</dd>
						</div>
						<div>
							<dt>{text.deployCache}</dt>
							<dd>{cache.actualStamp?.assetVersion ?? "-"}</dd>
						</div>
					</dl>}
				</section>

				<section className="yd-control-card" data-yune-persistence-diagnostics-panel>
					<h4>{text.persistenceTitle}</h4>
					{latestDiagnostics.length
						? <ol className="yd-diagnostic-list">
							{latestDiagnostics.map((diagnostic, index) =>
								<li key={`${diagnostic.source}-${diagnostic.marker?.phase}-${index}`}>
									{formatDiagnostic(diagnostic)}
								</li>)}
						</ol>
						: <p className="yd-muted">{text.noDiagnostics}</p>}
				</section>

				<section className="yd-control-card" data-yune-injected-assets>
					<h4>{text.injectedAssetsTitle}</h4>
					{assets?.assets.length
						? <ol className="yd-diagnostic-list">
							{assets.assets.map((asset) =>
								<li key={asset.path} data-yune-injected-asset>
									{asset.path} ({asset.bytes} B)
								</li>)}
						</ol>
						: <p className="yd-muted">{text.noAssets}</p>}
				</section>

				<section className="yd-control-card" data-yune-raw-response-viewer>
					<h4>{text.rawResponseTitle}</h4>
					<pre className="yd-raw-inline">{diagnostics.lastActionResult || text.rawResponseEmpty}</pre>
				</section>

				<section className="yd-control-card" data-yune-freeform-set-option>
					<h4>{text.setOptionTitle}</h4>
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
						className="yd-button"
						data-yune-freeform-set-option-submit
						disabled={!isEngineReady}
						onClick={() => void applyFreeformOption()}>
						{text.apply}
					</button>
				</section>

				<section className="yd-control-card" data-yune-freeform-customize>
					<h4>{text.customizeTitle}</h4>
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
				</section>

				<section className="yd-control-card" data-yune-debug-url-reference>
					<h4>{text.debugUrlsTitle}</h4>
					<ul>
						{debugUrls.map(url => <li key={url}><code>{url}</code></li>)}
					</ul>
				</section>

				<section className="yd-control-card" data-yune-action-error-history>
					<h4>{text.errorHistoryTitle}</h4>
					{latestErrors.length
						? <ol className="yd-diagnostic-list">
							{latestErrors.map((error, index) =>
								<li key={`${error.action}-${index}`}>{formatActionError(error)}</li>)}
						</ol>
						: <p className="yd-muted">{text.noErrors}</p>}
				</section>
			</>}
		</div>
	</section>;
}

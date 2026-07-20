import {
	CHINESE_TYPEFACE_OPTIONS,
	OUTPUT_STANDARD_OPTIONS,
	Language,
	normalizeOutputStandard,
} from "./consts";
import { Checkbox, Radio, Range, Section, Toggle } from "./Inputs";
import {
	declareWeb06ControlFanout,
	resetYuneWebStorage,
	withWeb06ControlEvent,
} from "./rime";
import { candidateLayoutText, displayLanguageText, outputStandardText, showRomanizationText, typefaceText, uiText } from "./uiText";

import type { ShowRomanization } from "./consts";
import type { PreferencesWithSetter } from "./types";
import { useEffect } from "react";
import { web06ControlAction } from "./yune-integration/private-protocol";
import {
	WEB06_ACTION_OWNER,
	web06DeployPreferenceFanout,
	web06LiveOptionFanout,
	web06SingleActionFanout,
} from "./yune-integration/web06-app-action-map";

const DICTIONARY_EXCLUDE_BY_SCHEMA = {
	jyut6ping3: ["你"],
	cangjie5: ["日"],
	luna_pinyin: ["侴"],
	luna_pinyin_octagram: ["侴"],
} as const;

const thresholdRange = { min: 0, max: 200000, step: 1000 } as const;
const languageOrder = Object.values(Language);

type PreferencesProps = PreferencesWithSetter & {
	isAsciiPunct: boolean;
	setIsAsciiPunct(checked: boolean): void;
};

export default function Preferences(prefs: PreferencesProps) {
	const outputStandard = normalizeOutputStandard(prefs.outputStandard, "hong_kong_traditional");
	const text = uiText[prefs.uiLanguage].settings;
	const dictionaryExcludeText = prefs.dictionaryExclude.join("\n");

	useEffect(() => {
		document.documentElement.dataset["yuneDictionaryExcludeCount"] = String(prefs.dictionaryExclude.length);
	}, [prefs.dictionaryExclude.length]);

	function toggleDisplayLanguage(language: Language, checked: boolean) {
		const nextLanguages = new Set(prefs.displayLanguages);
		if (checked) {
			nextLanguages.add(language);
		}
		else if (nextLanguages.size > 1) {
			nextLanguages.delete(language);
		}

		const nextPrimary = nextLanguages.has(prefs.mainLanguage)
			? prefs.mainLanguage
			: languageOrder.find(candidate => nextLanguages.has(candidate)) ?? Language.Eng;

		prefs.setDisplayLanguages(nextLanguages);
		prefs.setMainLanguage(nextPrimary);
	}

	function hardResetStorage() {
		if (window.confirm(text.hardResetConfirm)) {
			void resetYuneWebStorage();
		}
	}

	function setDictionaryExcludeText(value: string) {
		prefs.setDictionaryExclude(
			value
				.split(/\r?\n|,/)
				.map(item => item.trim())
				.filter(Boolean),
		);
	}
	function deployPreferencePlan(patch: Partial<{
		pageSize: number;
		enableCompletion: boolean;
		enableCorrection: boolean;
		enableSentence: boolean;
		enableLearning: boolean;
		combineCandidates: boolean;
		predictionNeverFirst: boolean;
		predictionThreshold: number;
		dictionaryExclude: string[];
	}>) {
		return web06DeployPreferenceFanout({
			pageSize: prefs.pageSize,
			enableCompletion: prefs.enableCompletion,
			enableCorrection: prefs.enableCorrection,
			enableSentence: prefs.enableSentence,
			enableLearning: prefs.enableLearning,
			combineCandidates: prefs.combineCandidates,
			predictionNeverFirst: prefs.predictionNeverFirst,
			predictionThreshold: prefs.predictionThreshold,
			dictionaryExclude: prefs.dictionaryExclude,
			isCangjie5: prefs.isCangjie5,
			...patch,
		});
	}
	function liveOptionPlan(patch: Partial<{
		isAsciiMode: boolean;
		isFullShape: boolean;
		outputStandard: typeof outputStandard;
		isExtendedCharset: boolean;
		isDisabled: boolean;
	}>) {
		return web06LiveOptionFanout({
			isAsciiMode: prefs.isAsciiMode,
			isFullShape: prefs.isFullShape,
			outputStandard,
			activeSchema: prefs.activeSchema,
			isExtendedCharset: prefs.isExtendedCharset,
			isDisabled: prefs.isDisabled,
			...patch,
		});
	}
	function declareDeployPreferenceChange(reason: string, patch: Parameters<typeof deployPreferencePlan>[0]) {
		declareWeb06ControlFanout(reason, deployPreferencePlan(patch));
	}

	return <section className="yd-preferences" data-yune-preferences>
		<h3 className="yd-preferences-title">{text.title}</h3>
		<div className="yd-settings-grid">
			<Section
				sectionId="active"
				title={text.engineTitle}
				description={text.engineDescription}>
				<Toggle
					label={text.autoCompletion}
					description={<>
						<span className="block">{text.autoCompletionDescription}</span>
					</>}
					checked={prefs.enableCompletion}
					setChecked={checked => {
						declareDeployPreferenceChange("preference-enable-completion", { enableCompletion: checked });
						prefs.setEnableCompletion(checked);
					}} />
				<Toggle
					label={text.autoCorrection}
					description={<>
						<span className="block">{text.autoCorrectionDescription}</span>
					</>}
					checked={prefs.enableCorrection}
					setChecked={checked => {
						declareDeployPreferenceChange("preference-enable-correction", { enableCorrection: checked });
						prefs.setEnableCorrection(checked);
					}} />
				<Toggle
					label={text.autoComposition}
					description={<>
						{text.autoCompositionExample}
						<span className="block">{text.autoCompositionOn}</span>
						<span className="block">{text.autoCompositionOff}</span>
					</>}
					checked={prefs.enableSentence}
					setChecked={checked => {
						declareDeployPreferenceChange("preference-enable-sentence", { enableSentence: checked });
						prefs.setEnableSentence(checked);
					}} />
				<Toggle label={text.userDictionary} description={text.userDictionaryDescription} checked={prefs.enableLearning} setChecked={checked => {
					declareDeployPreferenceChange("preference-enable-learning", { enableLearning: checked });
					prefs.setEnableLearning(checked);
				}} />
				<Toggle label={text.aiCandidates} description={text.aiCandidatesDescription} checked={prefs.enableAI} setChecked={checked => {
					declareWeb06ControlFanout(
						"preference-enable-ai",
						web06SingleActionFanout(
							WEB06_ACTION_OWNER.aiSettings,
							web06ControlAction("customize", [{ enableAI: checked }]),
						),
					);
					prefs.setEnableAI(checked);
				}} />
				<Toggle label={text.combineCandidates} description={text.combineCandidatesDescription} checked={prefs.combineCandidates} setChecked={checked => {
					declareDeployPreferenceChange("preference-combine-candidates", { combineCandidates: checked });
					prefs.setCombineCandidates(checked);
				}} />
				<Toggle label={text.predictionNeverFirst} description={text.predictionNeverFirstDescription} checked={prefs.predictionNeverFirst} setChecked={checked => {
					declareDeployPreferenceChange("preference-prediction-never-first", { predictionNeverFirst: checked });
					prefs.setPredictionNeverFirst(checked);
				}} />
				<Range label={text.predictionThreshold} description={text.predictionThresholdDescription} min={thresholdRange.min} max={thresholdRange.max} step={thresholdRange.step} value={prefs.predictionThreshold} setValue={value => {
					declareDeployPreferenceChange("preference-prediction-threshold", { predictionThreshold: value });
					prefs.setPredictionThreshold(value);
				}} />
				<Toggle label={text.dictionaryExclude} description={text.dictionaryExcludeDescription} checked={prefs.dictionaryExclude.length > 0} setChecked={checked => {
					const value = checked ? [...DICTIONARY_EXCLUDE_BY_SCHEMA[prefs.activeSchema]] : [];
					declareDeployPreferenceChange("preference-dictionary-exclude", { dictionaryExclude: value });
					prefs.setDictionaryExclude(value);
				}} />
				<label className="yd-field">
					<span className="yd-field-label">{text.dictionaryExcludeEditor}</span>
					<span className="yd-field-description">{text.dictionaryExcludeEditorDescription}</span>
					<input
						type="text"
						className="yd-text-input"
						data-yune-dictionary-exclude-editor
						data-yune-dictionary-exclude-count={prefs.dictionaryExclude.length}
						value={dictionaryExcludeText}
						onChange={event => withWeb06ControlEvent(event, () => {
							const value = event.currentTarget.value
								.split(/\r?\n|,/)
								.map(item => item.trim())
								.filter(Boolean);
							declareDeployPreferenceChange("preference-dictionary-exclude-editor", { dictionaryExclude: value });
							setDictionaryExcludeText(event.currentTarget.value);
						})} />
				</label>
				<div className="yd-field yd-field-row yd-field-row--split">
					<span className="yd-field-copy">
						<span className="yd-field-label">{text.hardReset}</span>
						<span className="yd-field-description">{text.hardResetDescription}</span>
					</span>
					<button
						type="button"
						className="yd-button yd-button-danger"
						onClick={hardResetStorage}>
						{text.hardResetButton}
					</button>
				</div>
			</Section>

			<Section sectionId="display" title={text.displayTitle} description={text.displayDescription}>
				<div className="yd-field">
					<div className="yd-field-label">{text.displayLanguages}</div>
					<div className="yd-language-checklist grid gap-1 sm:grid-cols-2">
						{languageOrder.map(language =>
							<Checkbox
								key={language}
								label={displayLanguageText[prefs.uiLanguage][language]}
								checked={prefs.displayLanguages.has(language)}
								setChecked={checked => toggleDisplayLanguage(language, checked)} />
						)}
					</div>
				</div>
				<Range label={text.candidatesPerPage} min={3} max={10} step={1} value={prefs.pageSize} setValue={value => {
					declareDeployPreferenceChange("preference-page-size", { pageSize: value });
					prefs.setPageSize(value);
				}} />
				<Toggle
					label={text.dictionaryDetails}
					description={text.dictionaryDetailsDescription}
					checked={prefs.showDictionaryByDefault}
					setChecked={prefs.setShowDictionaryByDefault} />
				<div className="yd-field">
					<div className="yd-field-label">{text.candidateLayout}</div>
					<div className="grid gap-1">
						{(Object.entries(candidateLayoutText[prefs.uiLanguage]) as [typeof prefs.candidateMenuLayout, string][]).map(([value, label]) =>
							<Radio
								key={value}
								name="candidateMenuLayout"
								label={label}
								state={prefs.candidateMenuLayout}
								setState={prefs.setCandidateMenuLayout}
								value={value} />
						)}
					</div>
				</div>
				<div className="yd-field">
					<div className="yd-field-label">{text.font}</div>
					<div className="grid gap-1" data-yune-typeface-picker>
						{CHINESE_TYPEFACE_OPTIONS.map(option =>
							<Radio
								key={option.id}
								name="chineseTypeface"
								label={<span
									className={`${option.className} text-[13pt] font-normal leading-snug`}
									data-yune-typeface-option={option.id}
									data-yune-typeface-option-label>
									{typefaceText[prefs.uiLanguage][option.id]}
								</span>}
								state={prefs.chineseTypeface}
								setState={prefs.setChineseTypeface}
								value={option.id} />
						)}
					</div>
				</div>
				<div className="yd-field">
					<div className="yd-field-label">{text.candidateJyutping}</div>
					<div className="grid gap-1">
						{(Object.entries(showRomanizationText[prefs.uiLanguage]) as [ShowRomanization, string][]).map(([value, label]) =>
							<Radio
								key={value}
								name="showRomanization"
								label={label}
								state={prefs.showRomanization}
								setState={prefs.setShowRomanization}
								value={value} />
						)}
					</div>
				</div>
				<Toggle label={text.reverseCodeDisplay} checked={prefs.showReverseCode} setChecked={prefs.setShowReverseCode} />
			</Section>

			<Section
				sectionId="live"
				title={text.sessionTitle}
				description={text.sessionDescription}>
				<Toggle label={text.asciiMode} description={text.asciiModeDescription} checked={prefs.isAsciiMode} setChecked={checked => {
					declareWeb06ControlFanout("preference-ascii-mode", liveOptionPlan({ isAsciiMode: checked }));
					prefs.setIsAsciiMode(checked);
				}} />
				<Toggle label={text.fullShape} description={text.fullShapeDescription} checked={prefs.isFullShape} setChecked={checked => {
					declareWeb06ControlFanout("preference-full-shape", liveOptionPlan({ isFullShape: checked }));
					prefs.setIsFullShape(checked);
				}} />
				<label className="yd-field yd-field-row yd-field-row--split">
					<span className="yd-field-copy">
						<span className="yd-field-label">{text.asciiPunct}</span>
						<span className="yd-field-description">{text.asciiPunctDescription}</span>
					</span>
					<input
						type="checkbox"
						className="yd-check yd-toggle"
						data-yune-control-ascii-punct
						checked={prefs.isAsciiPunct}
						onChange={event => withWeb06ControlEvent(event, () => {
							declareWeb06ControlFanout(
								"preference-ascii-punct",
								web06SingleActionFanout(
									"rime-option:ascii_punct",
									web06ControlAction("setOption", ["ascii_punct", event.currentTarget.checked]),
								),
							);
							prefs.setIsAsciiPunct(event.currentTarget.checked);
						})} />
				</label>
				<div className="yd-field">
					<div className="yd-field-label">{text.outputStandard}</div>
					<div className="grid gap-1">
						{OUTPUT_STANDARD_OPTIONS.map(option =>
							<Radio
								key={option.id}
								name="outputStandard"
								label={outputStandardText[prefs.uiLanguage][option.id].label}
								state={outputStandard}
								setState={value => {
									declareWeb06ControlFanout("preference-output-standard", liveOptionPlan({ outputStandard: value }));
									prefs.setOutputStandard(value);
								}}
								value={option.id} />
						)}
					</div>
				</div>
				<Toggle label={text.extendedCharset} description={text.extendedCharsetDescription} checked={prefs.isExtendedCharset} setChecked={checked => {
					declareWeb06ControlFanout("preference-extended-charset", liveOptionPlan({ isExtendedCharset: checked }));
					prefs.setIsExtendedCharset(checked);
				}} />
				<Toggle label={text.disabled} description={text.disabledDescription} checked={prefs.isDisabled} setChecked={checked => {
					declareWeb06ControlFanout("preference-disabled", liveOptionPlan({ isDisabled: checked }));
					prefs.setIsDisabled(checked);
				}} />
			</Section>
		</div>
	</section>;
}

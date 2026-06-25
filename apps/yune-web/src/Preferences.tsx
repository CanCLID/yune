import {
	CHINESE_TYPEFACE_OPTIONS,
	OUTPUT_STANDARD_OPTIONS,
	Language,
	normalizeOutputStandard,
} from "./consts";
import { Checkbox, Radio, Range, Section, Toggle } from "./Inputs";
import { candidateLayoutText, displayLanguageText, outputStandardText, showRomanizationText, typefaceText, uiText } from "./uiText";

import type { ShowRomanization } from "./consts";
import type { PreferencesWithSetter } from "./types";

const DICTIONARY_EXCLUDE_BY_SCHEMA = {
	jyut6ping3: ["你"],
	cangjie5: ["日"],
	luna_pinyin: ["侴"],
} as const;

const thresholdRange = { min: 0, max: 200000, step: 1000 } as const;
const languageOrder = Object.values(Language);

export default function Preferences(prefs: PreferencesWithSetter) {
	const outputStandard = normalizeOutputStandard(prefs.outputStandard, "hk_traditional");
	const text = uiText[prefs.uiLanguage].settings;

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
					setChecked={prefs.setEnableCompletion} />
				<Toggle
					label={text.autoCorrection}
					description={<>
						<span className="block">{text.autoCorrectionDescription}</span>
					</>}
					checked={prefs.enableCorrection}
					setChecked={prefs.setEnableCorrection} />
				<Toggle
					label={text.autoComposition}
					description={<>
						{text.autoCompositionExample}
						<span className="block">{text.autoCompositionOn}</span>
						<span className="block">{text.autoCompositionOff}</span>
					</>}
					checked={prefs.enableSentence}
					setChecked={prefs.setEnableSentence} />
				<Toggle label={text.userDictionary} description={text.userDictionaryDescription} checked={prefs.enableLearning} setChecked={prefs.setEnableLearning} />
				<Toggle label={text.aiCandidates} description={text.aiCandidatesDescription} checked={prefs.enableAI} setChecked={prefs.setEnableAI} />
				<Toggle label={text.combineCandidates} description={text.combineCandidatesDescription} checked={prefs.combineCandidates} setChecked={prefs.setCombineCandidates} />
				<Toggle label={text.predictionNeverFirst} description={text.predictionNeverFirstDescription} checked={prefs.predictionNeverFirst} setChecked={prefs.setPredictionNeverFirst} />
				<Range label={text.predictionThreshold} description={text.predictionThresholdDescription} min={thresholdRange.min} max={thresholdRange.max} step={thresholdRange.step} value={prefs.predictionThreshold} setValue={prefs.setPredictionThreshold} />
				<Toggle label={text.dictionaryExclude} description={text.dictionaryExcludeDescription} checked={prefs.dictionaryExclude.length > 0} setChecked={checked => prefs.setDictionaryExclude(checked ? [...DICTIONARY_EXCLUDE_BY_SCHEMA[prefs.activeSchema]] : [])} />
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
				<Range label={text.candidatesPerPage} min={3} max={10} step={1} value={prefs.pageSize} setValue={prefs.setPageSize} />
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
				<Toggle label={text.asciiMode} description={text.asciiModeDescription} checked={prefs.isAsciiMode} setChecked={prefs.setIsAsciiMode} />
				<Toggle label={text.fullShape} description={text.fullShapeDescription} checked={prefs.isFullShape} setChecked={prefs.setIsFullShape} />
				<div className="yd-field">
					<div className="yd-field-label">{text.outputStandard}</div>
					<div className="grid gap-1">
						{OUTPUT_STANDARD_OPTIONS.map(option =>
							<Radio
								key={option.id}
								name="outputStandard"
								label={outputStandardText[prefs.uiLanguage][option.id].label}
								state={outputStandard}
								setState={prefs.setOutputStandard}
								value={option.id} />
						)}
					</div>
				</div>
				<Toggle label={text.extendedCharset} description={text.extendedCharsetDescription} checked={prefs.isExtendedCharset} setChecked={prefs.setIsExtendedCharset} />
				<Toggle label={text.disabled} description={text.disabledDescription} checked={prefs.isDisabled} setChecked={prefs.setIsDisabled} />
			</Section>
		</div>
	</section>;
}

import {
	CANDIDATE_MENU_LAYOUT_LABELS,
	CHINESE_TYPEFACE_OPTIONS,
	LANGUAGE_LABELS,
	SHOW_ROMANIZATION_LABELS,
	Language,
	ShowRomanization,
} from "./consts";
import { Checkbox, Radio, Range, Section, Toggle } from "./Inputs";

import type { PreferencesWithSetter } from "./types";

const DICTIONARY_EXCLUDE_BY_SCHEMA = {
	jyut6ping3_mobile: ["你"],
	cangjie5: ["日"],
	luna_pinyin: ["侴"],
} as const;

type OutputStandard = "hk_traditional" | "simplified";

const OUTPUT_STANDARD_LABELS: Record<OutputStandard, string> = {
	hk_traditional: "香港繁體 Hong Kong Traditional",
	simplified: "簡化字 Simplified Chinese (hk2s)",
};

const thresholdRange = { min: 0, max: 200000, step: 1000 } as const;
const languageOrder = Object.values(Language);

export default function Preferences(prefs: PreferencesWithSetter) {
	const outputStandard: OutputStandard = prefs.isSimplification ? "simplified" : "hk_traditional";

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
		<h3 className="yd-preferences-title">輸入法設定 <span>IME Settings</span></h3>
		<div className="yd-settings-grid">
			<Section
				sectionId="active"
				title={<>引擎設定 <span>ENGINE</span></>}
				description="會重新部署 schema，影響候選詞、排序、記憶同除詞。">
				<Toggle
					label="自動補詞 Auto-completion"
					description={<>
						<span className="block">例：santai → 身體健康（第二候選）</span>
					</>}
					checked={prefs.enableCompletion}
					setChecked={prefs.setEnableCompletion} />
				<Toggle
					label="自動校正 Auto-correction"
					description={<>
						<span className="block">例：nri → 你</span>
					</>}
					checked={prefs.enableCorrection}
					setChecked={prefs.setEnableCorrection} />
				<Toggle
					label="自動組句 Auto-composition"
					description={<>
						例：zidungzouhapgeoizi
						<span className="block">開 → 自動組合句子（第一候選）</span>
						<span className="block">關 → 自動zouhapgeoizi（第一候選）</span>
					</>}
					checked={prefs.enableSentence}
					setChecked={prefs.setEnableSentence} />
				<Toggle label="用户詞庫 User Dictionary" description="允許本機 userdb 學習已提交詞。" checked={prefs.enableLearning} setChecked={prefs.setEnableLearning} />
				<Toggle label="AI 候選重排 AI Candidates" description="本機第二輪候選，預設關閉，不改 classic 第一候選。" checked={prefs.enableAI} setChecked={prefs.setEnableAI} />
				<Toggle label="合併同字候選 Combine same-text candidates" description="把同文字典列合併成一個候選。" checked={prefs.combineCandidates} setChecked={prefs.setCombineCandidates} />
				<Toggle label="預測不排第一 Prediction never first" description="長預測候選不得壓過直接輸入候選。" checked={prefs.predictionNeverFirst} setChecked={prefs.setPredictionNeverFirst} />
				<Range label="預測門檻 Prediction threshold" description="提高門檻會隱藏較弱預測候選。" min={thresholdRange.min} max={thresholdRange.max} step={thresholdRange.step} value={prefs.predictionThreshold} setValue={prefs.setPredictionThreshold} />
				<Toggle label="除詞 Dictionary exclude" description="套用目前 schema 的示範除詞清單。" checked={prefs.dictionaryExclude.length > 0} setChecked={checked => prefs.setDictionaryExclude(checked ? [...DICTIONARY_EXCLUDE_BY_SCHEMA[prefs.activeSchema]] : [])} />
			</Section>

			<Section sectionId="display" title={<>顯示設定 <span>DISPLAY</span></>} description="只改候選顯示，不影響引擎輸出。">
				<div className="yd-field">
					<div className="yd-field-label">顯示語言 Display languages</div>
					<div className="grid gap-1 sm:grid-cols-2">
						{(Object.entries(LANGUAGE_LABELS) as [Language, string][]).map(([language, label]) =>
							<Checkbox
								key={language}
								label={label}
								checked={prefs.displayLanguages.has(language)}
								setChecked={checked => toggleDisplayLanguage(language, checked)} />
						)}
					</div>
				</div>
				<Range label="每頁候選詞數量 No. of Candidates Per Page" min={3} max={10} step={1} value={prefs.pageSize} setValue={prefs.setPageSize} />
				<Toggle
					label="字典註解 Dictionary details"
					description="預設顯示目前候選嘅字典註解；關閉後只會喺滑鼠移過候選時顯示。"
					checked={prefs.showDictionaryByDefault}
					setChecked={prefs.setShowDictionaryByDefault} />
				<div className="yd-field">
					<div className="yd-field-label">候選排版 Candidate Menu Layout</div>
					<div className="grid gap-1">
						{(Object.entries(CANDIDATE_MENU_LAYOUT_LABELS) as [typeof prefs.candidateMenuLayout, string][]).map(([value, label]) =>
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
					<div className="yd-field-label">字體 Font</div>
					<div className="grid gap-1" data-yune-typeface-picker>
						{CHINESE_TYPEFACE_OPTIONS.map(option =>
							<Radio
								key={option.id}
								name="chineseTypeface"
								label={<span
									className={`${option.className} text-[13pt] font-normal leading-snug`}
									data-yune-typeface-option={option.id}
									data-yune-typeface-option-label>
									{option.label}
								</span>}
								state={prefs.chineseTypeface}
								setState={prefs.setChineseTypeface}
								value={option.id} />
						)}
					</div>
				</div>
				<div className="yd-field">
					<div className="yd-field-label">候選粵拼 Candidate Jyutping</div>
					<div className="grid gap-1">
						{(Object.entries(SHOW_ROMANIZATION_LABELS) as [ShowRomanization, string][]).map(([value, label]) =>
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
				<Toggle label="反查編碼 Reverse code display" checked={prefs.showReverseCode} setChecked={prefs.setShowReverseCode} />
			</Section>

			<Section
				sectionId="live"
				title={<>即時狀態 <span>SESSION</span></>}
				description="只改目前 session 狀態，不改候選引擎設定。">
				<Toggle label="中英模式 ASCII mode" description="切換中文輸入或直接輸入英文字母。" checked={prefs.isAsciiMode} setChecked={prefs.setIsAsciiMode} />
				<Toggle label="全形 Full shape" description="切換全形或半形字母及符號。" checked={prefs.isFullShape} setChecked={prefs.setIsFullShape} />
				<div className="yd-field">
					<div className="yd-field-label">輸出字形 Output standard</div>
					<div className="grid gap-1">
						{(Object.entries(OUTPUT_STANDARD_LABELS) as [OutputStandard, string][]).map(([value, label]) =>
							<Radio
								key={value}
								name="outputStandard"
								label={label}
								state={outputStandard}
								setState={nextValue => prefs.setIsSimplification(nextValue === "simplified")}
								value={value} />
						)}
					</div>
				</div>
				<Toggle label="擴展字 Extended charset" description="允許 charset filter 顯示擴展字。" checked={prefs.isExtendedCharset} setChecked={prefs.setIsExtendedCharset} />
				<Toggle label="停用輸入法 Disabled" description="暫停 IME 處理，鍵盤直接輸入文字。" checked={prefs.isDisabled} setChecked={prefs.setIsDisabled} />
			</Section>
		</div>
	</section>;
}

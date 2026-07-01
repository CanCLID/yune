import { Language } from "./consts";

import type {
  CandidateMenuLayout,
  ChineseTypefaceId,
  OutputStandard,
  ShowRomanization,
} from "./consts";
import type { RimeSchemaId } from "./types";

export const UI_LANGUAGE_OPTIONS = [
  { id: "yue", label: "粵" },
  { id: "en", label: "En" },
] as const;

export type UiLanguage = (typeof UI_LANGUAGE_OPTIONS)[number]["id"];

export const DEFAULT_UI_LANGUAGE: UiLanguage = "yue";

export function isUiLanguage(value: string): value is UiLanguage {
  return UI_LANGUAGE_OPTIONS.some((option) => option.id === value);
}

export const uiText = {
  yue: {
    header: {
      home: "yune-web 首頁",
      title: "新韻輸入法引擎",
      footer: "yune-web 公開示範。Yune 同上游衍生資源按上游條款授權。",
      provenance: "來源記錄",
    },
    languageSwitcher: {
      label: "介面語言",
      title: "切換介面語言",
    },
    theme: {
      switchToDark: "切換至深色模式",
      switchToLight: "切換至淺色模式",
    },
    toolbar: {
      schema: "方案",
      schemaChoices: "方案相容選擇",
      reverseLookup: "反查支援",
      modeSpacer: "模式",
      asciiMode: "中英模式",
      chinese: "中文",
      ascii: "英文",
      outputStandard: "輸出字形",
      fullShape: "全半形",
      halfShape: "半形",
      fullShapeValue: "全形",
      cangjieLookup: "倉頡反查",
      cangjie3: "三代",
      cangjie5: "五代",
    },
    compose: {
      panelAria: "輸入測試",
      title: "輸入測試",
      fixedFloating: "固定浮窗",
      inputAria: "yune-web 輸入區",
      placeholder: "粵語拼音",
      loading: "載入中...",
      startupFailed: "引擎啟動失敗，請重新載入頁面。",
    },
    metrics: {
      aria: "即時引擎數據",
      lookup: "輸入延遲",
      wasmHeap: "WASM 佔用",
      peakWasmHeap: "WASM 峰值佔用",
      aiRerank: "智能重排",
      candidates: "候選",
      userdb: "用戶詞庫",
      off: "關",
      rows: "列",
      na: "N/A",
    },
    userdb: {
      title: "用戶詞庫",
      refresh: "刷新",
      refreshing: "刷新中",
      schema: "方案",
      dictionary: "詞庫",
      path: "路徑",
      bytes: "大小",
      rows: "列數",
      text: "文字",
      code: "編碼",
      commits: "提交次數",
      weight: "權重",
      tick: "時間點",
      sortBy: "排序",
      sortAscending: "升序",
      sortDescending: "降序",
      noRows: "呢個檔案未有學習記錄。",
      parseNotes: "解析備註",
      line: "第",
      rawFile: "原始檔案",
      downloadRaw: "下載",
      importRaw: "匯入",
      importingRaw: "匯入中",
      downloadRawAria: "下載原始 userdb 檔案",
      importRawAria: "匯入原始 userdb 檔案",
      noActiveFile: "未有啟用中嘅用戶詞庫檔案。",
      loadingSnapshot: "載入用戶詞庫快照中...",
      noSnapshot: "未載入用戶詞庫快照。",
    },
    inspector: {
      gateTitle: "引擎檢視",
      traceAria: "引擎檢視",
      traceOn: "追蹤開",
      traceOff: "追蹤關",
      input: "輸入",
      segmentsShort: "分段",
      raw: "原始",
      filtered: "過濾後",
      candidatesShort: "候選",
      ai: "AI",
      segments: "分段",
      spellingAlgebra: "拼寫代數",
      filters: "過濾",
      prediction: "預測",
      source: "來源",
      text: "文字",
      quality: "分數",
      empty: "未有檢視資料。開啟 TRACE 後再輸入。",
    },
    status: {
      schemaId: "方案識別碼",
      schemaName: "方案名稱",
      disabled: "停用",
      composing: "組字",
      mode: "模式",
      width: "字寬",
      output: "輸出",
      traditional: "繁體",
      punct: "標點",
      disabledValue: "停用",
      enabledValue: "啟用",
      yes: "是",
      idle: "待機",
      chinese: "中文",
      full: "全形",
      half: "半形",
      on: "開",
      off: "關",
      empty: "空",
    },
    dictionary: {
      kicker: "字典詳情",
      moreLanguages: "更多語言",
      detailsAria: "字典詳情",
    },
    settings: {
      title: "輸入法設定",
      engineTitle: "引擎設定",
      engineDescription: "會重新部署方案，影響候選詞、排序、記憶同除詞。",
      autoCompletion: "自動補詞",
      autoCompletionDescription: "例：santai → 身體健康（第二候選）",
      autoCorrection: "自動校正",
      autoCorrectionDescription: "例：nri → 你",
      autoComposition: "自動組句",
      autoCompositionExample: "例：zidungzouhapgeoizi",
      autoCompositionOn: "開 → 自動組合句子（第一候選）",
      autoCompositionOff: "關 → 自動zouhapgeoizi（第一候選）",
      userDictionary: "用戶詞庫",
      userDictionaryDescription: "允許本機詞庫學習已提交詞。",
      aiCandidates: "智能候選重排",
      aiCandidatesDescription: "本機第二輪候選，預設關閉，唔改原始第一候選。",
      combineCandidates: "合併同字候選",
      combineCandidatesDescription: "把同文字典列合併成一個候選。",
      predictionNeverFirst: "預測不排第一",
      predictionNeverFirstDescription: "長預測候選不得壓過直接輸入候選。",
      predictionThreshold: "預測門檻",
      predictionThresholdDescription: "提高門檻會隱藏較弱預測候選。",
      dictionaryExclude: "除詞",
      dictionaryExcludeDescription: "套用目前方案嘅示範除詞清單。",
      displayTitle: "顯示設定",
      displayDescription: "只改候選顯示，唔影響引擎輸出。",
      displayLanguages: "顯示語言",
      candidatesPerPage: "每頁候選詞數量",
      dictionaryDetails: "字典註解",
      dictionaryDetailsDescription:
        "預設顯示目前候選嘅字典註解；關閉後只會喺滑鼠移過候選時顯示。",
      candidateLayout: "候選排版",
      font: "字體",
      candidateJyutping: "候選粵拼",
      reverseCodeDisplay: "反查編碼",
      sessionTitle: "即時狀態",
      sessionDescription: "只改而家狀態，唔改候選引擎設定。",
      asciiMode: "中英模式",
      asciiModeDescription: "切換中文輸入或直接輸入英文字母。",
      fullShape: "全形",
      fullShapeDescription: "切換全形或半形字母及符號。",
      hardReset: "清除本機資料",
      hardResetDescription:
        "清除此瀏覽器嘅設定、快取、用戶詞庫同引擎儲存，然後重新載入頁面。",
      hardResetButton: "硬重設",
      hardResetConfirm:
        "要清除此瀏覽器嘅 yune-web 設定、快取、用戶詞庫同引擎儲存，然後重新載入頁面？",
      outputStandard: "輸出字形",
      extendedCharset: "擴展字集",
      extendedCharsetDescription: "允許字集過濾器顯示擴展字。",
      disabled: "停用輸入法",
      disabledDescription: "暫停輸入法處理，鍵盤直接輸入文字。",
    },
  },
  en: {
    header: {
      home: "yune-web home",
      title: "Yune Input Engine",
      footer:
        "yune-web public demo. Yune and upstream-derived assets are licensed under their upstream terms.",
      provenance: "Provenance",
    },
    languageSwitcher: {
      label: "Interface language",
      title: "Switch interface language",
    },
    theme: {
      switchToDark: "Switch to dark theme",
      switchToLight: "Switch to light theme",
    },
    toolbar: {
      schema: "Schema",
      schemaChoices: "Schema compatibility choices",
      reverseLookup: "Reverse lookup",
      modeSpacer: "Mode",
      asciiMode: "ASCII mode",
      chinese: "Chinese",
      ascii: "ASCII",
      outputStandard: "Output standard",
      fullShape: "Character width",
      halfShape: "Half shape",
      fullShapeValue: "Full shape",
      cangjieLookup: "Cangjie lookup",
      cangjie3: "Cangjie 3",
      cangjie5: "Cangjie 5",
    },
    compose: {
      panelAria: "Input test",
      title: "Input test",
      fixedFloating: "Fixed floating panel",
      inputAria: "yune-web composing input",
      placeholder: "Jyutping",
      loading: "Loading...",
      startupFailed: "Startup failed. Please reload the page.",
    },
    metrics: {
      aria: "Live engine metrics",
      lookup: "Input latency",
      wasmHeap: "WASM heap",
      peakWasmHeap: "Peak WASM heap",
      aiRerank: "AI rerank",
      candidates: "Candidates",
      userdb: "Userdb",
      off: "off",
      rows: "rows",
      na: "N/A",
    },
    userdb: {
      title: "User Dictionary",
      refresh: "Refresh",
      refreshing: "Refreshing",
      schema: "schema",
      dictionary: "dictionary",
      path: "path",
      bytes: "bytes",
      rows: "rows",
      text: "Text",
      code: "Code",
      commits: "Commits",
      weight: "Weight",
      tick: "Tick",
      sortBy: "Sort by",
      sortAscending: "ascending",
      sortDescending: "descending",
      noRows: "No learned rows in this file.",
      parseNotes: "Parse notes",
      line: "line",
      rawFile: "Raw file",
      downloadRaw: "Download",
      importRaw: "Import",
      importingRaw: "Importing",
      downloadRawAria: "Download raw userdb file",
      importRawAria: "Import raw userdb file",
      noActiveFile: "No active user dictionary file yet.",
      loadingSnapshot: "Loading user dictionary snapshot...",
      noSnapshot: "No user dictionary snapshot loaded.",
    },
    inspector: {
      gateTitle: "Engine Inspector",
      traceAria: "Engine Inspector",
      traceOn: "TRACE ON",
      traceOff: "TRACE OFF",
      input: "Input",
      segmentsShort: "Seg",
      raw: "Raw",
      filtered: "Filtered",
      candidatesShort: "Cands",
      ai: "AI",
      segments: "Segments",
      spellingAlgebra: "Spelling algebra",
      filters: "Filters",
      prediction: "Prediction",
      source: "Source",
      text: "Text",
      quality: "Quality",
      empty: "No inspector data yet. Type while trace is on.",
    },
    status: {
      schemaId: "Schema ID",
      schemaName: "Schema name",
      disabled: "Disabled",
      composing: "Composing",
      mode: "Mode",
      width: "Width",
      output: "Output",
      traditional: "Traditional",
      punct: "Punct",
      disabledValue: "disabled",
      enabledValue: "enabled",
      yes: "yes",
      idle: "idle",
      chinese: "Chinese",
      full: "full",
      half: "half",
      on: "on",
      off: "off",
      empty: "empty",
    },
    dictionary: {
      kicker: "Dictionary",
      moreLanguages: "More Languages",
      detailsAria: "dictionary details",
    },
    settings: {
      title: "IME Settings",
      engineTitle: "Engine",
      engineDescription:
        "Redeploys the schema and changes candidates, ranking, learning, and exclusions.",
      autoCompletion: "Auto-completion",
      autoCompletionDescription:
        "Example: santai → 身體健康 as the second candidate",
      autoCorrection: "Auto-correction",
      autoCorrectionDescription: "Example: nri → 你",
      autoComposition: "Auto-composition",
      autoCompositionExample: "Example input: zidungzouhapgeoizi",
      autoCompositionOn: "On → 自動組合句子",
      autoCompositionOff: "Off → 自動zouhapgeoizi",
      userDictionary: "User Dictionary",
      userDictionaryDescription:
        "Allows the local userdb to learn committed words.",
      aiCandidates: "AI Candidates",
      aiCandidatesDescription:
        "Local second-pass candidates. Default off; does not change the classic first candidate.",
      combineCandidates: "Combine same-text candidates",
      combineCandidatesDescription:
        "Merges dictionary rows with the same text into one candidate.",
      predictionNeverFirst: "Prediction never first",
      predictionNeverFirstDescription:
        "Prevents long predictions from outranking direct input candidates.",
      predictionThreshold: "Prediction threshold",
      predictionThresholdDescription:
        "Higher thresholds hide weaker prediction candidates.",
      dictionaryExclude: "Dictionary exclude",
      dictionaryExcludeDescription:
        "Applies the demo exclusion list for the current schema.",
      displayTitle: "Display",
      displayDescription:
        "Only changes candidate display; does not affect engine output.",
      displayLanguages: "Display languages",
      candidatesPerPage: "No. of Candidates Per Page",
      dictionaryDetails: "Dictionary details",
      dictionaryDetailsDescription:
        "Shows dictionary details for the current candidate by default; when off, details only appear on hover.",
      candidateLayout: "Candidate Menu Layout",
      font: "Font",
      candidateJyutping: "Candidate Jyutping",
      reverseCodeDisplay: "Reverse code display",
      sessionTitle: "Session",
      sessionDescription:
        "Only changes the current session state; does not change candidate engine settings.",
      asciiMode: "ASCII mode",
      asciiModeDescription:
        "Switches between Chinese input and direct English letters.",
      fullShape: "Full shape",
      fullShapeDescription:
        "Switches full-width or half-width letters and punctuation.",
      hardReset: "Clear local data",
      hardResetDescription:
        "Clears this browser's settings, cache, user dictionary, and engine storage, then reloads the page.",
      hardResetButton: "Hard reset",
      hardResetConfirm:
        "Clear this browser's yune-web settings, cache, user dictionary, and engine storage, then reload the page?",
      outputStandard: "Output standard",
      extendedCharset: "Extended charset",
      extendedCharsetDescription:
        "Allows the charset filter to show extended characters.",
      disabled: "Disabled",
      disabledDescription:
        "Pauses IME handling so keyboard input goes directly into the text area.",
    },
  },
} as const;

export const schemaText: Record<
  UiLanguage,
  Record<
    RimeSchemaId,
    {
      label: string;
      reverseLookups: readonly { trigger: string; schema: string }[];
    }
  >
> = {
  yue: {
    jyut6ping3: {
      label: "粵語拼音",
      reverseLookups: [
        { trigger: "`", schema: "朙月拼音" },
        { trigger: "`vl", schema: "粵語兩分" },
        { trigger: "`vc", schema: "倉頡五代" },
      ],
    },
    cangjie5: {
      label: "倉頡五代",
      reverseLookups: [{ trigger: "`…;", schema: "粵拼" }],
    },
    luna_pinyin: {
      label: "朙月拼音",
      reverseLookups: [{ trigger: "`…;", schema: "倉頡五代" }],
    },
    luna_pinyin_octagram: {
      label: "朙月拼音 + Octagram",
      reverseLookups: [{ trigger: "`…;", schema: "倉頡五代" }],
    },
  },
  en: {
    jyut6ping3: {
      label: "Jyutping",
      reverseLookups: [
        { trigger: "`", schema: "Luna Pinyin" },
        { trigger: "`vl", schema: "Cantonese two-stroke" },
        { trigger: "`vc", schema: "Cangjie 5" },
      ],
    },
    cangjie5: {
      label: "Cangjie 5",
      reverseLookups: [{ trigger: "`…;", schema: "Jyutping" }],
    },
    luna_pinyin: {
      label: "Luna Pinyin",
      reverseLookups: [{ trigger: "`…;", schema: "Cangjie 5" }],
    },
    luna_pinyin_octagram: {
      label: "Luna Pinyin + Octagram",
      reverseLookups: [{ trigger: "`…;", schema: "Cangjie 5" }],
    },
  },
};

export const outputStandardText: Record<
  UiLanguage,
  Record<OutputStandard, { label: string; shortLabel: string }>
> = {
  yue: {
    opencc_traditional: { label: "傳統漢字", shortLabel: "傳" },
    hong_kong_traditional: { label: "香港字形", shortLabel: "港" },
    taiwan_traditional: { label: "台灣字形", shortLabel: "台" },
    mainland_simplified: { label: "大陆简化字", shortLabel: "简" },
  },
  en: {
    opencc_traditional: {
      label: "OpenCC Traditional",
      shortLabel: "Traditional",
    },
    hong_kong_traditional: {
      label: "Hong Kong Traditional",
      shortLabel: "HK Trad.",
    },
    taiwan_traditional: {
      label: "Taiwan Traditional",
      shortLabel: "Taiwan Trad.",
    },
    mainland_simplified: {
      label: "Mainland Simplified",
      shortLabel: "Simplified",
    },
  },
};

export const displayLanguageText: Record<
  UiLanguage,
  Record<Language, string>
> = {
  yue: {
    [Language.Eng]: "英語",
    [Language.Hin]: "印地語",
    [Language.Ind]: "印尼語",
    [Language.Nep]: "尼泊爾語",
    [Language.Urd]: "烏爾都語",
  },
  en: {
    [Language.Eng]: "English",
    [Language.Hin]: "Hindi",
    [Language.Ind]: "Indonesian",
    [Language.Nep]: "Nepali",
    [Language.Urd]: "Urdu",
  },
};

export const candidateLayoutText: Record<
  UiLanguage,
  Record<CandidateMenuLayout, string>
> = {
  yue: {
    horizontal: "橫排",
    vertical: "直排",
  },
  en: {
    horizontal: "Horizontal",
    vertical: "Vertical",
  },
};

export const showRomanizationText: Record<
  UiLanguage,
  Record<ShowRomanization, string>
> = {
  yue: {
    always: "顯示",
    reverse_only: "只限反查",
    never: "隱藏",
  },
  en: {
    always: "Always Show",
    reverse_only: "Only in Reverse Lookup",
    never: "Hide",
  },
};

export const typefaceText: Record<
  UiLanguage,
  Record<ChineseTypefaceId, string>
> = {
  yue: {
    "chiron-hei-hk": "昭源黑體",
    "chiron-sung-hk": "昭源宋體",
    "chiron-goround-tc": "昭源環方",
    "chocolate-classical-sans": "朱古力黑體",
    "lxgw-wenkai-tc": "霞鶩文楷 TC",
    "lxgw-wenkai-mono-tc": "霞鶩文楷等寬 TC",
    iansui: "芫荽",
    huninn: "粉圓",
    "bpmf-huninn": "注音粉圓",
    "wdxl-lubrifont-tc": "滑油字",
  },
  en: {
    "chiron-hei-hk": "Chiron Hei HK",
    "chiron-sung-hk": "Chiron Sung HK",
    "chiron-goround-tc": "Chiron GoRound TC",
    "chocolate-classical-sans": "Chocolate Classical Sans",
    "lxgw-wenkai-tc": "LXGW WenKai TC",
    "lxgw-wenkai-mono-tc": "LXGW WenKai Mono TC",
    iansui: "Iansui",
    huninn: "Huninn",
    "bpmf-huninn": "Bpmf Huninn",
    "wdxl-lubrifont-tc": "WDXL Lubrifont TC",
  },
};

export const definitionLanguageNameText = displayLanguageText;

export const dictionaryMetaText = {
  yue: {
    changedTone: "變音",
    litColReadings: {
      lit: "文讀",
      col: "白讀",
    },
    registers: {
      wri: "書面語",
      ver: "口語",
      for: "公文體",
      lzh: "文言",
    },
    partsOfSpeech: {
      n: "名詞",
      v: "動詞",
      adj: "形容詞",
      adv: "副詞",
      morph: "語素",
      mw: "量詞",
      part: "助詞",
      oth: "其他",
      x: "非語素",
    },
    labels: {
      abbrev: "簡稱",
      astro: "天文",
      ChinMeta: "干支",
      horo: "星座",
      org: "機構",
      person: "人名",
      place: "地名",
      reli: "宗教",
      rare: "罕見",
      composition: "詞組",
    },
    otherData: {
      normalized: "標準字形",
      written: "書面語",
      vernacular: "口語",
      collocation: "配搭",
    },
  },
  en: {
    changedTone: "changed tone",
    litColReadings: {
      lit: "literary reading",
      col: "colloquial reading",
    },
    registers: {
      wri: "written",
      ver: "vernacular",
      for: "formal",
      lzh: "classical Chinese",
    },
    partsOfSpeech: {
      n: "noun",
      v: "verb",
      adj: "adjective",
      adv: "adverb",
      morph: "morpheme",
      mw: "measure word",
      part: "particle",
      oth: "other",
      x: "non-morpheme",
    },
    labels: {
      abbrev: "abbreviation",
      astro: "astronomy",
      ChinMeta: "sexagenary cycle",
      horo: "horoscope",
      org: "organisation",
      person: "person name",
      place: "place name",
      reli: "religion",
      rare: "rare",
      composition: "compound",
    },
    otherData: {
      normalized: "Standard Form",
      written: "Written Form",
      vernacular: "Vernacular Form",
      collocation: "Collocation",
    },
  },
} as const;

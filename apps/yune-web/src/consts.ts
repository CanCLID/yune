import type { CandidateEntry } from "./CandidateInfo";
import type { Preferences, RimeSchemaId } from "./types";

export enum Language {
	Eng = "eng",
	Hin = "hin",
	Ind = "ind",
	Nep = "nep",
	Urd = "urd",
}

export enum ShowRomanization {
	Always = "always",
	ReverseOnly = "reverse_only",
	Never = "never",
}

export const YUNE_PUBLIC_DEMO = import.meta.env.VITE_YUNE_PUBLIC_DEMO === "1";

export interface SchemaOption {
	id: RimeSchemaId;
	label: string;
	schemaName: string;
	secondaryLabel: string;
	reverseLookup: string;
}

export const SCHEMA_OPTIONS: readonly SchemaOption[] = [
	{
		id: "jyut6ping3_mobile",
		label: "粵語拼音 Jyutping",
		schemaName: "粵語拼音",
		secondaryLabel: "Jyutping",
		reverseLookup: "`zhe -> 這（普通話 / luna_pinyin 反查）; `vl... -> 粵語兩分; `vc... -> 倉頡",
	},
	{
		id: "cangjie5",
		label: "倉頡五代 Cangjie 5",
		schemaName: "倉頡五代",
		secondaryLabel: "Cangjie 5",
		reverseLookup: "`nei; -> 你（粵拼反查）",
	},
	{
		id: "luna_pinyin",
		label: "朙月拼音 Luna Pinyin",
		schemaName: "朙月拼音",
		secondaryLabel: "Luna Pinyin",
		reverseLookup: "`a; -> 日（倉頡反查）",
	},
];

export const PUBLIC_SCHEMA_OPTIONS: readonly SchemaOption[] = SCHEMA_OPTIONS.filter(
	option => option.id === "jyut6ping3_mobile",
);

export type CandidateMenuLayout = "horizontal" | "vertical";

export const CANDIDATE_MENU_LAYOUT_LABELS: Record<CandidateMenuLayout, string> = {
	horizontal: "橫排 Horizontal",
	vertical: "直排 Vertical",
};

export const CHINESE_TYPEFACE_OPTIONS = [
	{
		id: "chiron-hei-hk",
		label: "昭源黑體 Chiron Hei HK",
		className: "font-chinese-chiron-hei-hk",
	},
	{
		id: "chiron-sung-hk",
		label: "昭源宋體 Chiron Sung HK",
		className: "font-chinese-chiron-sung-hk",
	},
	{
		id: "chiron-goround-tc",
		label: "昭源環方 Chiron GoRound TC",
		className: "font-chinese-chiron-goround-tc",
	},
	{
		id: "chocolate-classical-sans",
		label: "朱古力黑體 Chocolate Classical Sans",
		className: "font-chinese-chocolate-classical-sans",
	},
	{
		id: "lxgw-wenkai-tc",
		label: "霞鶩文楷 TC LXGW WenKai TC",
		className: "font-chinese-lxgw-wenkai-tc",
	},
	{
		id: "lxgw-wenkai-mono-tc",
		label: "霞鶩文楷等寬 TC LXGW WenKai Mono TC",
		className: "font-chinese-lxgw-wenkai-mono-tc",
	},
	{
		id: "iansui",
		label: "芫荽 Iansui",
		className: "font-chinese-iansui",
	},
	{
		id: "huninn",
		label: "粉圓 Huninn",
		className: "font-chinese-huninn",
	},
	{
		id: "bpmf-huninn",
		label: "注音粉圓 Bpmf Huninn",
		className: "font-chinese-bpmf-huninn",
	},
	{
		id: "wdxl-lubrifont-tc",
		label: "滑油字 WDXL Lubrifont TC",
		className: "font-chinese-wdxl-lubrifont-tc",
	},
] as const;

export type ChineseTypefaceId = typeof CHINESE_TYPEFACE_OPTIONS[number]["id"];

export const CHINESE_TYPEFACE_BY_ID = Object.fromEntries(
	CHINESE_TYPEFACE_OPTIONS.map(option => [option.id, option]),
) as Record<ChineseTypefaceId, typeof CHINESE_TYPEFACE_OPTIONS[number]>;

export const LANGUAGE_CODES: Record<Language, string> = {
	[Language.Eng]: "en",
	[Language.Hin]: "hi",
	[Language.Ind]: "id",
	[Language.Nep]: "ne",
	[Language.Urd]: "ur",
};

export const LANGUAGE_NAMES: Record<Language, string> = {
	[Language.Eng]: "English",
	[Language.Hin]: "Hindi",
	[Language.Ind]: "Indonesian",
	[Language.Nep]: "Nepali",
	[Language.Urd]: "Urdu",
};

export const LANGUAGE_LABELS: Record<Language, string> = {
	[Language.Eng]: "英語 English",
	[Language.Hin]: "印地語 Hindi",
	[Language.Ind]: "印尼語 Indonesian",
	[Language.Nep]: "尼泊爾語 Nepali",
	[Language.Urd]: "烏爾都語 Urdu",
};

export const SHOW_ROMANIZATION_LABELS: Record<ShowRomanization, string> = {
	[ShowRomanization.Always]: "顯示 Always Show",
	[ShowRomanization.ReverseOnly]: "僅反查 Only in Reverse Lookup",
	[ShowRomanization.Never]: "隱藏 Hide",
};

export const DEFAULT_PREFERENCES: Preferences = {
	displayLanguages: new Set([Language.Eng]),
	mainLanguage: Language.Eng,
	pageSize: 6,
	chineseTypeface: "chiron-sung-hk",
	candidateMenuLayout: "horizontal",
	isCandidatePanelFixed: false,
	showDictionaryByDefault: true,
	showRomanization: ShowRomanization.Always,
	enableCompletion: true,
	enableCorrection: false,
	enableSentence: true,
	enableLearning: true,
	enableAI: false,
	// M20 playground default keeps the grouped candidate demo path visible; the raw mobile asset can still be set false.
	combineCandidates: true,
	predictionNeverFirst: true,
	predictionThreshold: 0,
	activeSchema: "jyut6ping3_mobile",
	isExtendedCharset: false,
	isDisabled: false,
	dictionaryExclude: [],
	isAsciiMode: false,
	isFullShape: false,
	isSimplification: false,
	showReverseCode: true,
	isCangjie5: true,
};

export const NO_AUTO_FILL = {
	autoComplete: "off",
	autoCorrect: "off",
	autoCapitalize: "off",
	spellCheck: "false",
} as const;

export const definitionLayout = [[Language.Eng, Language.Ind], [Language.Hin, Language.Nep], [Language.Urd]];

export const otherData: Record<string, Exclude<keyof CandidateEntry["properties"], "definition">> = {
	"Standard Form 標準字形": "normalized",
	"Written Form 書面語": "written",
	"Vernacular Form 口語": "vernacular",
	"Collocation 配搭": "collocation",
};

export const litColReadings: Record<string, string | undefined> = {
	lit: "literary reading 文讀",
	col: "colloquial reading 白讀",
};

export const registers: Record<string, string | undefined> = {
	wri: "written 書面語",
	ver: "vernacular 口語",
	for: "formal 公文體",
	lzh: "classical Chinese 文言",
};

export const partsOfSpeech: Record<string, string | undefined> = {
	n: "noun 名詞",
	v: "verb 動詞",
	adj: "adjective 形容詞",
	adv: "adverb 副詞",
	morph: "morpheme 語素",
	mw: "measure word 量詞",
	part: "particle 助詞",
	oth: "other 其他",
	x: "non-morpheme 非語素",
};

export const labels: Record<string, string | undefined> = {
	abbrev: "abbreviation 簡稱",
	astro: "astronomy 天文",
	ChinMeta: "sexagenary cycle 干支",
	horo: "horoscope 星座",
	org: "organisation 機構",
	person: "person 人名",
	place: "place 地名",
	reli: "religion 宗教",
	rare: "rare 罕見",
	composition: "compound 詞組",
};

export const checkColumns: (keyof CandidateEntry["properties"])[] = [
	"partOfSpeech",
	"register",
	"normalized",
	"written",
	"vernacular",
	"collocation",
];

export const RIME_KEY_MAP: Record<string, string | undefined> = {
	"Escape": "Escape",
	"F4": "F4",
	"Backspace": "BackSpace",
	"Delete": "Delete",
	"Tab": "Tab",
	"Enter": "Return",
	"Home": "Home",
	"End": "End",
	"PageUp": "Page_Up",
	"PageDown": "Page_Down",
	"ArrowUp": "Up",
	"ArrowRight": "Right",
	"ArrowDown": "Down",
	"ArrowLeft": "Left",
	"~": "asciitilde",
	"`": "quoteleft",
	"!": "exclam",
	"@": "at",
	"#": "numbersign",
	"$": "dollar",
	"%": "percent",
	"^": "asciicircum",
	"&": "ampersand",
	"*": "asterisk",
	"(": "parenleft",
	")": "parenright",
	"-": "minus",
	"_": "underscore",
	"+": "plus",
	"=": "equal",
	"{": "braceleft",
	"[": "bracketleft",
	"}": "braceright",
	"]": "bracketright",
	":": "colon",
	";": "semicolon",
	'"': "quotedbl",
	"'": "apostrophe",
	"|": "bar",
	"\\": "backslash",
	"<": "less",
	",": "comma",
	">": "greater",
	".": "period",
	"?": "question",
	"/": "slash",
	" ": "space",
};

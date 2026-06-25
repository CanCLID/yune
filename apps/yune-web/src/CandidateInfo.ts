import { LANGUAGE_CODES, Language, checkColumns } from "./consts";
import { definitionLanguageNameText, dictionaryMetaText } from "./uiText";
import { ConsumedString, nonEmptyArrayOrUndefined, parseCSV } from "./utils";

import type { InterfacePreferences } from "./types";

type KeyNameValue = [key: string, name: string, value: string];

export default class CandidateInfo {
	isReverseLookup: boolean;
	note: string;
	entries: CandidateEntry[];

	constructor(public label: string, public text: string, commentString = "", public source?: string) {
		const comment = new ConsumedString(normalizeCommentControls(commentString));
		this.isReverseLookup = comment.consume("\v");
		this.note = stripCommentControls(comment.consumeUntil("\f"));
		this.entries = comment.isNotEmpty
			? comment.consume("\r")
				? String(comment).split("\r").map(csv => new CandidateEntry(csv))
				: String(comment)
					.split("\f")
					.map(pron => stripCommentControls(pron).replace(/; $/g, ""))
					.filter(Boolean)
					.map(pron => new CandidateEntry({ honzi: text, jyutping: pron }))
			: [];
	}

	get matchedEntries() {
		return nonEmptyArrayOrUndefined(this.entries.filter(entry => entry.matchInputBuffer === "1"));
	}

	hasDictionaryEntry(preferences: InterfacePreferences) {
		return this.entries.some(entry => entry.isDictionaryEntry(preferences));
	}

	get isAi() {
		return this.source?.startsWith("ai:") ?? false;
	}

	inlineDefinitions(preferences: InterfacePreferences) {
		const matchedEntries: CandidateEntry[] = [...(this.matchedEntries ?? [])];
		const orderedEntries = [
			...matchedEntries,
			...this.entries.filter(entry => !matchedEntries.includes(entry)),
		];
		for (const entry of orderedEntries) {
			const definitions = entry.inlineDefinitions(preferences);
			if (definitions) {
				return definitions;
			}
		}
		return undefined;
	}
}

function normalizeCommentControls(value: string): string {
	return value
		.replace(/\\f/g, "\f")
		.replace(/\\r/g, "\r")
		.replace(/\\v/g, "\v");
}

function stripCommentControls(value: string | undefined): string {
	return normalizeCommentControls(value ?? "").replace(/[\f\r\v]/g, "").trim();
}

export class CandidateEntry {
	matchInputBuffer?: string;
	honzi?: string;
	jyutping?: string;
	pronOrder?: string;
	sandhi?: string;
	litColReading?: string;
	properties: {
		partOfSpeech?: string;
		register?: string;
		label?: string;
		normalized?: string;
		written?: string;
		vernacular?: string;
		collocation?: string;
		definition: Partial<Record<Language, string>>;
	};

	isJyutpingOnly: boolean;

	constructor(value: string | { honzi: string; jyutping: string }) {
		if ((this.isJyutpingOnly = typeof value === "object")) {
			this.honzi = stripCommentControls(value.honzi);
			this.jyutping = stripCommentControls(value.jyutping);
			this.properties = { definition: {} };
			return;
		}
		// dprint-ignore
		const [
			matchInputBuffer, honzi, jyutping, pronOrder, sandhi, litColReading,
			partOfSpeech, register, label, normalized, written, vernacular, collocation,
			eng, urd, nep, hin, ind
		] = parseCSV(value);
		this.matchInputBuffer = stripCommentControls(matchInputBuffer);
		this.honzi = stripCommentControls(honzi);
		this.jyutping = stripCommentControls(jyutping)?.replace(/\d(?!$)/g, "$& ");
		this.pronOrder = stripCommentControls(pronOrder);
		this.sandhi = stripCommentControls(sandhi);
		this.litColReading = stripCommentControls(litColReading);
		// dprint-ignore
		this.properties = {
			partOfSpeech: stripCommentControls(partOfSpeech),
			register: stripCommentControls(register),
			label: stripCommentControls(label),
			normalized: stripCommentControls(normalized),
			written: stripCommentControls(written),
			vernacular: stripCommentControls(vernacular),
			collocation: stripCommentControls(collocation),
			definition: {
				eng: stripCommentControls(eng),
				urd: stripCommentControls(urd),
				nep: stripCommentControls(nep),
				hin: stripCommentControls(hin),
				ind: stripCommentControls(ind),
			}
		};
	}

	pronunciationType(preferences: InterfacePreferences) {
		const text = dictionaryMetaText[preferences.uiLanguage];
		const types: string[] = [];
		if (this.sandhi === "1") types.push(text.changedTone);
		if (this.litColReading! in text.litColReadings) types.push(text.litColReadings[this.litColReading as keyof typeof text.litColReadings]!);
		return types.length ? `(${types.join(", ")})` : undefined;
	}

	formattedPartsOfSpeech(preferences: InterfacePreferences) {
		const text = dictionaryMetaText[preferences.uiLanguage];
		return nonEmptyArrayOrUndefined([
			...new Set(
				this.properties.partOfSpeech?.split(" ").map(
					partOfSpeech => text.partsOfSpeech[partOfSpeech as keyof typeof text.partsOfSpeech] || partOfSpeech,
				),
			),
		]);
	}

	formattedRegister(preferences: InterfacePreferences) {
		const text = dictionaryMetaText[preferences.uiLanguage];
		return text.registers[this.properties.register as keyof typeof text.registers];
	}

	formattedLabels(preferences: InterfacePreferences) {
		const text = dictionaryMetaText[preferences.uiLanguage];
		return nonEmptyArrayOrUndefined([
			...new Set(
				this.properties.label?.split(" ").flatMap(word => {
					for (const part of word.split("_")) {
						const label = text.labels[part as keyof typeof text.labels];
						if (label) return [`(${label})`];
					}
					return [];
				}),
			),
		]);
	}

	otherData(preferences: InterfacePreferences) {
		const text = dictionaryMetaText[preferences.uiLanguage];
		return nonEmptyArrayOrUndefined<KeyNameValue>(
			Object.entries(text.otherData).flatMap(([key, name]) => {
				const propertyKey = key as Exclude<keyof CandidateEntry["properties"], "definition">;
				return this.properties[propertyKey]
					? [[key, name, this.properties[propertyKey]!]]
					: []
			}),
		);
	}

	otherLanguages(preferences: InterfacePreferences) {
		return nonEmptyArrayOrUndefined<KeyNameValue>(
			[...preferences.displayLanguages].flatMap(language =>
				language !== preferences.mainLanguage && this.properties.definition[language]
					? [[LANGUAGE_CODES[language], definitionLanguageNameText[preferences.uiLanguage][language], this.properties.definition[language]!]]
					: []
			),
		);
	}

	inlineDefinitions(preferences: InterfacePreferences) {
		const languages = [
			preferences.mainLanguage,
			...[...preferences.displayLanguages].filter(language => language !== preferences.mainLanguage),
		];
		return nonEmptyArrayOrUndefined<KeyNameValue>(
			languages.flatMap(language =>
				this.properties.definition[language]
					? [[LANGUAGE_CODES[language], definitionLanguageNameText[preferences.uiLanguage][language], this.properties.definition[language]!]]
					: []
			),
		);
	}

	isDictionaryEntry(preferences: InterfacePreferences) {
		return !this.isJyutpingOnly && (checkColumns.some(key => this.properties[key])
			|| [...preferences.displayLanguages].some(language => this.properties.definition[language]));
	}
}

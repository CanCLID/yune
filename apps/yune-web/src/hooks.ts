import { useCallback, useEffect, useRef, useState } from "react";

import { CHINESE_TYPEFACE_BY_ID, DEFAULT_PREFERENCES, Language, PUBLIC_SCHEMA_OPTIONS, SCHEMA_OPTIONS, normalizeOutputStandard } from "./consts";
import Rime, { subscribe } from "./rime";
import { notify } from "./toast";
import { isUiLanguage } from "./uiText";

import type { ChineseTypefaceId, OutputStandard } from "./consts";
import type { Preferences, PreferencesWithSetter, RimeSchemaId } from "./types";
import type { UiLanguage } from "./uiText";
import type { Dispatch, DispatchWithoutAction, MouseEvent, SetStateAction, TouchEvent } from "react";

interface LocalStorageSerializer<T> {
	stringify(value: T): string;
	parse(value: string): T;
}

interface LocalStorageOptions<T> {
	defaultValue: T;
	serializer?: LocalStorageSerializer<T>;
}

const jsonSerializer: LocalStorageSerializer<unknown> = {
	stringify: JSON.stringify,
	parse: JSON.parse,
};

export function useLoading(): [boolean, (asyncTask: () => Promise<void>) => void, () => PromiseWithResolvers<void>] {
	const [pendingCount, setPendingCount] = useState(0);

	const runAsyncTask = useCallback((asyncTask: () => Promise<void>) => {
		async function processAsyncTask() {
			try {
				await asyncTask();
			}
			finally {
				setPendingCount(count => Math.max(0, count - 1));
			}
		}
		setPendingCount(count => count + 1);
		void processAsyncTask();
	}, []);

	const startAsyncTask = useCallback(() => {
		let resolve!: () => void;
		let reject!: () => void;
		const promise = new Promise<void>((_resolve, _reject) => {
			resolve = _resolve;
			reject = _reject;
		});
		runAsyncTask(() => promise);
		return { promise, resolve, reject };
	}, [runAsyncTask]);

	return [pendingCount > 0, runAsyncTask, startAsyncTask];
}

export function useRimeOption(option: string, defaultValue: boolean, deployStatus: number, localStorageKey?: string): [boolean, DispatchWithoutAction] {
	// eslint-disable-next-line react-hooks/rules-of-hooks
	const [value, setValue] = localStorageKey ? useLocalStorageValue(localStorageKey, { defaultValue }) : useState(defaultValue);

	useEffect(() => {
		async function setOption() {
			try {
				await Rime.setOption(option, value);
			}
			catch {
				notify("error", "套用選項", "applying the option");
			}
		}
		void setOption();
	}, [option, value, deployStatus]);

	useEffect(() =>
		subscribe("optionChanged", (rimeOption, rimeValue) => {
			if (rimeOption === option) {
				setValue(rimeValue);
			}
		}), [option, setValue]);

	return [value, useCallback(() => setValue(value => !value), [setValue])];
}

export function usePreferences() {
	return Object.fromEntries(
		Object.entries(DEFAULT_PREFERENCES).flatMap(([key, defaultValue]: [string, Preferences[keyof Preferences]]) => {
			const effectiveDefaultValue = key === "chineseTypeface"
				? legacyChineseTypefaceDefault(defaultValue as ChineseTypefaceId)
				: defaultValue;
			// eslint-disable-next-line react-hooks/rules-of-hooks
			const [optionValue, setOptionValue] = useLocalStorageValue(
				key,
				{
					defaultValue: effectiveDefaultValue,
					serializer: key === "displayLanguages"
						? {
							stringify: languages => [...languages as Set<Language>].join(),
							parse: values => {
								const parsed = new Set(values.split(",").map(value => value.trim()).filter(Boolean) as Language[]);
								return parsed.size ? parsed : new Set(DEFAULT_PREFERENCES.displayLanguages);
							},
						}
						: key === "chineseTypeface"
						? {
							stringify: String,
							parse: value => parseChineseTypeface(value, defaultValue as ChineseTypefaceId),
						}
						: key === "activeSchema"
						? {
							stringify: String,
							parse: value => parseActiveSchema(value, defaultValue as RimeSchemaId),
						}
						: key === "outputStandard"
						? {
							stringify: String,
							parse: value => parseOutputStandard(value, defaultValue as OutputStandard),
						}
						: key === "uiLanguage"
						? {
							stringify: String,
							parse: value => parseUiLanguage(value, defaultValue as UiLanguage),
						}
						: typeof defaultValue === "string"
						? {
							stringify: String,
							parse: String,
						}
						: JSON,
				},
			);
			return [[key, optionValue], [`set${key[0].toUpperCase()}${key.slice(1)}`, setOptionValue]];
		}),
	) as PreferencesWithSetter;
}

export function useLocalStorageValue<T>(
	key: string,
	{ defaultValue, serializer = jsonSerializer as LocalStorageSerializer<T> }: LocalStorageOptions<T>,
): [T, Dispatch<SetStateAction<T>>] {
	const [value, setValue] = useState<T>(() => readLocalStorageValue(key, defaultValue, serializer));

	const setStoredValue = useCallback<Dispatch<SetStateAction<T>>>((nextValue) => {
		setValue(currentValue => {
			const resolved = typeof nextValue === "function"
				? (nextValue as (value: T) => T)(currentValue)
				: nextValue;
			if (typeof window !== "undefined") {
				try {
					window.localStorage.setItem(key, serializer.stringify(resolved));
				}
				catch {
					// Local storage can be unavailable in private contexts; keep the in-memory state usable.
				}
			}
			return resolved;
		});
	}, [key, serializer]);

	return [value, setStoredValue];
}

export function useMediaQuery(query: string) {
	const [matches, setMatches] = useState(() =>
		typeof window !== "undefined" ? window.matchMedia(query).matches : false
	);

	useEffect(() => {
		const mediaQuery = window.matchMedia(query);
		const update = () => setMatches(mediaQuery.matches);
		update();
		mediaQuery.addEventListener("change", update);
		return () => mediaQuery.removeEventListener("change", update);
	}, [query]);

	return matches;
}

export function useLongPress(callback: () => void, delay = 800) {
	const timeoutRef = useRef<ReturnType<typeof setTimeout>>();

	const cancel = useCallback(() => {
		if (timeoutRef.current !== undefined) {
			clearTimeout(timeoutRef.current);
			timeoutRef.current = undefined;
		}
	}, []);

	const start = useCallback(() => {
		cancel();
		timeoutRef.current = setTimeout(() => {
			timeoutRef.current = undefined;
			callback();
		}, delay);
	}, [callback, cancel, delay]);

	useEffect(() => cancel, [cancel]);

	return {
		onMouseDown: (_event: MouseEvent) => start(),
		onMouseUp: cancel,
		onTouchStart: (_event: TouchEvent) => start(),
		onTouchEnd: cancel,
		cancel,
	};
}

function readLocalStorageValue<T>(
	key: string,
	defaultValue: T,
	serializer: LocalStorageSerializer<T>,
): T {
	if (typeof window === "undefined") {
		return defaultValue;
	}
	const stored = window.localStorage.getItem(key);
	if (stored === null) {
		return defaultValue;
	}
	try {
		return serializer.parse(stored);
	}
	catch {
		return defaultValue;
	}
}

function parseChineseTypeface(value: string, defaultValue: ChineseTypefaceId): ChineseTypefaceId {
	try {
		const parsed = JSON.parse(value) as unknown;
		if (typeof parsed === "string" && parsed in CHINESE_TYPEFACE_BY_ID) {
			return parsed as ChineseTypefaceId;
		}
	}
	catch {
		if (value in CHINESE_TYPEFACE_BY_ID) {
			return value as ChineseTypefaceId;
		}
	}
	return legacyChineseTypefaceDefault(defaultValue);
}

function parseActiveSchema(value: string, defaultValue: RimeSchemaId): RimeSchemaId {
	if (value === "jyut6ping3_mobile") {
		return "jyut6ping3";
	}
	const schemaOptions =
		import.meta.env.VITE_YUNE_PUBLIC_DEMO === "1"
			? PUBLIC_SCHEMA_OPTIONS
			: SCHEMA_OPTIONS;
	return schemaOptions.some(option => option.id === value) ? value as RimeSchemaId : defaultValue;
}

function parseOutputStandard(value: string, defaultValue: OutputStandard): OutputStandard {
	return normalizeOutputStandard(value, defaultValue);
}

function parseUiLanguage(value: string, defaultValue: UiLanguage): UiLanguage {
	return isUiLanguage(value) ? value : defaultValue;
}

function legacyChineseTypefaceDefault(defaultValue: ChineseTypefaceId): ChineseTypefaceId {
	if (typeof window === "undefined") {
		return defaultValue;
	}
	const stored = window.localStorage.getItem("chineseTypeface");
	if (stored) {
		try {
			const parsed = JSON.parse(stored) as string;
			if (parsed in CHINESE_TYPEFACE_BY_ID) {
				return parsed as ChineseTypefaceId;
			}
		}
		catch {
			if (stored in CHINESE_TYPEFACE_BY_ID) {
				return stored as ChineseTypefaceId;
			}
		}
	}
	const legacyHei = window.localStorage.getItem("isHeiTypeface");
	if (legacyHei === "true") {
		return "chiron-hei-hk";
	}
	if (legacyHei === "false") {
		return "chiron-sung-hk";
	}
	return defaultValue;
}

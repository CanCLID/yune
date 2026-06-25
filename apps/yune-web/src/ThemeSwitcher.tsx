import { useEffect } from "react";

import { useLocalStorageValue, useMediaQuery } from "./hooks";
import { uiText } from "./uiText";

import type { UiLanguage } from "./uiText";

export default function ThemeSwitcher({ uiLanguage }: { uiLanguage: UiLanguage }) {
	const systemTheme = useMediaQuery("(prefers-color-scheme: dark)") ? "dark" : "light";
	const [storedTheme, setTheme] = useLocalStorageValue<"light" | "dark" | undefined>("theme", {
		defaultValue: undefined,
		serializer: {
			stringify: v => String(v),
			parse: s => s === "dark" || s === "light" ? s : undefined,
		},
	});
	const theme = storedTheme ?? systemTheme;

	useEffect(() => {
		document.documentElement.dataset["theme"] = theme;
	}, [theme]);

	const nextTheme = theme === "dark" ? "light" : "dark";
	const label = nextTheme === "dark" ? uiText[uiLanguage].theme.switchToDark : uiText[uiLanguage].theme.switchToLight;

	return <button
		type="button"
		className="yd-theme-button yd-theme-button--icon"
		aria-label={label}
		aria-pressed={theme === "dark"}
		title={label}
		onClick={() => setTheme(nextTheme)}>
		{theme === "dark"
			? <svg aria-hidden="true" viewBox="0 0 24 24">
				<circle cx="12" cy="12" r="4" />
				<path d="M12 2v2" />
				<path d="M12 20v2" />
				<path d="M4.93 4.93l1.41 1.41" />
				<path d="M17.66 17.66l1.41 1.41" />
				<path d="M2 12h2" />
				<path d="M20 12h2" />
				<path d="M4.93 19.07l1.41-1.41" />
				<path d="M17.66 6.34l1.41-1.41" />
			</svg>
			: <svg aria-hidden="true" viewBox="0 0 24 24">
				<path d="M20.99 12.79A8.5 8.5 0 1 1 11.21 3a6.5 6.5 0 0 0 9.78 9.79Z" />
			</svg>}
	</button>;
}

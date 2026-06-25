import { useEffect } from "react";

import { useLocalStorageValue, useMediaQuery } from "./hooks";

export default function ThemeSwitcher() {
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

	return <label className="yd-theme-button">
		<input
			type="checkbox"
			checked={theme === "dark"}
			onChange={() => setTheme(theme === "dark" ? "light" : "dark")}
			className="yd-theme-switch sr-only" />
		<span className="sr-only">Theme Switcher</span>
		<span aria-hidden="true">{theme === "dark" ? "○" : "●"} THEME</span>
	</label>;
}

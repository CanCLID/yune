import { UI_LANGUAGE_OPTIONS, uiText } from "./uiText";

import type { UiLanguage } from "./uiText";
import type { Dispatch, SetStateAction } from "react";

export default function UiLanguageSwitcher({
	uiLanguage,
	setUiLanguage,
}: {
	uiLanguage: UiLanguage;
	setUiLanguage: Dispatch<SetStateAction<UiLanguage>>;
}) {
	const text = uiText[uiLanguage].languageSwitcher;
	return <div
		className="yd-language-switcher yd-segment-group"
		role="radiogroup"
		aria-label={text.label}
		title={text.title}
		data-yune-ui-language-switcher>
		{UI_LANGUAGE_OPTIONS.map(option =>
			<label
				key={option.id}
				className={`yd-segment${uiLanguage === option.id ? " yd-segment-active" : ""}`}>
				<input
					type="radio"
					className="sr-only"
					name="uiLanguage"
					value={option.id}
					checked={uiLanguage === option.id}
					onChange={() => setUiLanguage(option.id)} />
				{option.label}
			</label>
		)}
	</div>;
}

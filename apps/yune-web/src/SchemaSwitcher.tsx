import { SCHEMA_OPTIONS } from "./consts";
import { schemaText, uiText } from "./uiText";

import type { RimeSchemaId } from "./types";
import type { UiLanguage } from "./uiText";
import type { Dispatch, SetStateAction } from "react";

export default function SchemaSwitcher({
	activeSchema,
	setActiveSchema,
	uiLanguage,
	compact = false,
}: {
	activeSchema: RimeSchemaId;
	setActiveSchema: Dispatch<SetStateAction<RimeSchemaId>>;
	uiLanguage: UiLanguage;
	compact?: boolean;
}) {
	const schemaOptions = SCHEMA_OPTIONS;
	const active = schemaOptions.find(schema => schema.id === activeSchema) ?? schemaOptions[0];
	const text = uiText[uiLanguage].toolbar;

	return <div className={`yd-schema-dropdown${compact ? " yd-schema-dropdown-compact" : ""}`} data-yune-schema-switcher>
		<label className="yd-top-label" htmlFor="yune-schema-select">{text.schema}</label>
		<select
			id="yune-schema-select"
			className="yd-schema-select"
			aria-label={text.schema}
			value={active.id}
			onChange={event => setActiveSchema(event.currentTarget.value as RimeSchemaId)}>
			{schemaOptions.map(schema =>
				<option key={schema.id} value={schema.id}>
					{schemaText[uiLanguage][schema.id].label}
				</option>
			)}
		</select>
		<div className="sr-only" role="radiogroup" aria-label={text.schemaChoices}>
			{schemaOptions.map(schema =>
				<label key={schema.id}>
					<input
						type="radio"
						name="yuneSchema"
						checked={activeSchema === schema.id}
						onChange={() => setActiveSchema(schema.id)} />
					{schemaText[uiLanguage][schema.id].label}
				</label>
			)}
		</div>
	</div>;
}

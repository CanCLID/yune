import { PUBLIC_SCHEMA_OPTIONS, SCHEMA_OPTIONS, YUNE_PUBLIC_DEMO } from "./consts";

import type { RimeSchemaId } from "./types";
import type { Dispatch, SetStateAction } from "react";

export default function SchemaSwitcher({
	activeSchema,
	setActiveSchema,
	compact = false,
}: {
	activeSchema: RimeSchemaId;
	setActiveSchema: Dispatch<SetStateAction<RimeSchemaId>>;
	compact?: boolean;
}) {
	const schemaOptions = YUNE_PUBLIC_DEMO ? PUBLIC_SCHEMA_OPTIONS : SCHEMA_OPTIONS;
	if (YUNE_PUBLIC_DEMO && schemaOptions.length <= 1) {
		return null;
	}
	const active = schemaOptions.find(schema => schema.id === activeSchema) ?? schemaOptions[0];

	return <div className={`yd-schema-dropdown${compact ? " yd-schema-dropdown-compact" : ""}`} data-yune-schema-switcher>
		<label className="yd-top-label" htmlFor="yune-schema-select">方案 Schema</label>
		<select
			id="yune-schema-select"
			className="yd-schema-select"
			aria-label="方案 Schema"
			value={active.id}
			onChange={event => setActiveSchema(event.currentTarget.value as RimeSchemaId)}>
			{schemaOptions.map(schema =>
				<option key={schema.id} value={schema.id}>
					{schema.schemaName} · {schema.secondaryLabel}
				</option>
			)}
		</select>
		<div className="sr-only" role="radiogroup" aria-label="方案 Schema compatibility choices">
			{schemaOptions.map(schema =>
				<label key={schema.id}>
					<input
						type="radio"
						name="yuneSchema"
						checked={activeSchema === schema.id}
						onChange={() => setActiveSchema(schema.id)} />
					{schema.schemaName} {schema.secondaryLabel}
				</label>
			)}
		</div>
		<p className="yd-schema-hint">{active.reverseLookup}</p>
	</div>;
}

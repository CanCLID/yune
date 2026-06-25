import { outputStandardText, uiText } from "./uiText";

import type { YuneStatusSnapshot } from "./types";
import type { OutputStandard } from "./consts";
import type { UiLanguage } from "./uiText";

function boolStatus(value: boolean, enabled: string, disabled: string) {
	return `${value ? enabled : disabled}`;
}

export default function YuneStatusStrip({ status, outputStandard, uiLanguage }: { status?: YuneStatusSnapshot; outputStandard: OutputStandard; uiLanguage: UiLanguage }) {
	if (!status) {
		return null;
	}

	const text = uiText[uiLanguage].status;
	const outputLabel = outputStandardText[uiLanguage][outputStandard].shortLabel;

	return <section className="yd-status-strip" data-yune-status>
		<span className="yd-square-chip" data-yune-status-schema-id-field>
			<b>{text.schemaId}</b> {status.schema_id}
		</span>
		<span className="yd-square-chip" data-yune-status-schema data-yune-status-schema-id={status.schema_id} data-yune-status-schema-name={status.schema_name}>
			<b>{text.schemaName}</b> {status.schema_name || `(${text.empty})`}
		</span>
		<span className="yd-square-chip" data-yune-status-disabled={status.is_disabled}>
			<b>{text.disabled}</b> {boolStatus(status.is_disabled, text.disabledValue, text.enabledValue)}
		</span>
		<span className="yd-square-chip" data-yune-status-composing={status.is_composing}>
			<b>{text.composing}</b> {boolStatus(status.is_composing, text.yes, text.idle)}
		</span>
		<span className="yd-square-chip" data-yune-status-ascii={status.is_ascii_mode} data-yune-status-ascii-mode={status.is_ascii_mode}>
			<b>{text.mode}</b> {boolStatus(status.is_ascii_mode, "ASCII", text.chinese)}
		</span>
		<span className="yd-square-chip" data-yune-status-full-shape={status.is_full_shape}>
			<b>{text.width}</b> {boolStatus(status.is_full_shape, text.full, text.half)}
		</span>
		<span className="yd-square-chip" data-yune-status-simplified={status.is_simplified} data-yune-status-output-standard={outputStandard}>
			<b>{text.output}</b> {outputLabel}
		</span>
		<span className="yd-square-chip" data-yune-status-traditional={status.is_traditional}>
			<b>{text.traditional}</b> {boolStatus(status.is_traditional, text.on, text.off)}
		</span>
		<span className="yd-square-chip" data-yune-status-ascii-punct={status.is_ascii_punct}>
			<b>{text.punct}</b> {boolStatus(status.is_ascii_punct, "ASCII", text.chinese)}
		</span>
	</section>;
}

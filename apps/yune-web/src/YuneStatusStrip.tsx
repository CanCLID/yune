import type { YuneStatusSnapshot } from "./types";

function boolStatus(value: boolean, enabled: string, disabled: string) {
	return `${value ? enabled : disabled}`;
}

export default function YuneStatusStrip({ status }: { status?: YuneStatusSnapshot }) {
	if (!status) {
		return null;
	}

	const outputStandard = status.is_simplified ? "simplified" : "hk_traditional";

	return <section className="yd-status-strip" data-yune-status>
		<span className="yd-square-chip" data-yune-status-schema-id-field>
			<b>schema_id</b> {status.schema_id}
		</span>
		<span className="yd-square-chip" data-yune-status-schema data-yune-status-schema-id={status.schema_id} data-yune-status-schema-name={status.schema_name}>
			<b>schema_name</b> {status.schema_name || "(empty)"}
		</span>
		<span className="yd-square-chip" data-yune-status-disabled={status.is_disabled}>
			<b>disabled</b> {boolStatus(status.is_disabled, "disabled", "enabled")}
		</span>
		<span className="yd-square-chip" data-yune-status-composing={status.is_composing}>
			<b>composing</b> {boolStatus(status.is_composing, "yes", "idle")}
		</span>
		<span className="yd-square-chip" data-yune-status-ascii={status.is_ascii_mode} data-yune-status-ascii-mode={status.is_ascii_mode}>
			<b>mode</b> {boolStatus(status.is_ascii_mode, "ASCII", "Chinese")}
		</span>
		<span className="yd-square-chip" data-yune-status-full-shape={status.is_full_shape}>
			<b>width</b> {boolStatus(status.is_full_shape, "full", "half")}
		</span>
		<span className="yd-square-chip" data-yune-status-simplified={status.is_simplified} data-yune-status-output-standard={outputStandard}>
			<b>output</b> {boolStatus(status.is_simplified, "simplified", "HK traditional")}
		</span>
		<span className="yd-square-chip" data-yune-status-traditional={status.is_traditional}>
			<b>traditional</b> {boolStatus(status.is_traditional, "on", "off")}
		</span>
		<span className="yd-square-chip" data-yune-status-ascii-punct={status.is_ascii_punct}>
			<b>punct</b> {boolStatus(status.is_ascii_punct, "ASCII", "Chinese")}
		</span>
	</section>;
}

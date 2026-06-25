import { uiText } from "./uiText";

import type { YuneInspectorDebug } from "./types";
import type { UiLanguage } from "./uiText";

function formatScore(value: number | null | undefined) {
	if (value === null || value === undefined) {
		return "-";
	}
	return Number.isInteger(value) ? String(value) : value.toFixed(3);
}

function summary(debug?: YuneInspectorDebug) {
	const firstAudit = debug?.filter_audit[0];
	const lastAudit = debug?.filter_audit[debug.filter_audit.length - 1];
	const input = debug?.ai_staging.for_input
		?? debug?.spelling_algebra.find(algebra => algebra.input)?.input
		?? "-";
	return {
		input,
		segments: debug?.segments.length || debug?.segment_tags.length || 0,
		raw: firstAudit?.before_count ?? "-",
		filtered: lastAudit?.after_count ?? "-",
		candidates: debug?.prediction.candidates.length ?? 0,
		ai: debug?.ai_staging.state ?? "off",
	};
}

export default function YuneInspector({
	debug,
	uiLanguage,
}: {
	debug?: YuneInspectorDebug;
	uiLanguage: UiLanguage;
}) {
	const values = summary(debug);
	const text = uiText[uiLanguage].inspector;
	return <section className="yd-inspector-panel" data-yune-inspector="panel">
		<div className="yd-inspector-summary">
			<span><b>{text.input}</b> {values.input}</span>
			<span><b>{text.segmentsShort}</b> {values.segments}</span>
			<span><b>{text.raw}</b> {values.raw}</span>
			<span><b>{text.filtered}</b> {values.filtered}</span>
			<span><b>{text.candidatesShort}</b> {values.candidates}</span>
			<span><b>{text.ai}</b> {values.ai}</span>
		</div>
		{debug
			? <div className="yd-inspector-grid">
					<div className="yd-inspector-column">
						<h3>{text.segments}</h3>
						<div className="yd-chip-row" data-yune-inspector-segments>
							{debug.segments.length
								? debug.segments.map((segment, index) =>
									<span key={`${segment.tag}-${index}`} className="yd-square-chip">
										{segment.tag} {segment.start}-{segment.end}
									</span>)
								: debug.segment_tags.map(tag =>
									<span key={tag} className="yd-square-chip">{tag}</span>)}
						</div>
						<h3>{text.spellingAlgebra}</h3>
						<div className="yd-inspector-list">
							{debug.spelling_algebra.map(algebra =>
								<div key={`${algebra.translator}-${algebra.input}`} className="yd-inspector-list-row" data-yune-inspector-algebra>
									<span>{algebra.translator}</span>
									<span>{algebra.expanded_codes.slice(0, 3).join(" · ") || algebra.lookup_code || "-"}</span>
								</div>)}
						</div>
					</div>
					<div className="yd-inspector-column">
						<h3>{text.filters}</h3>
						<div className="yd-inspector-list" data-yune-inspector-filters>
							{debug.filter_audit.length
								? debug.filter_audit.map(record =>
									<div key={record.name} className="yd-inspector-list-row">
										<span>{record.name}</span>
										<span>{`${record.before_count} → ${record.after_count}`}</span>
									</div>)
								: debug.filter_pipeline.map(name =>
									<div key={name} className="yd-inspector-list-row">
										<span>{name}</span>
										<span>-</span>
									</div>)}
						</div>
						<h3>{text.prediction}</h3>
						<table className="yd-inspector-table" data-yune-inspector-prediction>
							<thead>
								<tr>
									<th>{text.source}</th>
									<th>{text.text}</th>
									<th>{text.quality}</th>
								</tr>
							</thead>
							<tbody>
								{debug.prediction.candidates.slice(0, 8).map(candidate =>
									<tr key={`${candidate.index}-${candidate.text}`}>
										<td data-yune-inspector-source>{candidate.source}</td>
										<td>{candidate.text}</td>
										<td>{formatScore(candidate.quality)}</td>
									</tr>)}
							</tbody>
						</table>
					</div>
				</div>
			: <div className="yd-inspector-empty" data-yune-inspector-empty>{text.empty}</div>}
	</section>;
}

import { OUTPUT_STANDARD_BY_ID, OUTPUT_STANDARD_OPTIONS, normalizeOutputStandard } from "./consts";
import { Segment } from "./Inputs";
import SchemaSwitcher from "./SchemaSwitcher";
import { schemaText, uiText } from "./uiText";

import type { OutputStandard } from "./consts";
import type { RimeSchemaId } from "./types";
import type { UiLanguage } from "./uiText";
import type { Dispatch, SetStateAction } from "react";

function ModeButton({
	ariaLabel,
	active,
	activeGlyph,
	inactiveGlyph,
	activeLabel,
	inactiveLabel,
	onClick,
}: {
	ariaLabel: string;
	active: boolean;
	activeGlyph: string;
	inactiveGlyph: string;
	activeLabel: string;
	inactiveLabel: string;
	onClick(): void;
}) {
	return <button
		type="button"
		className="yd-mode-button"
		aria-label={ariaLabel}
		data-active={active}
		onClick={onClick}>
		<span className="yd-mode-glyph">{active ? activeGlyph : inactiveGlyph}</span>
		<span className="yd-mode-label">{active ? activeLabel : inactiveLabel}</span>
	</button>;
}

function ReverseLookupSummary({
	activeSchema,
	uiLanguage,
}: {
	activeSchema: RimeSchemaId;
	uiLanguage: UiLanguage;
}) {
	const text = uiText[uiLanguage].toolbar;
	const lookups = schemaText[uiLanguage][activeSchema].reverseLookups;
	return <div className="yd-reverse-lookup" data-yune-reverse-lookup-summary>
		<div className="yd-top-label">{text.reverseLookup}</div>
		<ul className="yd-reverse-lookup-list" aria-label={text.reverseLookup}>
			{lookups.map(lookup =>
				<li key={`${lookup.trigger}-${lookup.schema}`} className="yd-reverse-lookup-item">
					<code>{lookup.trigger}</code>
					<span>{lookup.schema}</span>
				</li>
			)}
		</ul>
	</div>;
}

export default function Toolbar({
	isAsciiMode,
	setIsAsciiMode,
	outputStandard,
	setOutputStandard,
	isFullShape,
	setIsFullShape,
	activeSchema,
	setActiveSchema,
	isCangjie5,
	setIsCangjie5,
	uiLanguage,
}: {
	isAsciiMode: boolean;
	setIsAsciiMode: Dispatch<SetStateAction<boolean>>;
	outputStandard: OutputStandard;
	setOutputStandard: Dispatch<SetStateAction<OutputStandard>>;
	isFullShape: boolean;
	setIsFullShape: Dispatch<SetStateAction<boolean>>;
	activeSchema: RimeSchemaId;
	setActiveSchema: Dispatch<SetStateAction<RimeSchemaId>>;
	isCangjie5: boolean;
	setIsCangjie5: Dispatch<SetStateAction<boolean>>;
	uiLanguage: UiLanguage;
}) {
	const outputStandardValue = normalizeOutputStandard(outputStandard, "hong_kong_traditional");
	const currentOutputStandard = OUTPUT_STANDARD_BY_ID[outputStandardValue];
	const text = uiText[uiLanguage].toolbar;
	function cycleOutputStandard() {
		setOutputStandard(currentValue => {
			const normalizedCurrentValue = normalizeOutputStandard(currentValue, "hong_kong_traditional");
			const currentIndex = OUTPUT_STANDARD_OPTIONS.findIndex(option => option.id === normalizedCurrentValue);
			return OUTPUT_STANDARD_OPTIONS[(currentIndex + 1) % OUTPUT_STANDARD_OPTIONS.length].id;
		});
	}

	return <div className="yd-toolbar" data-yune-top-controls>
		<div className="yd-toolbar-row yd-toolbar-row-primary">
			<SchemaSwitcher
				activeSchema={activeSchema}
				setActiveSchema={setActiveSchema}
				uiLanguage={uiLanguage}
				compact />
			<div className="yd-top-field yd-mode-choice" data-yune-control="mode-buttons">
				<span className="yd-top-label yd-top-label-spacer" aria-hidden="true">{text.modeSpacer}</span>
				<div className="yd-mode-row">
					<ModeButton
						ariaLabel={text.asciiMode}
						active={!isAsciiMode}
						activeGlyph="中"
						inactiveGlyph="英"
						activeLabel={text.chinese}
						inactiveLabel={text.ascii}
						onClick={() => setIsAsciiMode(value => !value)} />
					<ModeButton
						ariaLabel={text.outputStandard}
						active
						activeGlyph={currentOutputStandard.glyph}
						inactiveGlyph={currentOutputStandard.glyph}
						activeLabel={currentOutputStandard.shortLabel}
						inactiveLabel={currentOutputStandard.shortLabel}
						onClick={cycleOutputStandard} />
					<ModeButton
						ariaLabel={text.fullShape}
						active={isFullShape}
						activeGlyph="全"
						inactiveGlyph="半"
						activeLabel={text.fullShapeValue}
						inactiveLabel={text.halfShape}
						onClick={() => setIsFullShape(value => !value)} />
				</div>
			</div>
		</div>
		<div className="yd-toolbar-row yd-toolbar-row-secondary">
			<ReverseLookupSummary
				activeSchema={activeSchema}
				uiLanguage={uiLanguage} />
			<div className="yd-top-field yd-cangjie-choice" data-yune-control="cangjie-version">
				<span className="yd-top-label">{text.cangjieLookup}</span>
				<div className="yd-segment-group" role="radiogroup" aria-label={text.cangjieLookup}>
					<Segment name="cangjieVersion" label={text.cangjie3} state={isCangjie5} setState={setIsCangjie5} value={false} />
					<Segment name="cangjieVersion" label={text.cangjie5} state={isCangjie5} setState={setIsCangjie5} value={true} />
				</div>
			</div>
		</div>
	</div>;
}

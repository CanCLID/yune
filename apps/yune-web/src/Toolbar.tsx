import { Segment } from "./Inputs";
import SchemaSwitcher from "./SchemaSwitcher";

import type { RimeSchemaId } from "./types";
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

export default function Toolbar({
	isAsciiMode,
	setIsAsciiMode,
	isSimplification,
	setIsSimplification,
	isFullShape,
	setIsFullShape,
	activeSchema,
	setActiveSchema,
	isCangjie5,
	setIsCangjie5,
}: {
	isAsciiMode: boolean;
	setIsAsciiMode: Dispatch<SetStateAction<boolean>>;
	isSimplification: boolean;
	setIsSimplification: Dispatch<SetStateAction<boolean>>;
	isFullShape: boolean;
	setIsFullShape: Dispatch<SetStateAction<boolean>>;
	activeSchema: RimeSchemaId;
	setActiveSchema: Dispatch<SetStateAction<RimeSchemaId>>;
	isCangjie5: boolean;
	setIsCangjie5: Dispatch<SetStateAction<boolean>>;
}) {
	return <div className="yd-toolbar" data-yune-top-controls>
		<SchemaSwitcher
			activeSchema={activeSchema}
			setActiveSchema={setActiveSchema}
			compact />
		<div className="yd-top-field yd-mode-choice" data-yune-control="mode-buttons">
			<span className="yd-top-label yd-top-label-spacer" aria-hidden="true">&nbsp;</span>
			<div className="yd-mode-row">
				<ModeButton
					ariaLabel="ASCII mode"
					active={!isAsciiMode}
					activeGlyph="中"
					inactiveGlyph="英"
					activeLabel="中文"
					inactiveLabel="英文 ASCII"
					onClick={() => setIsAsciiMode(value => !value)} />
				<ModeButton
					ariaLabel="Output standard"
					active={!isSimplification}
					activeGlyph="繁"
					inactiveGlyph="简"
					activeLabel="繁體 HK"
					inactiveLabel="簡體 hk2s"
					onClick={() => setIsSimplification(value => !value)} />
				<ModeButton
					ariaLabel="Full shape"
					active={!isFullShape}
					activeGlyph="半"
					inactiveGlyph="全"
					activeLabel="半形"
					inactiveLabel="全形"
					onClick={() => setIsFullShape(value => !value)} />
			</div>
		</div>
		<div className="yd-top-field yd-cangjie-choice" data-yune-control="cangjie-version">
			<span className="yd-top-label">倉頡反查 Cangjie lookup</span>
			<div className="yd-segment-group" role="radiogroup" aria-label="倉頡反查 Cangjie lookup">
				<Segment name="cangjieVersion" label="三代" state={isCangjie5} setState={setIsCangjie5} value={false} />
				<Segment name="cangjieVersion" label="五代" state={isCangjie5} setState={setIsCangjie5} value={true} />
			</div>
		</div>
	</div>;
}

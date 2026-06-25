import { NO_AUTO_FILL } from "./consts";

import type { Dispatch, ReactNode } from "react";

interface CheckboxProps {
	label: ReactNode;
	checked: boolean;
	setChecked: Dispatch<boolean>;
	description?: ReactNode;
}

interface RadioProps<T> {
	name: string;
	label: ReactNode;
	state: T;
	setState: Dispatch<T>;
	value: T;
	description?: ReactNode;
}

interface RangeProps {
	label: ReactNode;
	min: number;
	max: number;
	step: number;
	value: number;
	setValue(value: number): void;
	description?: ReactNode;
}

export function Section({ title, description, children, sectionId }: { title: ReactNode; description?: ReactNode; children: ReactNode; sectionId?: string }) {
	return <section className="yd-section" data-yune-section={sectionId}>
		<header>
			<h4 className="yd-section-title">{title}</h4>
			{description && <p className="yd-section-description">{description}</p>}
		</header>
		<div className="yd-section-body">{children}</div>
	</section>;
}

export function Toggle({ label, checked, setChecked, description }: CheckboxProps) {
	return <label className="yd-field yd-field-row yd-field-row--split">
		<span className="yd-field-copy">
			<span className="yd-field-label">{label}</span>
			{description && <span className="yd-field-description">{description}</span>}
		</span>
		<input
			type="checkbox"
			className="yd-check yd-toggle"
			{...NO_AUTO_FILL}
			checked={checked}
			onChange={event => setChecked(event.target.checked)} />
	</label>;
}

export function Checkbox({ label, checked, setChecked, description }: CheckboxProps) {
	return <label className="yd-field yd-field-row yd-field-row--inline">
		<input
			type="checkbox"
			className="yd-check"
			{...NO_AUTO_FILL}
			checked={checked}
			onChange={event => setChecked(event.target.checked)} />
		<span className="yd-field-copy">
			<span className="yd-field-label">{label}</span>
			{description && <span className="yd-field-description">{description}</span>}
		</span>
	</label>;
}

export function Range({ label, min, max, step, value, setValue, description }: RangeProps) {
	return <label className="yd-field">
		<div className="flex items-center gap-3">
			<span className="yd-field-copy">
				<span className="yd-field-label">{label}</span>
				{description && <span className="yd-field-description">{description}</span>}
			</span>
			<span className="yd-tag">{value}</span>
		</div>
		<input
			type="range"
			className="yd-slider"
			min={min}
			max={max}
			step={step}
			value={value}
			onChange={event => setValue(Number(event.target.value))} />
	</label>;
}

export function Radio<T>({ name, label, state, setState, value, description }: RadioProps<T>) {
	return <label className="yd-field yd-field-row yd-field-row--inline">
		<input
			type="radio"
			name={name}
			className="yd-choice"
			{...NO_AUTO_FILL}
			value={String(value)}
			checked={state === value}
			onChange={() => setState(value)} />
		<span className="yd-field-copy">
			<span className="yd-field-label">{label}</span>
			{description && <span className="yd-field-description">{description}</span>}
		</span>
	</label>;
}

export function Segment<T>({ name, label, state, setState, value }: RadioProps<T>) {
	const active = state === value;
	return <label className={`yd-segment${active ? " yd-segment-active" : ""}`}>
		<input
			type="radio"
			className="sr-only"
			name={name}
			{...NO_AUTO_FILL}
			value={String(value)}
			checked={state === value}
			onChange={() => setState(value)} />
		{label}
	</label>;
}

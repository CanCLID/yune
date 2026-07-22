import { useEffect, useRef } from "react";

import CandidateInfo from "./CandidateInfo";
import { ShowRomanization } from "./consts";
import { useLongPress } from "./hooks";
import { uiText } from "./uiText";
import { cancelWeb06EventFanout } from "./rime";

import type { InterfacePreferences, Web06DomEventIdentity } from "./types";
import type { MouseEvent, TouchEvent } from "react";

export default function Candidate({ isHighlighted, info, selectCandidate, deleteCandidate, prepareDeleteCandidate, showDictionary, hideDictionary, prefs }: {
	isHighlighted: boolean;
	info: CandidateInfo;
	selectCandidate(event: MouseEvent): void;
	deleteCandidate(): void;
	prepareDeleteCandidate(event: MouseEvent | TouchEvent): Web06DomEventIdentity;
	showDictionary(): void;
	hideDictionary(): void;
	prefs: InterfacePreferences;
}) {
	const justDeletedCandidate = useRef(false);
	const pendingDeleteEvent = useRef<Web06DomEventIdentity>();
	function _deleteCandidate() {
		deleteCandidate();
		pendingDeleteEvent.current = undefined;
		justDeletedCandidate.current = true;
	}
	const {
		onMouseDown: startLongPress,
		onTouchStart: startTouchLongPress,
		cancel: cancelLongPressTimer,
	} = useLongPress(_deleteCandidate, 800);
	function beginDeleteCandidate(event: MouseEvent | TouchEvent) {
		pendingDeleteEvent.current = prepareDeleteCandidate(event);
		if (event.type === "mousedown") startLongPress(event as MouseEvent);
		else startTouchLongPress(event as TouchEvent);
	}
	function cancelDeleteCandidate() {
		cancelLongPressTimer();
		if (pendingDeleteEvent.current !== undefined) {
			cancelWeb06EventFanout(pendingDeleteEvent.current, "candidate-long-press-cancelled");
			pendingDeleteEvent.current = undefined;
		}
	}
	const numOfMoves = useRef(0);
	useEffect(() => {
		numOfMoves.current = 0;
	}, [info]);
	useEffect(() => () => {
		cancelLongPressTimer();
		if (pendingDeleteEvent.current !== undefined) {
			cancelWeb06EventFanout(pendingDeleteEvent.current, "candidate-long-press-unmounted");
			pendingDeleteEvent.current = undefined;
		}
	}, [cancelLongPressTimer]);
	function _selectCandidate(event: MouseEvent) {
		if (justDeletedCandidate.current) {
			justDeletedCandidate.current = false;
		}
		else {
			cancelDeleteCandidate();
			selectCandidate(event);
		}
	}
	function _showDictionary(event: MouseEvent) {
		event.preventDefault();
		numOfMoves.current++;
		showDictionary();
	}
	function _hideDictionary() {
		cancelDeleteCandidate();
		hideDictionary();
	}
	const showJyutping = prefs.showRomanization === ShowRomanization.Always || prefs.showRomanization === ShowRomanization.ReverseOnly && info.isReverseLookup;
	const labels = info.matchedEntries?.flatMap(entry => entry.formattedLabels(prefs) ?? []) ?? [];
	const firstEntry = info.matchedEntries?.[0] ?? info.entries[0];
	const sourceBadge = info.isAi && <span className="ai-source" data-ai-source={info.source}>AI</span>;
	const inlineDefinitions = info.inlineDefinitions(prefs);
	const text = uiText[prefs.uiLanguage].dictionary;
	const detailTags = [
		(!info.isReverseLookup || prefs.showReverseCode) ? info.note : "",
		...labels.slice(0, 2),
	].filter(Boolean);
	const detailText = detailTags.join(" ");

	return <tbody
		className={`candidate-row${isHighlighted ? " highlighted" : ""}${info.isAi ? " ai-candidate" : ""}`}
		data-candidate-text={info.text}
		data-source={info.source}
		onClick={_selectCandidate}
		onMouseEnter={_showDictionary}
		onMouseMove={_showDictionary}
		onMouseLeave={_hideDictionary}
		onMouseDown={beginDeleteCandidate}
		onMouseUp={cancelDeleteCandidate}
		onTouchStart={event => {
			beginDeleteCandidate(event);
			showDictionary();
		}}
		onTouchMove={showDictionary}
		onTouchEnd={() => {
			cancelDeleteCandidate();
			_hideDictionary();
		}}
		onTouchCancel={_hideDictionary}>
		<tr>
			<td className="candidate-index">{info.label}</td>
			<td className="candidate-main">
				{showJyutping && firstEntry?.jyutping && <div className="candidate-reading">{firstEntry.jyutping}</div>}
				<div
					className={`candidate-text${showJyutping ? " candidate-text-spaced" : ""}`}
					data-chinese-typeface={prefs.chineseTypeface}>
					{info.text}
				</div>
			</td>
			<td className="candidate-note">
				{inlineDefinitions
					? <div className="candidate-definitions">
						{inlineDefinitions.map(([lang, name, value]) =>
							<span key={lang} className="candidate-definition" lang={lang} title={name}>
								{value}
							</span>
						)}
					</div>
					: detailText}
				{inlineDefinitions && detailText && <div className="candidate-note-tags">{detailText}</div>}
			</td>
			<td className="candidate-info">
				{sourceBadge}
				{info.hasDictionaryEntry(prefs) && <span aria-label={text.detailsAria}>ⓘ</span>}
			</td>
		</tr>
	</tbody>;
}

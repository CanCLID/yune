import { useEffect, useRef } from "react";

import CandidateInfo from "./CandidateInfo";
import { ShowRomanization } from "./consts";
import { useLongPress } from "./hooks";
import { uiText } from "./uiText";

import type { InterfacePreferences } from "./types";
import type { MouseEvent } from "react";

export default function Candidate({ isHighlighted, info, selectCandidate, deleteCandidate, showDictionary, hideDictionary, prefs }: {
	isHighlighted: boolean;
	info: CandidateInfo;
	selectCandidate(): void;
	deleteCandidate(): void;
	showDictionary(): void;
	hideDictionary(): void;
	prefs: InterfacePreferences;
}) {
	const justDeletedCandidate = useRef(false);
	function _deleteCandidate() {
		deleteCandidate();
		justDeletedCandidate.current = true;
	}
	const {
		onMouseDown: startLongPress,
		onMouseUp: cancelLongPress,
		onTouchStart: startTouchLongPress,
		onTouchEnd: cancelTouchLongPress,
		cancel: cancelLongPressTimer,
	} = useLongPress(_deleteCandidate, 800);
	const numOfMoves = useRef(0);
	useEffect(() => {
		numOfMoves.current = 0;
	}, [info]);
	function _selectCandidate() {
		if (justDeletedCandidate.current) {
			justDeletedCandidate.current = false;
		}
		else {
			cancelLongPressTimer();
			selectCandidate();
		}
	}
	function _showDictionary(event: MouseEvent) {
		event.preventDefault();
		numOfMoves.current++;
		showDictionary();
	}
	function _hideDictionary() {
		cancelLongPressTimer();
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
		onMouseDown={startLongPress}
		onTouchStart={event => {
			startTouchLongPress(event);
			showDictionary();
		}}
		onTouchMove={showDictionary}
		onTouchEnd={() => {
			cancelTouchLongPress();
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

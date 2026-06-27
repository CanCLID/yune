import { useEffect, useMemo, useRef, useState } from "react";

import { uiText } from "./uiText";

import type { YuneWebUserdbRow, YuneWebUserdbSnapshot } from "./types";
import type { UiLanguage } from "./uiText";
import type { ChangeEvent, MouseEvent } from "react";

type UserdbSortKey = "text" | "code" | "commits" | "dee" | "tick";
type UserdbSortDirection = "asc" | "desc";

interface UserdbColumn {
	key: UserdbSortKey;
	label: string;
}

interface IndexedUserdbRow {
	row: YuneWebUserdbRow;
	index: number;
}

const USERDB_TEXT_COLLATOR = new Intl.Collator(undefined, {
	numeric: true,
	sensitivity: "base",
});

function formatMetric(value: number | null) {
	if (value === null) {
		return "-";
	}
	return Number.isInteger(value) ? String(value) : value.toFixed(3);
}

function formatBytes(bytes: number) {
	if (bytes < 1024) {
		return `${bytes} B`;
	}
	return `${(bytes / 1024).toFixed(1)} KB`;
}

function defaultSortDirectionFor(key: UserdbSortKey): UserdbSortDirection {
	return key === "text" || key === "code" ? "asc" : "desc";
}

function compareNullableMetric(left: number | null, right: number | null) {
	if (left === null && right === null) {
		return 0;
	}
	if (left === null) {
		return 1;
	}
	if (right === null) {
		return -1;
	}
	return left - right;
}

function compareUserdbRows(left: YuneWebUserdbRow, right: YuneWebUserdbRow, key: UserdbSortKey) {
	switch (key) {
		case "text":
			return USERDB_TEXT_COLLATOR.compare(left.text, right.text);
		case "code":
			return USERDB_TEXT_COLLATOR.compare(left.code, right.code);
		case "commits":
			return compareNullableMetric(left.commits, right.commits);
		case "dee":
			return compareNullableMetric(left.dee, right.dee);
		case "tick":
			return compareNullableMetric(left.tick, right.tick);
	}
}

function sortUserdbRows(rows: YuneWebUserdbRow[], key: UserdbSortKey, direction: UserdbSortDirection) {
	const directionMultiplier = direction === "asc" ? 1 : -1;
	return rows
		.map((row, index): IndexedUserdbRow => ({ row, index }))
		.sort((left, right) => {
			const compared = compareUserdbRows(left.row, right.row, key);
			return compared === 0
				? left.index - right.index
				: compared * directionMultiplier;
		});
}

export default function YuneUserdbViewer({
	snapshot,
	isLoading,
	error,
	onRefresh,
	onImport,
	uiLanguage,
}: {
	snapshot?: YuneWebUserdbSnapshot;
	isLoading: boolean;
	error?: string;
	onRefresh(): Promise<void> | void;
	onImport(rawText: string): Promise<void> | void;
	uiLanguage: UiLanguage;
}) {
	const text = uiText[uiLanguage].userdb;
	const importInput = useRef<HTMLInputElement>(null);
	const [isImporting, setIsImporting] = useState(false);
	const [downloadHref, setDownloadHref] = useState<string>();
	const [sort, setSort] = useState<{ key: UserdbSortKey; direction: UserdbSortDirection }>({
		key: "code",
		direction: "asc",
	});
	const isExportDisabled = isLoading || isImporting || !downloadHref;
	const columns: UserdbColumn[] = [
		{ key: "text", label: text.text },
		{ key: "code", label: text.code },
		{ key: "commits", label: text.commits },
		{ key: "dee", label: text.weight },
		{ key: "tick", label: text.tick },
	];
	const sortedRows = useMemo(
		() => sortUserdbRows(snapshot?.rows ?? [], sort.key, sort.direction),
		[snapshot?.rows, sort.direction, sort.key],
	);
	useEffect(() => {
		if (!snapshot) {
			setDownloadHref(undefined);
			return;
		}
		const blob = new Blob([snapshot.rawText], { type: "text/plain;charset=utf-8" });
		const url = URL.createObjectURL(blob);
		setDownloadHref(url);
		return () => URL.revokeObjectURL(url);
	}, [snapshot]);
	function updateSort(key: UserdbSortKey) {
		setSort(current => {
			if (current.key === key) {
				return {
					key,
					direction: current.direction === "asc" ? "desc" : "asc",
				};
			}
			return {
				key,
				direction: defaultSortDirectionFor(key),
			};
		});
	}
	function handleDownloadClick(event: MouseEvent<HTMLAnchorElement>) {
		event.stopPropagation();
		if (isExportDisabled) {
			event.preventDefault();
		}
	}
	function openImportPicker(event: MouseEvent<HTMLButtonElement>) {
		event.preventDefault();
		event.stopPropagation();
		importInput.current?.click();
	}
	async function importUserdbFile(event: ChangeEvent<HTMLInputElement>) {
		const file = event.currentTarget.files?.[0];
		event.currentTarget.value = "";
		if (!file) {
			return;
		}
		setIsImporting(true);
		try {
			await onImport(await file.text());
		} finally {
			setIsImporting(false);
		}
	}
	return <section className="yd-userdb-panel" data-yune-userdb-viewer>
		<div className="yd-panel-heading yd-panel-heading-split">
			<span>{text.title}</span>
			<button
				type="button"
				className="yd-small-button"
				data-yune-userdb-refresh
				disabled={isLoading}
				onClick={() => void onRefresh()}>
				{isLoading ? text.refreshing : `↻ ${text.refresh}`}
			</button>
		</div>

		{error && <p className="yd-error" data-yune-userdb-error>{error}</p>}

		{snapshot
			? <>
				<div className="yd-meta-strip">
					<span className="yd-square-chip" data-yune-userdb-schema>
						<b>{text.schema}</b> {snapshot.schemaId}
					</span>
					<span className="yd-square-chip" data-yune-userdb-dictionary>
						<b>{text.dictionary}</b> {snapshot.dictionaryId}
					</span>
					<span className="yd-square-chip" data-yune-userdb-path>
						<b>{text.path}</b> {snapshot.path}
					</span>
					<span className="yd-square-chip" data-yune-userdb-bytes>
						<b>{text.bytes}</b> {formatBytes(snapshot.bytes)}
					</span>
					<span className="yd-square-chip" data-yune-userdb-row-count>
						<b>{text.rows}</b> {snapshot.rows.length}
					</span>
				</div>

				{snapshot.exists
					? <>
						<div className="yd-table-wrap" data-yune-userdb-table>
							<table className="yd-userdb-table">
								<thead>
									<tr>
										{columns.map(column => {
											const isActiveSort = sort.key === column.key;
											const nextDirection = isActiveSort && sort.direction === "asc"
												? "desc"
												: defaultSortDirectionFor(column.key);
											return <th
												key={column.key}
												aria-sort={isActiveSort ? sort.direction === "asc" ? "ascending" : "descending" : "none"}>
												<button
													type="button"
													className="yd-table-sort-button"
													data-yune-userdb-sort-key={column.key}
													data-yune-userdb-sort-direction={isActiveSort ? sort.direction : undefined}
													aria-label={`${text.sortBy} ${column.label} ${nextDirection === "asc" ? text.sortAscending : text.sortDescending}`}
													aria-pressed={isActiveSort}
													onClick={() => updateSort(column.key)}>
													<span>{column.label}</span>
													{isActiveSort && <span className="yd-sort-indicator" aria-hidden="true">
														{sort.direction === "asc" ? "▲" : "▼"}
													</span>}
												</button>
											</th>;
										})}
									</tr>
								</thead>
								<tbody>
									{sortedRows.length
										? sortedRows.map(({ row, index }) =>
											<tr key={`${row.raw}-${index}`} data-yune-userdb-row>
												<td>{row.text}</td>
												<td>{row.code}</td>
												<td>{formatMetric(row.commits)}</td>
												<td>{formatMetric(row.dee)}</td>
												<td>{formatMetric(row.tick)}</td>
											</tr>)
										: <tr>
											<td className="yd-muted-cell" colSpan={columns.length}>{text.noRows}</td>
										</tr>}
								</tbody>
							</table>
						</div>
						{snapshot.parseErrors.length > 0 && <div className="yd-warning" data-yune-userdb-parse-errors>
							<div>{text.parseNotes}</div>
							<ul>
								{snapshot.parseErrors.map(error =>
									<li key={`${error.line}-${error.reason}`}>{text.line} {error.line}: {error.reason}</li>
								)}
							</ul>
						</div>}
						<details className="yd-raw-block" data-yune-userdb-raw>
							<summary>
								<span className="yd-raw-summary-content">
									<span>{text.rawFile}</span>
									<span className="yd-raw-actions">
										<a
											className="yd-small-button"
											href={downloadHref ?? "#"}
											download={`${snapshot.dictionaryId}.userdb`}
											data-yune-userdb-export
											aria-label={text.downloadRawAria}
											aria-disabled={isExportDisabled}
											tabIndex={isExportDisabled ? -1 : undefined}
											onClick={handleDownloadClick}>
											{text.downloadRaw}
										</a>
										<button
											type="button"
											className="yd-small-button"
											data-yune-userdb-import
											aria-label={text.importRawAria}
											disabled={isLoading || isImporting}
											onClick={openImportPicker}>
											{isImporting ? text.importingRaw : text.importRaw}
										</button>
									</span>
								</span>
							</summary>
							<input
								ref={importInput}
								type="file"
								accept=".userdb,.txt,text/plain"
								hidden
								data-yune-userdb-import-input
								onChange={event => void importUserdbFile(event)} />
							<pre>{snapshot.rawText}</pre>
						</details>
					</>
					: <p className="yd-empty-panel" data-yune-userdb-empty>
						{text.noActiveFile}
					</p>}
			</>
			: <p className="yd-empty-panel" data-yune-userdb-loading>
				{isLoading ? text.loadingSnapshot : text.noSnapshot}
			</p>}
	</section>;
}

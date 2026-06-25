import type { YuneWebUserdbSnapshot } from "./types";

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

export default function YuneUserdbViewer({
	snapshot,
	isLoading,
	error,
	onRefresh,
}: {
	snapshot?: YuneWebUserdbSnapshot;
	isLoading: boolean;
	error?: string;
	onRefresh(): Promise<void> | void;
}) {
	return <section className="yd-userdb-panel" data-yune-userdb-viewer>
		<div className="yd-panel-heading yd-panel-heading-split">
			<span>用戶詞庫 <span>User Dictionary</span></span>
			<button
				type="button"
				className="yd-small-button"
				data-yune-userdb-refresh
				disabled={isLoading}
				onClick={() => void onRefresh()}>
				{isLoading ? "Refreshing" : "↻ 刷新"}
			</button>
		</div>

		{error && <p className="yd-error" data-yune-userdb-error>{error}</p>}

		{snapshot
			? <>
				<div className="yd-meta-strip">
					<span className="yd-square-chip" data-yune-userdb-schema>
						<b>schema</b> {snapshot.schemaId}
					</span>
					<span className="yd-square-chip" data-yune-userdb-dictionary>
						<b>dictionary</b> {snapshot.dictionaryId}
					</span>
					<span className="yd-square-chip" data-yune-userdb-path>
						<b>path</b> {snapshot.path}
					</span>
					<span className="yd-square-chip" data-yune-userdb-bytes>
						<b>bytes</b> {formatBytes(snapshot.bytes)}
					</span>
					<span className="yd-square-chip" data-yune-userdb-row-count>
						<b>rows</b> {snapshot.rows.length}
					</span>
				</div>

				{snapshot.exists
					? <>
						<div className="yd-table-wrap" data-yune-userdb-table>
							<table className="yd-userdb-table">
								<thead>
									<tr>
										<th>文字 TEXT</th>
										<th>編碼 CODE</th>
										<th>c</th>
										<th>d</th>
										<th>t</th>
									</tr>
								</thead>
								<tbody>
									{snapshot.rows.length
										? snapshot.rows.map((row, index) =>
											<tr key={`${row.raw}-${index}`} data-yune-userdb-row>
												<td>{row.text}</td>
												<td>{row.code}</td>
												<td>{formatMetric(row.commits)}</td>
												<td>{formatMetric(row.dee)}</td>
												<td>{formatMetric(row.tick)}</td>
											</tr>)
										: <tr>
											<td className="yd-muted-cell" colSpan={5}>No learned rows in this file.</td>
										</tr>}
								</tbody>
							</table>
						</div>
						{snapshot.parseErrors.length > 0 && <div className="yd-warning" data-yune-userdb-parse-errors>
							<div>Parse notes</div>
							<ul>
								{snapshot.parseErrors.map(error =>
									<li key={`${error.line}-${error.reason}`}>line {error.line}: {error.reason}</li>
								)}
							</ul>
						</div>}
						<details className="yd-raw-block" data-yune-userdb-raw>
							<summary>▸ Raw file</summary>
							<pre>{snapshot.rawText}</pre>
						</details>
					</>
					: <p className="yd-empty-panel" data-yune-userdb-empty>
						No active user dictionary file yet.
					</p>}
			</>
			: <p className="yd-empty-panel" data-yune-userdb-loading>
				{isLoading ? "Loading user dictionary snapshot..." : "No user dictionary snapshot loaded."}
			</p>}
	</section>;
}

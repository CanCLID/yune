from __future__ import annotations

import csv
import hashlib
import subprocess
import sys
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
TOKEN = "WITHDRAWN_ENDPOINT_MISMATCH"
CORRECTION_DATE = "2026-07-21"
PACKET = (
    ROOT
    / "apps/yune-web/e2e/results/yune-web-vs-my-rime-baseline/current-dashboard"
)
PACKET_HASHES = {
    "report.md": "ec67722114a8b73b315c6eea0a0f0e3a62bfaf38c6e3ae082a23acd6447dd8f1",
    "samples.csv": "4afa39f78980c15a61df3ec0fc508ff22e6c39b0fbbc600572477181f63dbbef",
    "samples.json": "d1327c2d42dece917bc640468f954c06894609381ee314994e503c1825d95c24",
    "summary.csv": "525758c709c954f4104249f1ad1228de5fdfecdc12c601ffafc716315a20851f",
    "summary.json": "d26d8d2275e2e7e3e700cbbbb001f4453dbc469326a3dfb7c185d4702d9cc383",
}
RATIO_BUNDLE = (
    ROOT / "docs/reports/evidence/current-ratio-visuals-2026-07-14"
)
DASHBOARD_BUNDLES = [
    ROOT / "docs/reports/evidence/current-performance-dashboard-2026-06-28",
    ROOT / "docs/reports/evidence/current-performance-dashboard-2026-06-29",
]
PERFORMANCE_REPORT = ROOT / "docs/reports/yune-vs-librime-performance.md"
HISTORY_REPORT = (
    ROOT
    / "docs/reports/history/2026-06-28-yune-web-vs-my-rime-browser-baseline-pre-consolidation.md"
)


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


class Web06EndpointWithdrawalTests(unittest.TestCase):
    def test_historical_comparator_packet_is_byte_exact(self) -> None:
        files = {path.name for path in PACKET.iterdir() if path.is_file()}
        self.assertEqual(files, set(PACKET_HASHES))
        for name, expected in PACKET_HASHES.items():
            actual = hashlib.sha256((PACKET / name).read_bytes()).hexdigest()
            self.assertEqual(actual, expected, name)

    def test_ratio_csv_preserves_values_and_withdraws_only_interactions(self) -> None:
        records = {
            row["metric"]: row
            for row in read_csv(RATIO_BUNDLE / "browser-peer-ratios.csv")
        }
        expected = {
            "Ready to input": ("1000", "634", "1.577"),
            "Input to candidate": ("74", "95", "0.779"),
            "Commit": ("107", "119", "0.899"),
            "WASM ready": ("64.0", "16.0", "4.000"),
            "WASM peak": ("64.0", "16.0", "4.000"),
            "Unique encoded resources": ("29.5", "8.5", "3.471"),
        }
        self.assertEqual(set(records), set(expected))
        for metric, values in expected.items():
            row = records[metric]
            self.assertEqual(
                (
                    row["yune_value"],
                    row["my_rime_value"],
                    row["yune_my_rime_ratio"],
                ),
                values,
                metric,
            )

        for metric in ("Input to candidate", "Commit"):
            row = records[metric]
            self.assertEqual(row["parity_read"], TOKEN)
            self.assertEqual(row["claim_status"], TOKEN)
            self.assertEqual(row["correction_date"], CORRECTION_DATE)
            self.assertTrue(row["correction_reason"])

        for metric in (
            "Ready to input",
            "WASM ready",
            "WASM peak",
            "Unique encoded resources",
        ):
            self.assertEqual(records[metric]["claim_status"], "RETAINED_DATED_SNAPSHOT")
            self.assertNotEqual(records[metric]["parity_read"], TOKEN)

    def test_dashboard_csvs_add_matching_interaction_only_overlays(self) -> None:
        csv_paths = [
            bundle / "current-browser-peer-comparator.csv"
            for bundle in DASHBOARD_BUNDLES
        ]
        self.assertEqual(csv_paths[0].read_bytes(), csv_paths[1].read_bytes())

        records = {
            (row["scenario"], row["schema"]): row
            for row in read_csv(csv_paths[0])
        }
        yune = records[("yune-public-demo", "luna_pinyin")]
        peer = records[("my-rime-live", "luna_pinyin")]
        self.assertEqual(
            (
                yune["median_ready_to_input_ms"],
                yune["median_input_to_candidate_ms"],
                yune["median_commit_ms"],
                yune["wasm_ready_mib"],
                yune["wasm_peak_mib"],
                yune["unique_encoded_resources_mib"],
            ),
            ("1000", "74", "107", "64.0", "64.0", "29.5"),
        )
        self.assertEqual(
            (
                peer["median_ready_to_input_ms"],
                peer["median_input_to_candidate_ms"],
                peer["median_commit_ms"],
                peer["wasm_ready_mib"],
                peer["wasm_peak_mib"],
                peer["unique_encoded_resources_mib"],
            ),
            ("634", "95", "119", "16.0", "16.0", "8.5"),
        )
        for row in (yune, peer):
            self.assertEqual(
                row["comparison_validity"],
                "same_schema_startup_wasm_payload_only",
            )
            self.assertEqual(row["interaction_claim_status"], TOKEN)
            self.assertEqual(row["interaction_correction_date"], CORRECTION_DATE)
            self.assertTrue(row["interaction_correction_reason"])

        jyutping = [row for (_, schema), row in records.items() if schema == "jyutping"]
        self.assertEqual(len(jyutping), 2)
        for row in jyutping:
            self.assertEqual(
                row["interaction_claim_status"],
                "NOT_APPLICABLE_DICTIONARY_CONFOUNDED",
            )
            self.assertEqual(row["comparison_validity"], "guard_only_dictionary_confounded")

    def test_public_surfaces_carry_the_dated_correction_without_old_reads(self) -> None:
        surfaces = [
            PERFORMANCE_REPORT,
            HISTORY_REPORT,
            RATIO_BUNDLE / "README.md",
            *(bundle / "README.md" for bundle in DASHBOARD_BUNDLES),
        ]
        for path in surfaces:
            source = path.read_text(encoding="utf-8")
            self.assertIn(TOKEN, source, path)
            self.assertIn(CORRECTION_DATE, source, path)

        performance = PERFORMANCE_REPORT.read_text(encoding="utf-8")
        self.assertNotIn(
            "First-input and commit latency are below peer parity",
            performance,
        )
        self.assertNotIn("latest fair browser peer snapshot", performance)
        self.assertNotIn("Latest fair browser peer normalized rows", performance)
        self.assertIn(
            "| ready to input | `1,000 ms` | `634 ms` | `1.577x` | peer lower |",
            performance,
        )
        self.assertIn(
            "| WASM ready | `64 MiB` | `16 MiB` | `4.000x` | peer lower |",
            performance,
        )
        self.assertIn(
            "| unique encoded resources | `29.5 MiB` | `8.5 MiB` | `3.471x` | peer lower |",
            performance,
        )

        history = HISTORY_REPORT.read_text(encoding="utf-8")
        self.assertNotIn("Fair first-input latency", history)
        self.assertNotIn("Fair commit latency", history)
        self.assertNotIn("below parity; Yune uses less time", history)
        self.assertIn(
            "| `jyut6ping3_mobile` | `taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng` | `74 ms` |",
            history,
        )

    def test_svg_overlays_are_well_formed_and_keep_historical_values(self) -> None:
        ratio_svg = RATIO_BUNDLE / "visuals/browser-luna-peer-parity.svg"
        ET.parse(ratio_svg)
        ratio_source = ratio_svg.read_text(encoding="utf-8")
        self.assertEqual(ratio_source.count(TOKEN), 2)
        for value in ("0.779x", "0.899x", "1.577x", "4.000x", "3.471x"):
            self.assertIn(value, ratio_source)
        self.assertIn(CORRECTION_DATE, ratio_source)
        self.assertIn("other rows unchanged", ratio_source)

        dashboard_svgs = [
            bundle / "visuals/current-browser-peer-latency.svg"
            for bundle in DASHBOARD_BUNDLES
        ]
        self.assertEqual(dashboard_svgs[0].read_bytes(), dashboard_svgs[1].read_bytes())
        ET.parse(dashboard_svgs[0])
        dashboard_source = dashboard_svgs[0].read_text(encoding="utf-8")
        self.assertEqual(dashboard_source.count(TOKEN), 2)
        self.assertIn("74/95 ms historical", dashboard_source)
        self.assertIn("107/119 ms historical", dashboard_source)
        self.assertIn(CORRECTION_DATE, dashboard_source)
        self.assertIn("ready time is unchanged", dashboard_source)

    def test_ratio_visual_build_is_reproducible(self) -> None:
        build = RATIO_BUNDLE / "build_visuals.py"
        generated_roots = [
            RATIO_BUNDLE / "visuals",
            ROOT
            / "docs/reports/evidence/history/performance-ratio-visuals-2026-07-14/visuals",
        ]
        before = {
            path: path.read_bytes()
            for generated_root in generated_roots
            for path in generated_root.glob("*.svg")
        }
        result = subprocess.run(
            [sys.executable, str(build)],
            cwd=RATIO_BUNDLE,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        after = {
            path: path.read_bytes()
            for generated_root in generated_roots
            for path in generated_root.glob("*.svg")
        }
        self.assertEqual(set(after), set(before))
        for path, contents in before.items():
            self.assertEqual(after[path], contents, path)


if __name__ == "__main__":
    unittest.main()

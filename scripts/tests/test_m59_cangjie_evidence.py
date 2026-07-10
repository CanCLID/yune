import copy
import csv
import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
FIXTURE = (
    ROOT
    / "crates"
    / "yune-core"
    / "tests"
    / "fixtures"
    / "upstream-1.17.0"
    / "cangjie5-composition.json"
)
PACKET = (
    ROOT
    / "docs"
    / "reports"
    / "evidence"
    / "m59-cangjie5-order-parity"
    / "increment-1-executable-evidence"
)
PACKET_INPUTS = [
    "hwmvsqtt",
    "ebcnyripm",
    "takohaeosk",
    "hwmvs",
    "qtt",
    "ebcn",
    "yripm",
    "tak",
    "oha",
    "eosk",
    "hdaetcu",
    "lyk",
]


def load_module(name: str, path: Path):
    specification = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(specification)
    assert specification.loader is not None
    specification.loader.exec_module(module)
    return module


curator = load_module(
    "m59_cangjie_curator", SCRIPTS / "curate-upstream-cangjie5.py"
)


def valid_raw_document():
    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
    cases = copy.deepcopy(fixture["cases"])
    repository_records = {
        repository: {
            "commit": identity["commit"],
            "tree": identity["tree"],
            "clean": True,
            "status_short": [],
        }
        for repository, identity in curator.SOURCE_REPOSITORIES.items()
    }
    expected_commits = {
        repository: identity["commit"]
        for repository, identity in curator.SOURCE_REPOSITORIES.items()
    }
    expected_trees = {
        repository: identity["tree"]
        for repository, identity in curator.SOURCE_REPOSITORIES.items()
    }
    page_sizes = sorted({case["page_size"] for case in cases})
    runtime_options = dict(curator.RUNTIME_OPTIONS)
    parameters = {
        "oracle_root": "target/upstream-oracle/1.17.0",
        "cangjie_dir": "target/upstream-oracle/1.17.0/schema-src/rime-cangjie",
        "essay": "target/upstream-oracle/1.17.0/schema-src/rime-essay/essay.txt",
        "raw_output": "target/m59-cangjie/raw.json",
        "output": "target/m59-cangjie/curated.json",
        "work_root": "target/m59-cangjie/work",
        "work_root_source": "explicit",
        "keep_work_root": True,
        "inputs": list(curator.EXPECTED_INPUTS),
        "schema_id": "cangjie5",
        "modules": ["default"],
        "expected_rime_dll_sha256": curator.RIME_DLL_SHA256,
        "expected_rime_deployer_sha256": curator.RIME_DEPLOYER_SHA256,
        "expected_repository_commits": expected_commits,
        "expected_repository_trees": expected_trees,
        "runtime_options": runtime_options,
        "runtime_options_source": curator.RUNTIME_OPTIONS_SOURCE,
        "additional_runtime_option_patches": [],
        "page_policy": curator.PAGE_POLICY,
        "serialization": curator.SERIALIZATION,
        "path_serialization_policy": (
            "repo-relative forward-slash paths; external paths replaced with external/<role>"
        ),
    }
    output_provenance = {
        "raw": {
            "path": parameters["raw_output"],
            "existed_before_capture": False,
            "write_policy": curator.WRITE_POLICY,
            "generated_by": "scripts/capture-upstream-cangjie5.ps1",
        },
        "curated": {
            "path": parameters["output"],
            "existed_before_capture": False,
            "write_policy": curator.WRITE_POLICY,
            "generated_by": "scripts/curate-upstream-cangjie5.py",
        },
    }
    return {
        "milestone": "M59",
        "task": "D-48 item 2: cangjie5 order-parity onboarding",
        "status": "raw_cangjie5_capture_complete",
        "canonical": True,
        "capture": {
            "engine": "rime/librime",
            "version": "1.17.0",
            "librime_commit": curator.LIBRIME_COMMIT,
            "source_commit": "a" * 40,
            "source_tree": "b" * 40,
            "source_clean": True,
            "source_status_short": [],
            "schema_id": "cangjie5",
            "modules": ["default"],
            "inputs": list(curator.EXPECTED_INPUTS),
            "input_count": len(curator.EXPECTED_INPUTS),
            "inputs_source": "fixed_m59_cangjie_order_lane",
            "page_sizes_observed": page_sizes,
            "captured_all_pages": True,
            "page_policy": curator.PAGE_POLICY,
            "runtime_options": runtime_options,
            "runtime_options_source": curator.RUNTIME_OPTIONS_SOURCE,
            "additional_runtime_option_patches": [],
            "rime_dll_sha256": curator.RIME_DLL_SHA256,
            "rime_deployer_sha256": curator.RIME_DEPLOYER_SHA256,
            "source_repositories": repository_records,
            "tool_hashes": {
                "capture_script_sha256": "c" * 64,
                "probe_sha256": "d" * 64,
                "curator_sha256": "e" * 64,
            },
            "source_shared_tree_sha256": "f" * 64,
            "staged_shared_tree_sha256": "1" * 64,
            "deployed_build_tree_sha256": "2" * 64,
            "commands": curator._expected_commands(parameters),
            "effective_parameters": parameters,
            "output_provenance": output_provenance,
        },
        "oracle": {
            "engine": "rime/librime",
            "version": "1.17.0",
            "commit": curator.LIBRIME_COMMIT,
            "dll": "target/upstream-oracle/1.17.0/extract/dist/lib/rime.dll",
            "dll_sha256": curator.RIME_DLL_SHA256,
            "deployer": "target/upstream-oracle/1.17.0/extract/dist/bin/rime_deployer.exe",
            "deployer_sha256": curator.RIME_DEPLOYER_SHA256,
        },
        "schema": {
            "yune_facing_schema_id": "cangjie5",
            "source_repo": "rime/rime-cangjie",
            "source_commit": curator.SOURCE_REPOSITORIES["rime/rime-cangjie"][
                "commit"
            ],
            "source_tree": curator.SOURCE_REPOSITORIES["rime/rime-cangjie"][
                "tree"
            ],
            "dependency_commits": expected_commits,
            "dependency_trees": expected_trees,
            "note": "upstream lane",
        },
        "options": {
            "runtime_options": runtime_options,
            "runtime_options_source": curator.RUNTIME_OPTIONS_SOURCE,
            "additional_runtime_option_patches": [],
            "custom_yaml": "default.custom.yaml only selects cangjie5",
            "page_sizes_observed": page_sizes,
        },
        "owner_target_specs": copy.deepcopy(list(curator.OWNER_TARGET_SPECS)),
        "control_target_specs": copy.deepcopy(list(curator.CONTROL_TARGET_SPECS)),
        "inputs": list(curator.EXPECTED_INPUTS),
        "cases": cases,
    }


class CangjieCuratorContractTests(unittest.TestCase):
    def test_valid_raw_derives_owner_targets_from_candidate_zero(self):
        raw = valid_raw_document()
        validated = curator._validate_raw_document(raw)
        self.assertEqual(
            [row["input"] for row in validated["composition_rows"]],
            [specification["input"] for specification in curator.OWNER_TARGET_SPECS],
        )
        for row, specification in zip(
            validated["composition_rows"], curator.OWNER_TARGET_SPECS
        ):
            case = validated["cases"][row["input"]]
            self.assertEqual(row["target"], case["all_candidates"][0]["text"])
            self.assertEqual(row["target"], curator._decode_codepoints(specification["target_codepoints"]))
            self.assertEqual(
                row["target_codepoints"], specification["target_codepoints"]
            )
            self.assertIn("derived from cases[", row["provenance"])
            self.assertNotIn("pending capture", row["provenance"])

    def test_mojibaked_or_invented_owner_target_is_rejected(self):
        raw = valid_raw_document()
        case = raw["cases"][0]
        replacement = "".join(chr(value) for value in (0xE7, 0xB2, 0xB5))
        case["all_candidates"][0]["text"] = replacement
        case["pages"][0]["candidates"][0]["text"] = replacement
        case["selected_candidates"][0]["text"] = replacement
        case["commit_text_preview"] = replacement
        with self.assertRaisesRegex(ValueError, "owner U\\+ declaration"):
            curator._validate_raw_document(raw)

    def test_atomic_target_linkage_and_complete_paging_are_required(self):
        raw = valid_raw_document()
        atomic_case = next(case for case in raw["cases"] if case["input"] == "qtt")
        replacement = chr(0x624B)
        atomic_case["all_candidates"][0]["text"] = replacement
        atomic_case["pages"][0]["candidates"][0]["text"] = replacement
        atomic_case["selected_candidates"][0]["text"] = replacement
        with self.assertRaisesRegex(ValueError, "do not reconstruct"):
            curator._validate_raw_document(raw)

        raw = valid_raw_document()
        raw["cases"][0]["captured_all_pages"] = False
        with self.assertRaisesRegex(ValueError, "not complete"):
            curator._validate_raw_document(raw)

    def test_identifiers_options_and_order_fail_closed(self):
        mutations = (
            lambda document: document["capture"]["source_repositories"][
                "rime/rime-cangjie"
            ].update(tree="0" * 40),
            lambda document: document["capture"]["runtime_options"].update(
                ascii_mode=True
            ),
            lambda document: document["inputs"].reverse(),
            lambda document: document["capture"]["tool_hashes"].pop(
                "probe_sha256"
            ),
            lambda document: document["capture"]["output_provenance"][
                "curated"
            ].update(existed_before_capture=True),
            lambda document: document["capture"]["commands"].update(
                capture="powershell tampered.ps1"
            ),
            lambda document: document["capture"].update(inputs_source="tampered"),
            lambda document: document["capture"]["effective_parameters"].update(
                path_serialization_policy="tampered"
            ),
            lambda document: document["options"].update(custom_yaml="tampered"),
            lambda document: document["oracle"].update(dll="tampered"),
        )
        for index, mutation in enumerate(mutations):
            with self.subTest(index=index):
                raw = valid_raw_document()
                mutation(raw)
                with self.assertRaises(ValueError):
                    curator._validate_raw_document(raw)

        raw = valid_raw_document()
        later_page_case = next(
            case for case in raw["cases"] if len(case["pages"]) > 1
        )
        later_page_case["pages"][1]["page_size"] = 999
        with self.assertRaisesRegex(ValueError, "page-size settings changed"):
            curator._validate_raw_document(raw)

    def test_source_slice_requires_complete_atomic_cohorts(self):
        validated = curator._validate_raw_document(valid_raw_document())
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            cangjie = root / "rime-cangjie"
            cangjie.mkdir()
            atomic_characters = {
                code: validated["cases"][code]["all_candidates"][0]["text"]
                for code in curator.ATOMIC_CODES
            }
            rows = [
                f"{character}\t{code}"
                for code, character in atomic_characters.items()
            ]
            (cangjie / curator.DICT_FILES[0]).write_text(
                "---\nname: base\n...\n" + "\n".join(rows) + "\n",
                encoding="utf-8",
                newline="\n",
            )
            for file_name in curator.DICT_FILES[1:]:
                (cangjie / file_name).write_text(
                    "---\nname: empty\n...\n", encoding="utf-8", newline="\n"
                )
            vocabulary = {
                character
                for specification in (
                    *curator.OWNER_TARGET_SPECS,
                    *curator.CONTROL_TARGET_SPECS,
                )
                for character in curator._decode_codepoints(
                    specification["target_codepoints"]
                )
            }
            vocabulary.update(atomic_characters.values())
            essay = root / "essay.txt"
            essay.write_text(
                "".join(
                    f"{character}\t{index + 1}\n"
                    for index, character in enumerate(sorted(vocabulary, key=ord))
                ),
                encoding="utf-8",
                newline="\n",
            )
            source_slice = curator._build_source_slice(
                cangjie, essay, validated["cases"]
            )
            self.assertEqual(source_slice["atomic_codes"], list(curator.ATOMIC_CODES))
            self.assertEqual(
                sum(len(values) for values in source_slice["import_rows"].values()),
                len(curator.ATOMIC_CODES),
            )

            missing_code = curator.ATOMIC_CODES[-1]
            filtered = [row for row in rows if not row.endswith(f"\t{missing_code}")]
            (cangjie / curator.DICT_FILES[0]).write_text(
                "---\nname: base\n...\n" + "\n".join(filtered) + "\n",
                encoding="utf-8",
                newline="\n",
            )
            with self.assertRaisesRegex(ValueError, "cohorts are absent"):
                curator._build_source_slice(cangjie, essay, validated["cases"])

    def test_canonical_loader_and_atomic_create_new_are_biting(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            output = root / "out.json"
            payload = {"target": curator._decode_codepoints("U+7CB5 U+62FC")}
            content = curator._canonical_json_bytes(payload)
            curator._write_atomic_create_new(output, content)
            self.assertEqual(output.read_bytes(), content)
            self.assertEqual(curator._load_canonical_json(output, "output"), payload)
            with self.assertRaises(FileExistsError):
                curator._write_atomic_create_new(output, content)
            bad = root / "bad.json"
            bad.write_bytes(b"\xef\xbb\xbf{}\r\n")
            with self.assertRaises(ValueError):
                curator._load_canonical_json(bad, "bad")

    def test_cli_paths_are_bound_to_evidence_path_roles(self):
        internal = ROOT / "target" / "m59-cangjie" / "curated.json"
        self.assertEqual(
            curator._evidence_path(internal, "output"),
            "target/m59-cangjie/curated.json",
        )
        with tempfile.TemporaryDirectory() as temp:
            external = Path(temp) / "curated.json"
            self.assertEqual(curator._evidence_path(external, "output"), "external/output")
        source = (SCRIPTS / "curate-upstream-cangjie5.py").read_text(
            encoding="utf-8"
        )
        self.assertIn("CLI {parameter_name} does not match raw effective parameters", source)

    def test_raw_input_is_rehashed_immediately_before_publication(self):
        raw = valid_raw_document()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            raw_path = root / "raw.json"
            output = root / "curated.json"
            oracle_root = root / "oracle"
            cangjie_dir = root / "cangjie"
            essay = root / "essay.txt"
            for path in (oracle_root, cangjie_dir):
                path.mkdir()
            essay.write_text("placeholder\n", encoding="utf-8")
            parameters = raw["capture"]["effective_parameters"]
            parameters.update(
                {
                    "oracle_root": "external/oracle-root",
                    "cangjie_dir": "external/cangjie-dir",
                    "essay": "external/essay",
                    "raw_output": "external/raw-output",
                    "output": "external/output",
                }
            )
            raw["capture"]["commands"] = curator._expected_commands(parameters)
            raw["capture"]["output_provenance"]["raw"]["path"] = parameters[
                "raw_output"
            ]
            raw["capture"]["output_provenance"]["curated"]["path"] = parameters[
                "output"
            ]
            raw["oracle"]["dll"] = "external/rime-dll"
            raw["oracle"]["deployer"] = "external/rime-deployer"
            raw_path.write_bytes(curator._canonical_json_bytes(raw))
            validation_calls = 0

            def validate_then_mutate(*_args):
                nonlocal validation_calls
                validation_calls += 1
                if validation_calls == 2:
                    raw_path.write_bytes(b"{}\n")
                return {}

            source_slice = {
                "policy": "test-policy",
                "import_rows": {},
                "vocabulary_rows": [],
            }
            with mock.patch.object(
                curator, "_validate_live_inputs", side_effect=validate_then_mutate
            ), mock.patch.object(
                curator, "_build_source_slice", return_value=source_slice
            ):
                with self.assertRaisesRegex(ValueError, "changed before curated publication"):
                    curator._curate(
                        [
                            "--raw-input",
                            str(raw_path),
                            "--output",
                            str(output),
                            "--oracle-root",
                            str(oracle_root),
                            "--cangjie-dir",
                            str(cangjie_dir),
                            "--essay",
                            str(essay),
                        ]
                    )
            self.assertFalse(output.exists())


class CangjiePowerShellContractTests(unittest.TestCase):
    def test_upstream_capture_script_is_strict_ascii_and_non_overwriting(self):
        script = SCRIPTS / "capture-upstream-cangjie5.ps1"
        raw = script.read_bytes()
        self.assertTrue(all(byte < 0x80 for byte in raw))
        source = raw.decode("ascii")
        for specification in (
            "U+7CB5 U+62FC",
            "U+6E2C U+8A66",
            "U+83AB U+4F2F U+6D22",
        ):
            self.assertIn(specification, source)
        self.assertNotIn("oracle_backed pending capture", source)
        self.assertIn("[System.IO.FileMode]::CreateNew", source)
        self.assertIn("Write-NewUtf8NoBom $RawOutput", source)
        self.assertIn("--raw-input $RawOutput", source)
        self.assertIn("--output $Output", source)
        self.assertIn("Assert-NoReparsePoints", source)
        self.assertIn("Assert-CangjiePublicationPathsUnchanged", source)
        self.assertIn("publication path changed after canonical preflight", source)
        self.assertIn("invalid marker", source)
        self.assertIn("canonical path changed", source)
        preflight = source.index("$PathPreflight = Assert-CangjieCapturePathPreflight")
        first_mutation = source.index(
            "New-Item -ItemType Directory -Path $WorkRoot, $Shared, $User, $Build, $RawDirectory"
        )
        self.assertLess(preflight, first_mutation)
        final_write = source.index("Write-NewUtf8NoBom $RawOutput $EvidenceJson")
        for guard in (
            'Assert-GitStateUnchanged $RepoRoot "yune immediately before raw output"',
            'Assert-FileSha256Unchanged $RimeDll "rime.dll immediately before raw output"',
            'Assert-FileSha256Unchanged $ProbeSource "oracle-rime-probe.cs immediately before raw output"',
            'Assert-FileSha256Unchanged $Curator "curator immediately before raw output"',
            "Tree-Sha256 $Shared",
        ):
            self.assertLess(source.index(guard), final_write)

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_cangjie_path_preflight_rejects_aliases_containment_ads_and_junctions(self):
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            oracle = root / "oracle"
            extract = oracle / "extract"
            schema = oracle / "schema-src"
            repository = schema / "rime-cangjie"
            repository.mkdir(parents=True)
            extract.mkdir(parents=True)
            protected = root / "capture.ps1"
            protected.write_bytes(b"protected")
            existing = root / "existing.json"
            existing.write_bytes(b"existing")
            junction = root / "oracle-junction"
            junction_result = subprocess.run(
                ["cmd", "/c", "mklink", "/J", str(junction), str(oracle)],
                check=False,
                capture_output=True,
                text=True,
                timeout=30,
            )
            has_junction = junction_result.returncode == 0 and junction.exists()
            common_paths = {
                "oracle": str(oracle),
                "extract": str(extract),
                "schema": str(schema),
                "repository": str(repository),
            }
            cases = [
                {
                    **common_paths,
                    "name": "safe",
                    "raw": str(root / "results" / "raw.json"),
                    "output": str(root / "results" / "curated.json"),
                    "work": str(oracle / "work-safe"),
                },
                {
                    **common_paths,
                    "name": "existing",
                    "raw": str(existing),
                    "output": str(root / "results" / "existing-curated.json"),
                    "work": str(oracle / "work-existing"),
                },
                {
                    **common_paths,
                    "name": "nested_outputs",
                    "raw": str(root / "nested"),
                    "output": str(root / "nested" / "curated.json"),
                    "work": str(oracle / "work-nested"),
                },
                {
                    **common_paths,
                    "name": "inside_oracle",
                    "raw": str(oracle / "raw.json"),
                    "output": str(root / "results" / "oracle-curated.json"),
                    "work": str(oracle / "work-oracle-output"),
                },
                {
                    **common_paths,
                    "name": "work_inside_source",
                    "raw": str(root / "results" / "source-raw.json"),
                    "output": str(root / "results" / "source-curated.json"),
                    "work": str(repository / "work"),
                },
                {
                    **common_paths,
                    "name": "ads",
                    "raw": str(protected) + "::$DATA",
                    "output": str(root / "results" / "ads-curated.json"),
                    "work": str(oracle / "work-ads"),
                },
                {
                    **common_paths,
                    "name": "protected_alias",
                    "raw": str(root / "results" / "alias-raw.json"),
                    "output": str(protected),
                    "work": str(oracle / "work-alias"),
                },
            ]
            if has_junction:
                cases.append(
                    {
                        **common_paths,
                        "name": "junction_into_oracle",
                        "raw": str(junction / "raw.json"),
                        "output": str(root / "results" / "junction-curated.json"),
                        "work": str(oracle / "work-junction"),
                    }
                )
            bad_oracle = root / "bad-oracle"
            bad_schema = bad_oracle / "schema-src"
            bad_repository = bad_schema / "rime-cangjie"
            bad_repository.mkdir(parents=True)
            outside_extract = root / "outside-extract"
            outside_extract.mkdir()
            escaped_extract = bad_oracle / "extract"
            escaped_extract_result = subprocess.run(
                ["cmd", "/c", "mklink", "/J", str(escaped_extract), str(outside_extract)],
                check=False,
                capture_output=True,
                text=True,
                timeout=30,
            )
            has_escaped_extract = (
                escaped_extract_result.returncode == 0 and escaped_extract.exists()
            )
            if has_escaped_extract:
                cases.append(
                    {
                        "name": "extract_junction_escape",
                        "raw": str(root / "results" / "escape-raw.json"),
                        "output": str(root / "results" / "escape-curated.json"),
                        "work": str(bad_oracle / "work"),
                        "oracle": str(bad_oracle),
                        "extract": str(escaped_extract),
                        "schema": str(bad_schema),
                        "repository": str(bad_repository),
                    }
                )
            cases_path = root / "cases.json"
            cases_path.write_text(json.dumps(cases), encoding="utf-8")
            command = r'''
$ErrorActionPreference = "Stop"
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $env:YUNE_CANGJIE_CAPTURE_SCRIPT_TEST,
    [ref]$tokens,
    [ref]$errors
)
if ($errors.Count -ne 0) { throw "capture script parse failed" }
foreach ($name in @(
    "Get-CanonicalCangjiePath",
    "Test-CangjiePathWithinOrEqual",
    "Assert-CangjieCapturePathPreflight"
)) {
    $functionAst = $ast.Find({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $node.Name -eq $name
    }, $true)
    if ($null -eq $functionAst) { throw "missing helper $name" }
    Invoke-Expression $functionAst.Extent.Text
}
$document = Get-Content -LiteralPath $env:YUNE_CANGJIE_CASES_TEST -Raw -Encoding UTF8 | ConvertFrom-Json
$protected = @{ CaptureScript = $env:YUNE_CANGJIE_PROTECTED_TEST }
$results = @(
    foreach ($case in $document) {
        $sources = [string[]]@([string]$case.repository)
        try {
            $null = Assert-CangjieCapturePathPreflight `
                ([string]$case.raw) `
                ([string]$case.output) `
                ([string]$case.work) `
                ([string]$case.oracle) `
                ([string]$case.extract) `
                ([string]$case.schema) `
                $sources `
                $protected
            [pscustomobject]@{ name = $case.name; accepted = $true; error = $null }
        }
        catch {
            [pscustomobject]@{ name = $case.name; accepted = $false; error = $_.Exception.Message }
        }
    }
)
$results | ConvertTo-Json -Depth 5 -Compress
'''
            environment = os.environ.copy()
            environment.update(
                {
                    "YUNE_CANGJIE_CAPTURE_SCRIPT_TEST": str(
                        SCRIPTS / "capture-upstream-cangjie5.ps1"
                    ),
                    "YUNE_CANGJIE_CASES_TEST": str(cases_path),
                    "YUNE_CANGJIE_PROTECTED_TEST": str(protected),
                }
            )
            try:
                completed = subprocess.run(
                    [powershell, "-NoProfile", "-Command", command],
                    check=False,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    env=environment,
                    timeout=60,
                )
                if completed.returncode != 0:
                    self.fail(completed.stderr or completed.stdout)
                results = {
                    result["name"]: result
                    for result in json.loads(completed.stdout.strip().splitlines()[-1])
                }
                self.assertTrue(results["safe"]["accepted"])
                for name in results:
                    if name != "safe":
                        self.assertFalse(results[name]["accepted"], name)
                self.assertEqual(protected.read_bytes(), b"protected")
                self.assertEqual(existing.read_bytes(), b"existing")
                for case in cases:
                    if case["name"] not in {"existing", "protected_alias", "ads"}:
                        self.assertFalse(Path(case["raw"]).exists())
            finally:
                if has_junction and junction.exists():
                    junction.rmdir()
                if has_escaped_extract and escaped_extract.exists():
                    escaped_extract.rmdir()

    def test_generic_yune_capture_rejects_oracle_schema_mismatch_and_revalidates(self):
        source = (SCRIPTS / "capture-yune-candidate-order.ps1").read_text(
            encoding="utf-8-sig"
        )
        self.assertIn("function Assert-OracleSchemaMatch", source)
        schema_guard = source.index("Assert-OracleSchemaMatch $Oracle $SchemaId")
        mutation = source.index(
            "New-Item -ItemType Directory -Force -Path $WorkRoot, $User, $Build, $Bin"
        )
        self.assertLess(schema_guard, mutation)
        final_write = source.index("Write-Utf8NoBom $Output $EvidenceJson -CreateNew")
        for guard in (
            "Assert-GitStateUnchanged $RepoRoot $RepoState",
            'Assert-FileSha256Unchanged $YuneDll "Yune DLL" $YuneDllSha256',
            'Assert-FileSha256Unchanged $ProbeSource "Rime probe" $ProbeSha256',
            'Assert-FileSha256Unchanged $OracleCapture "oracle capture" $OracleCaptureSha256',
            "Tree-Sha256 $SharedDataDir",
        ):
            self.assertLess(source.index(guard), final_write)
        self.assertIn("source_tree = $RepoState.tree", source)
        self.assertIn("source_clean = [bool]$RepoState.clean", source)
        self.assertIn("$StagedSharedTreeSha256 = Tree-Sha256 $Shared", source)
        self.assertIn("Tree-Sha256 $Shared) -ne $StagedSharedTreeSha256", source)
        self.assertIn("function Assert-CapturedCaseContract", source)
        self.assertIn("Assert-CapturedCaseContract $Cases $Inputs $SchemaId", source)
        self.assertIn("canonical path changed", source)
        self.assertIn(
            "Remove-Item -LiteralPath $PathPreflight.work_root -Recurse", source
        )

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_generic_yune_oracle_schema_guard_runtime(self):
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        command = r'''
$ErrorActionPreference = "Stop"
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $env:YUNE_CAPTURE_SCRIPT_TEST,
    [ref]$tokens,
    [ref]$errors
)
foreach ($name in @("Assert-OracleSchemaMatch", "Assert-CapturedCaseContract")) {
    $functionAst = $ast.Find({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $node.Name -eq $name
    }, $true)
    Invoke-Expression $functionAst.Extent.Text
}
function Accepted([string]$Json, [string]$Expected) {
    try {
        Assert-OracleSchemaMatch ($Json | ConvertFrom-Json) $Expected
        return $true
    }
    catch {
        return $false
    }
}
function CasesAccepted([object[]]$Cases) {
    try {
        Assert-CapturedCaseContract $Cases @("first", "second") "cangjie5"
        return $true
    }
    catch {
        return $false
    }
}
$matching = '{"capture":{"schema_id":"cangjie5"},"schema":{"yune_facing_schema_id":"cangjie5"},"cases":[{"schema_id":"cangjie5","input":"a"}]}'
$metadataMismatch = '{"capture":{"schema_id":"luna_pinyin"},"cases":[{"schema_id":"cangjie5","input":"a"}]}'
$caseMismatch = '{"capture":{"schema_id":"cangjie5"},"cases":[{"schema_id":"luna_pinyin","input":"a"}]}'
$matchingCases = @(
    @{ input = "first"; schema_id = "cangjie5"; captured_all_pages = $true },
    @{ input = "second"; schema_id = "cangjie5"; captured_all_pages = $true }
)
$reorderedCases = @($matchingCases[1], $matchingCases[0])
$wrongSchemaCases = @(
    $matchingCases[0],
    @{ input = "second"; schema_id = "luna_pinyin"; captured_all_pages = $true }
)
$incompleteCases = @(
    $matchingCases[0],
    @{ input = "second"; schema_id = "cangjie5"; captured_all_pages = $false }
)
[ordered]@{
    matching = Accepted $matching "cangjie5"
    metadata_mismatch = Accepted $metadataMismatch "cangjie5"
    case_mismatch = Accepted $caseMismatch "cangjie5"
    matching_cases = CasesAccepted $matchingCases
    reordered_cases = CasesAccepted $reorderedCases
    wrong_schema_cases = CasesAccepted $wrongSchemaCases
    incomplete_cases = CasesAccepted $incompleteCases
} | ConvertTo-Json -Compress
'''
        environment = os.environ.copy()
        environment["YUNE_CAPTURE_SCRIPT_TEST"] = str(
            SCRIPTS / "capture-yune-candidate-order.ps1"
        )
        completed = subprocess.run(
            [powershell, "-NoProfile", "-Command", command],
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            env=environment,
            timeout=30,
        )
        if completed.returncode != 0:
            self.fail(completed.stderr or completed.stdout)
        result = json.loads(completed.stdout.strip().splitlines()[-1])
        self.assertTrue(result["matching"])
        self.assertFalse(result["metadata_mismatch"])
        self.assertFalse(result["case_mismatch"])
        self.assertTrue(result["matching_cases"])
        self.assertFalse(result["reordered_cases"])
        self.assertFalse(result["wrong_schema_cases"])
        self.assertFalse(result["incomplete_cases"])


class CangjieExecutableEvidencePacketTests(unittest.TestCase):
    def test_packet_manifest_binds_all_nonself_files_and_fixture_import(self):
        manifest_path = PACKET / "cangjie-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        self.assertEqual(manifest["packet_file_count"], 7)
        self.assertFalse(manifest["acceptance_evidence"])
        self.assertFalse(manifest["closes_d48"])
        self.assertFalse(manifest["binary_payloads_copied"])
        self.assertEqual(manifest["inputs"], PACKET_INPUTS)

        packet_files = sorted(path.name for path in PACKET.iterdir() if path.is_file())
        self.assertEqual(
            packet_files,
            sorted(
                [
                    "README.md",
                    "cangjie-diff.csv",
                    "cangjie-diff.json",
                    "cangjie-manifest.json",
                    "cangjie-oracle-raw.json",
                    "cangjie-oracle.json",
                    "cangjie-yune.json",
                ]
            ),
        )
        inventory = manifest["file_inventory"]
        self.assertEqual(len(inventory), 6)
        self.assertEqual(
            {row["path"] for row in inventory},
            set(packet_files) - {"cangjie-manifest.json"},
        )
        for row in inventory:
            path = PACKET / row["path"]
            raw = path.read_bytes()
            self.assertEqual(len(raw), row["bytes"], row["path"])
            self.assertEqual(hashlib.sha256(raw).hexdigest(), row["sha256"])

        for path in PACKET.iterdir():
            if not path.is_file():
                continue
            raw = path.read_bytes()
            self.assertFalse(raw.startswith(b"\xef\xbb\xbf"), path.name)
            self.assertNotIn(b"\r", raw, path.name)
            self.assertNotIn(b"\x00", raw, path.name)
            self.assertTrue(raw.endswith(b"\n"), path.name)
            self.assertFalse(raw.endswith(b"\n\n"), path.name)
            raw.decode("utf-8")

            relative = path.relative_to(ROOT).as_posix()
            filtered = subprocess.run(
                ["git", "hash-object", f"--path={relative}", relative],
                cwd=ROOT,
                check=True,
                capture_output=True,
                text=True,
                encoding="utf-8",
            ).stdout.strip()
            unfiltered = subprocess.run(
                ["git", "hash-object", "--no-filters", relative],
                cwd=ROOT,
                check=True,
                capture_output=True,
                text=True,
                encoding="utf-8",
            ).stdout.strip()
            self.assertEqual(filtered, unfiltered, relative)

        curated = (PACKET / "cangjie-oracle.json").read_bytes()
        self.assertEqual(curated, FIXTURE.read_bytes())
        self.assertEqual(
            hashlib.sha256(curated).hexdigest(),
            manifest["tracked_fixture_import"]["sha256"],
        )

    def test_exact_diff_preserves_cj1_red_without_exceptions(self):
        manifest = json.loads(
            (PACKET / "cangjie-manifest.json").read_text(encoding="utf-8")
        )
        diff = json.loads((PACKET / "cangjie-diff.json").read_text(encoding="utf-8"))
        oracle = (PACKET / "cangjie-oracle.json").read_bytes()
        yune = (PACKET / "cangjie-yune.json").read_bytes()
        self.assertEqual(diff["policy"], "exact")
        self.assertFalse(diff["all_accepted"])
        self.assertIsNone(diff["provenance"]["exceptions"])
        self.assertEqual(diff["inputs"], PACKET_INPUTS)
        self.assertEqual(
            diff["provenance"]["oracle"]["sha256"],
            hashlib.sha256(oracle).hexdigest(),
        )
        self.assertEqual(
            diff["provenance"]["actual"]["sha256"],
            hashlib.sha256(yune).hexdigest(),
        )
        verdicts = {case["input"]: case["verdict"] for case in diff["cases"]}
        self.assertEqual(sum(value == "pass" for value in verdicts.values()), 4)
        self.assertEqual(sum(value == "fail" for value in verdicts.values()), 8)
        self.assertEqual(
            [name for name in PACKET_INPUTS if verdicts[name] == "pass"],
            manifest["comparison"]["accepted_inputs"],
        )
        self.assertEqual(
            [name for name in PACKET_INPUTS if verdicts[name] == "fail"],
            manifest["comparison"]["failed_inputs"],
        )
        for name in ("hwmvsqtt", "ebcnyripm", "takohaeosk", "hdaetcu", "lyk"):
            self.assertEqual(verdicts[name], "fail", name)
        for case in diff["cases"]:
            self.assertTrue(case["oracle_captured_all_pages"], case["input"])
            self.assertTrue(case["actual_captured_all_pages"], case["input"])
            self.assertEqual(case["accepted_exceptions"], [], case["input"])

        with (PACKET / "cangjie-diff.csv").open(
            encoding="utf-8", newline=""
        ) as handle:
            csv_rows = list(csv.DictReader(handle))
        self.assertEqual([row["input"] for row in csv_rows], PACKET_INPUTS)
        self.assertEqual(
            [row["verdict"] for row in csv_rows],
            [verdicts[name] for name in PACKET_INPUTS],
        )
        self.assertTrue(all(row["accepted_exceptions"] == "" for row in csv_rows))
        self.assertEqual(manifest["comparison"]["expected_exit_code"], 1)
        self.assertTrue(manifest["comparison"]["no_exceptions_applied"])
        self.assertEqual(
            manifest["cj1_disposition"]["ignored_test_retained"],
            "cangjie5_upstream_lane_segmentation_scoring_is_blocked",
        )

    def test_oracle_manifest_binds_fresh_capture_and_target_only_import(self):
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        oracle_manifest = json.loads(
            (
                FIXTURE.parent / "oracle-manifest.json"
            ).read_text(encoding="utf-8")
        )
        row = next(
            entry
            for entry in oracle_manifest["files"]
            if entry["path"] == "cangjie5-composition.json"
        )
        self.assertEqual(row["sha256"], hashlib.sha256(FIXTURE.read_bytes()).hexdigest())
        self.assertEqual(row["capture_source_commit"], fixture["capture"]["source_commit"])
        self.assertEqual(row["capture_command"], fixture["capture"]["commands"]["capture"])
        self.assertEqual(
            row["source_row_policy"], fixture["source_slice"]["policy"]
        )
        self.assertIn("-RawOutput 'target/", row["capture_command"])
        self.assertIn("-Output 'target/", row["capture_command"])
        self.assertNotIn(
            "-Output 'crates/yune-core/tests/fixtures/", row["capture_command"]
        )
        self.assertIn("never overwrites", row["import_policy"])
        self.assertEqual(fixture["curation"]["version"], 2)
        self.assertEqual(
            fixture["curation"]["raw_input_sha256"],
            "91dca789769cbbed160132bf23a54d891bf164f3f2617d5fcf5a2ac5d4443be1",
        )


if __name__ == "__main__":
    unittest.main()

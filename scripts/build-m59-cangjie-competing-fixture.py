#!/usr/bin/env python3
"""Build the pinned M59 Cangjie competing-segmentation source fixture.

The historical ``cangjie5-composition.json`` oracle and its Increment 1 packet
are immutable inputs.  This tool extracts a separate, source-only slice from
the exact pinned rime-cangjie and rime-essay Git trees.  It deliberately scans
the complete imported dictionaries so every requested exact-code cohort is
complete, and it records both present and absent essay weights needed to audit
the target-vs-one-letter-root sentence competition.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Iterable


PINNED_ORACLE_SHA256 = (
    "24408c3b2b83db516ae1382d2ba743b41ead50c7c026aee2837a01137c7ecbcf"
)
PINNED_LIBRIME_COMMIT = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
SCHEMA_REPOSITORY = "rime/rime-cangjie"
ESSAY_REPOSITORY = "rime/rime-essay"
CONTROL_INPUT = "hdaetcu"
CONTROL_CODES = ("hda", "etcu")
IMPORTED_DICTIONARIES = (
    ("cangjie5.base.dict.yaml", ("text", "code")),
    ("cangjie5.stem.dict.yaml", ("text", "code", "stem")),
    ("cangjie5.extended.dict.yaml", ("text", "code")),
)
EXPECTED_IMPORT_TABLES = ("cangjie5.base", "cangjie5.stem", "cangjie5.extended")
GENERATOR_VERSION = 1
WEIGHT_PATTERN = re.compile(r"^-?[0-9]+$")


class FixtureBuildError(RuntimeError):
    """Raised when a pinned input or extraction invariant is not satisfied."""


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _canonical_json_bytes(document: dict[str, Any]) -> bytes:
    text = json.dumps(document, ensure_ascii=False, indent=2) + "\n"
    return text.encode("utf-8")


def write_canonical_json_create_new(path: Path, document: dict[str, Any]) -> None:
    path = path.resolve()
    if path.exists():
        raise FixtureBuildError(f"output already exists: {path}")
    if not path.parent.is_dir():
        raise FixtureBuildError(f"output parent does not exist: {path.parent}")
    with path.open("xb") as stream:
        stream.write(_canonical_json_bytes(document))


def _run_git(repo: Path, *arguments: str, binary: bool = False) -> str | bytes:
    command = ["git", "-C", str(repo), *arguments]
    result = subprocess.run(
        command,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=not binary,
        encoding=None if binary else "utf-8",
    )
    if result.returncode != 0:
        stderr = (
            result.stderr.decode("utf-8", errors="replace")
            if binary
            else result.stderr
        )
        raise FixtureBuildError(
            f"git command failed ({' '.join(command)}): {stderr.strip()}"
        )
    return result.stdout


def _git_repository_state(path: Path, label: str) -> dict[str, Any]:
    root_text = _run_git(path, "rev-parse", "--show-toplevel")
    assert isinstance(root_text, str)
    root = Path(root_text.strip()).resolve()
    commit_text = _run_git(root, "rev-parse", "HEAD")
    tree_text = _run_git(root, "rev-parse", "HEAD^{tree}")
    status_text = _run_git(root, "status", "--porcelain=v1", "--untracked-files=all")
    assert isinstance(commit_text, str)
    assert isinstance(tree_text, str)
    assert isinstance(status_text, str)
    status = [line for line in status_text.splitlines() if line]
    if status:
        raise FixtureBuildError(f"{label} repository is dirty: {status!r}")
    return {
        "root": root,
        "commit": commit_text.strip().lower(),
        "tree": tree_text.strip().lower(),
        "clean": True,
        "status_short": [],
    }


def _git_tracked_bytes(repo: Path, path: Path, label: str) -> tuple[str, bytes]:
    root = repo.resolve()
    resolved = path.resolve()
    try:
        relative = resolved.relative_to(root).as_posix()
    except ValueError as error:
        raise FixtureBuildError(f"{label} is outside its pinned Git tree: {resolved}") from error
    if not resolved.is_file():
        raise FixtureBuildError(f"missing {label}: {resolved}")
    blob = _run_git(root, "show", f"HEAD:{relative}", binary=True)
    assert isinstance(blob, bytes)
    # Hash and parse the pinned blob, not checkout-filtered bytes.  The canonical
    # upstream trees are often checked out with core.autocrlf=true on Windows;
    # Git status still proves cleanliness, while HEAD:<path> keeps the generated
    # fixture byte-identical across LF and CRLF worktrees.
    return relative, blob


def _require(value: bool, message: str) -> None:
    if not value:
        raise FixtureBuildError(message)


def _ordered_unique(values: Iterable[str]) -> list[str]:
    seen: set[str] = set()
    ordered: list[str] = []
    for value in values:
        if value not in seen:
            seen.add(value)
            ordered.append(value)
    return ordered


def _oracle_case(document: dict[str, Any], input_text: str) -> dict[str, Any]:
    cases = [case for case in document.get("cases", []) if case.get("input") == input_text]
    _require(len(cases) == 1, f"oracle must contain exactly one case for {input_text}")
    return cases[0]


def _parse_owner_scope(oracle: dict[str, Any]) -> dict[str, Any]:
    rows = oracle.get("composition_rows")
    _require(isinstance(rows, list) and len(rows) == 3, "oracle must contain three owner rows")
    owner_rows: list[dict[str, Any]] = []
    atom_codes: list[str] = []
    for row in rows:
        input_text = row.get("input")
        target = row.get("target")
        char_code_specs = row.get("char_codes")
        _require(isinstance(input_text, str) and input_text, "owner input must be text")
        _require(isinstance(target, str) and target, f"owner target missing for {input_text}")
        _require(
            isinstance(char_code_specs, list) and char_code_specs,
            f"owner char codes missing for {input_text}",
        )
        target_characters: list[str] = []
        codes: list[str] = []
        for specification in char_code_specs:
            _require(
                isinstance(specification, str) and specification.count("=") == 1,
                f"invalid owner char-code specification for {input_text}",
            )
            character, code = specification.split("=", 1)
            _require(len(character) == 1 and code, f"invalid owner char-code row: {specification}")
            target_characters.append(character)
            codes.append(code)
        _require("".join(target_characters) == target, f"owner target/code mismatch for {input_text}")
        _require("".join(codes) == input_text, f"owner input/code mismatch for {input_text}")
        case = _oracle_case(oracle, input_text)
        candidates = case.get("all_candidates")
        _require(
            isinstance(candidates, list)
            and candidates
            and candidates[0].get("text") == target
            and case.get("commit_text_preview") == target,
            f"owner oracle candidate zero is not locked for {input_text}",
        )
        _require(case.get("captured_all_pages") is True, f"owner capture is incomplete: {input_text}")
        atom_codes.extend(codes)
        owner_rows.append(
            {
                "input": input_text,
                "target": target,
                "target_codepoints": row.get("target_codepoints"),
                "target_characters": target_characters,
                "target_atom_codes": codes,
                "oracle_candidate_index": 0,
                "oracle_preedit": case.get("preedit"),
                "oracle_commit_text_preview": case.get("commit_text_preview"),
            }
        )

    declared_atoms = oracle.get("source_slice", {}).get("atomic_codes")
    unique_atoms = _ordered_unique(atom_codes)
    _require(declared_atoms == unique_atoms, "owner atom codes do not match the pinned source slice")

    control_rows = oracle.get("control_rows")
    _require(isinstance(control_rows, list), "oracle control rows must be an array")
    controls = [row for row in control_rows if row.get("input") == CONTROL_INPUT]
    _require(len(controls) == 1, f"oracle must contain exactly one {CONTROL_INPUT} control")
    control = controls[0]
    control_target = control.get("target")
    _require(
        isinstance(control_target, str) and len(control_target) == len(CONTROL_CODES),
        "hdaetcu control target must have two characters",
    )
    _require("".join(CONTROL_CODES) == CONTROL_INPUT, "control code declaration is invalid")
    control_case = _oracle_case(oracle, CONTROL_INPUT)
    control_index = control.get("candidate_index")
    candidates = control_case.get("all_candidates")
    _require(
        control_index == 0
        and isinstance(candidates, list)
        and candidates
        and candidates[0].get("text") == control_target
        and control_case.get("commit_text_preview") == control_target,
        "hdaetcu oracle control is not locked at candidate zero",
    )
    _require(control_case.get("captured_all_pages") is True, "hdaetcu capture is incomplete")
    control_row = {
        "input": CONTROL_INPUT,
        "target": control_target,
        "target_codepoints": control.get("target_codepoints"),
        "target_characters": list(control_target),
        "target_atom_codes": list(CONTROL_CODES),
        "oracle_candidate_index": 0,
        "oracle_preedit": control_case.get("preedit"),
        "oracle_commit_text_preview": control_case.get("commit_text_preview"),
    }

    inputs = [row["input"] for row in owner_rows] + [CONTROL_INPUT]
    root_codes = _ordered_unique(character for input_text in inputs for character in input_text)
    _require(all(len(code) == 1 for code in root_codes), "root-code scope must be one letter")
    return {
        "owner_rows": owner_rows,
        "control_row": control_row,
        "owner_atom_codes": unique_atoms,
        "control_codes": list(CONTROL_CODES),
        "one_letter_root_codes": root_codes,
    }


def _parse_import_tables(source: str) -> list[str]:
    imports: list[str] = []
    in_imports = False
    for raw in source.splitlines():
        if raw == "import_tables:":
            _require(not in_imports and not imports, "duplicate import_tables section")
            in_imports = True
            continue
        if not in_imports:
            continue
        match = re.fullmatch(r"  - ([A-Za-z0-9_.-]+)", raw)
        if match:
            imports.append(match.group(1))
            continue
        if raw and not raw[0].isspace():
            break
    _require(imports, "cangjie5.dict.yaml has no import_tables")
    return imports


def _parse_columns(source: str, file_name: str) -> list[str]:
    columns: list[str] = []
    in_columns = False
    for raw in source.splitlines():
        if raw == "columns:":
            _require(not in_columns and not columns, f"duplicate columns section in {file_name}")
            in_columns = True
            continue
        if not in_columns:
            continue
        match = re.fullmatch(r"  - ([A-Za-z0-9_]+)", raw)
        if match:
            columns.append(match.group(1))
            continue
        if raw and not raw[0].isspace():
            break
    _require(columns, f"dictionary columns are missing in {file_name}")
    return columns


def _parse_dictionary_rows(
    source: str, file_name: str, expected_columns: tuple[str, ...]
) -> list[dict[str, Any]]:
    columns = _parse_columns(source, file_name)
    _require(columns == list(expected_columns), f"unexpected columns in {file_name}: {columns!r}")
    rows: list[dict[str, Any]] = []
    body_started = False
    for line_number, raw in enumerate(source.splitlines(), 1):
        if not body_started:
            if raw == "...":
                body_started = True
            continue
        if not raw or raw.startswith("#"):
            continue
        fields = raw.split("\t")
        _require(
            len(fields) == len(columns),
            f"unexpected field count in {file_name}:{line_number}",
        )
        row = dict(zip(columns, fields, strict=True))
        _require(row["text"] and row["code"], f"blank text/code in {file_name}:{line_number}")
        row.update({"source_file": file_name, "source_line": line_number, "raw": raw})
        rows.append(row)
    _require(body_started and rows, f"dictionary body is missing in {file_name}")
    return rows


def _parse_essay_rows(source: str) -> dict[str, list[dict[str, Any]]]:
    rows: dict[str, list[dict[str, Any]]] = {}
    comments_enabled = True
    for line_number, raw in enumerate(source.splitlines(), 1):
        if not raw:
            continue
        if comments_enabled and raw.startswith("#"):
            if raw == "# no comment":
                comments_enabled = False
            continue
        fields = raw.split("\t")
        _require(len(fields) >= 1 and fields[0], f"invalid essay row at line {line_number}")
        weight_raw = fields[1] if len(fields) > 1 else "0"
        _require(
            WEIGHT_PATTERN.fullmatch(weight_raw) is not None,
            f"non-integral essay weight at line {line_number}: {weight_raw!r}",
        )
        row = {
            "text": fields[0],
            "weight": int(weight_raw),
            "weight_raw": weight_raw,
            "source_line": line_number,
            "raw": raw,
        }
        rows.setdefault(fields[0], []).append(row)
    _require(rows, "essay.txt has no vocabulary rows")
    return rows


def _source_file_record(relative: str, data: bytes) -> dict[str, Any]:
    return {
        "path": relative,
        "sha256": sha256_bytes(data),
        "bytes": len(data),
        "line_count": len(data.decode("utf-8").splitlines()),
    }


def build_fixture(
    oracle_path: Path,
    cangjie_dir: Path,
    essay_path: Path,
    *,
    expected_oracle_sha256: str = PINNED_ORACLE_SHA256,
) -> dict[str, Any]:
    oracle_path = oracle_path.resolve()
    cangjie_dir = cangjie_dir.resolve()
    essay_path = essay_path.resolve()
    oracle_bytes = oracle_path.read_bytes()
    actual_oracle_sha = sha256_bytes(oracle_bytes)
    _require(
        actual_oracle_sha == expected_oracle_sha256,
        f"oracle SHA-256 mismatch: expected {expected_oracle_sha256}, got {actual_oracle_sha}",
    )
    _require(not oracle_bytes.startswith(b"\xef\xbb\xbf"), "oracle fixture must be UTF-8 without BOM")
    oracle = json.loads(oracle_bytes.decode("utf-8"))
    _require(oracle.get("canonical") is True, "oracle fixture is not canonical")
    _require(
        oracle.get("status") == "cangjie5_capture_curated_complete",
        "unexpected oracle fixture status",
    )
    _require(
        oracle.get("oracle", {}).get("commit") == PINNED_LIBRIME_COMMIT,
        "unexpected pinned librime commit",
    )
    scope = _parse_owner_scope(oracle)

    cangjie_state = _git_repository_state(cangjie_dir, SCHEMA_REPOSITORY)
    _require(cangjie_state["root"] == cangjie_dir, "cangjie-dir must be the Git repository root")
    expected_cangjie_commit = oracle.get("schema", {}).get("source_commit")
    expected_cangjie_tree = oracle.get("schema", {}).get("source_tree")
    _require(
        cangjie_state["commit"] == expected_cangjie_commit
        and cangjie_state["tree"] == expected_cangjie_tree,
        "rime-cangjie commit/tree does not match the pinned oracle",
    )

    essay_state = _git_repository_state(essay_path.parent, ESSAY_REPOSITORY)
    _require(essay_state["root"] == essay_path.parent, "essay must be at its Git repository root")
    expected_essay = oracle.get("capture", {}).get("source_repositories", {}).get(ESSAY_REPOSITORY)
    _require(isinstance(expected_essay, dict), "oracle is missing rime-essay provenance")
    _require(
        essay_state["commit"] == expected_essay.get("commit")
        and essay_state["tree"] == expected_essay.get("tree"),
        "rime-essay commit/tree does not match the pinned oracle",
    )

    main_relative, main_bytes = _git_tracked_bytes(
        cangjie_state["root"], cangjie_dir / "cangjie5.dict.yaml", "cangjie5 dictionary"
    )
    main_source = main_bytes.decode("utf-8")
    imports = _parse_import_tables(main_source)
    _require(imports == list(EXPECTED_IMPORT_TABLES), f"unexpected Cangjie imports: {imports!r}")
    for expected in (
        "sort: by_weight",
        "use_preset_vocabulary: true",
        "max_phrase_length: 7",
        "min_phrase_weight: 100",
        "formula: \"AaAzBaBbBz\"",
        "formula: \"AaAzBaBzCz\"",
        "formula: \"AaBzCaYzZz\"",
        "tail_anchor: \"'\"",
    ):
        _require(expected in main_source, f"pinned Cangjie metadata missing: {expected}")

    requested_codes = (
        scope["owner_atom_codes"]
        + scope["control_codes"]
        + scope["one_letter_root_codes"]
    )
    _require(
        len(requested_codes) == len(set(requested_codes)),
        "requested Cangjie cohort codes must not overlap",
    )
    cohorts: dict[str, list[dict[str, Any]]] = {code: [] for code in requested_codes}
    source_files = [_source_file_record(main_relative, main_bytes)]
    for file_name, columns in IMPORTED_DICTIONARIES:
        relative, data = _git_tracked_bytes(
            cangjie_state["root"], cangjie_dir / file_name, file_name
        )
        source_files.append(_source_file_record(relative, data))
        for row in _parse_dictionary_rows(data.decode("utf-8"), file_name, columns):
            code = row["code"]
            if code in cohorts:
                cohorts[code].append(row)
    for code, rows in cohorts.items():
        _require(rows, f"complete exact-code cohort is empty for {code}")

    essay_relative, essay_bytes = _git_tracked_bytes(
        essay_state["root"], essay_path, "rime-essay essay.txt"
    )
    essay_rows = _parse_essay_rows(essay_bytes.decode("utf-8"))
    for code in requested_codes:
        for row in cohorts[code]:
            rows_for_text = essay_rows.get(row["text"], [])
            row["essay_rows"] = rows_for_text
            # Yune's preset-weight map, like std::unordered_map assignment in
            # librime's collector, keeps the final duplicate vocabulary row.
            row["effective_compiler_weight"] = (
                rows_for_text[-1]["weight"] if rows_for_text else 0
            )
    cohort_texts = _ordered_unique(
        row["text"] for code in requested_codes for row in cohorts[code]
    )

    segmentations = scope["owner_rows"] + [scope["control_row"]]
    primary_scoring_texts: list[str] = []
    for segmentation in segmentations:
        root_path = list(segmentation["input"])
        target_path = segmentation["target_atom_codes"]
        target_characters = segmentation["target_characters"]
        for character, code in zip(target_characters, target_path, strict=True):
            weighted_top = max(
                enumerate(cohorts[code]),
                key=lambda indexed: (indexed[1]["effective_compiler_weight"], -indexed[0]),
            )[1]
            _require(
                weighted_top["text"] == character,
                f"target character {character!r} is not weighted candidate zero for {code}",
            )
        root_rows = [
            max(
                enumerate(cohorts[code]),
                key=lambda indexed: (indexed[1]["effective_compiler_weight"], -indexed[0]),
            )[1]
            for code in root_path
        ]
        root_characters = [row["text"] for row in root_rows]
        segmentation["root_code_path"] = root_path
        segmentation["root_candidate_zero_characters"] = root_characters
        segmentation["root_candidate_zero_sentence"] = "".join(root_characters)
        primary_scoring_texts.extend(target_characters)
        primary_scoring_texts.extend(root_characters)

    primary_scoring_texts = _ordered_unique(primary_scoring_texts)
    missing_primary = [text for text in primary_scoring_texts if text not in essay_rows]
    _require(not missing_primary, f"primary competing-path essay weights are missing: {missing_primary!r}")

    composition_texts = _ordered_unique(
        [segmentation["target"] for segmentation in segmentations]
        + [segmentation["root_candidate_zero_sentence"] for segmentation in segmentations]
    )
    selected_essay_texts = _ordered_unique(cohort_texts + composition_texts)
    selected_essay_rows = {
        text: essay_rows.get(text, []) for text in selected_essay_texts
    }

    script_path = Path(__file__).resolve()
    script_bytes = script_path.read_bytes()
    return {
        "milestone": "M59",
        "task": "Increment 4d Cangjie CJ-1 competing-segmentation source fixture",
        "status": "cangjie5_competing_segmentation_source_complete",
        "canonical": True,
        "generation": {
            "generated_by": "scripts/build-m59-cangjie-competing-fixture.py",
            "generator_version": GENERATOR_VERSION,
            "generator_sha256": sha256_bytes(script_bytes),
            "serialization": {
                "encoding": "utf-8",
                "bom": False,
                "line_endings": "lf",
                "terminal_newline": "exactly_one",
                "write_policy": "create_new",
            },
            "command_template": (
                "python -B scripts/build-m59-cangjie-competing-fixture.py "
                "--oracle crates/yune-core/tests/fixtures/upstream-1.17.0/"
                "cangjie5-composition.json --cangjie-dir <pinned-rime-cangjie> "
                "--essay <pinned-rime-essay>/essay.txt --output <create-new-output>"
            ),
        },
        "oracle_fixture": {
            "path": "crates/yune-core/tests/fixtures/upstream-1.17.0/cangjie5-composition.json",
            "sha256": actual_oracle_sha,
            "librime_commit": PINNED_LIBRIME_COMMIT,
            "status": oracle["status"],
            "immutable_input": True,
        },
        "schema_source": {
            "repository": SCHEMA_REPOSITORY,
            "commit": cangjie_state["commit"],
            "tree": cangjie_state["tree"],
            "clean": True,
            "status_short": [],
            "files": source_files,
            "import_tables": imports,
            "dictionary_policy": {
                "sort": "by_weight",
                "use_preset_vocabulary": True,
                "vocabulary": "essay",
                "max_phrase_length": 7,
                "min_phrase_weight": 100,
                "encoder_rules": [
                    {"length_equal": 2, "formula": "AaAzBaBbBz"},
                    {"length_equal": 3, "formula": "AaAzBaBzCz"},
                    {"length_in_range": [4, 10], "formula": "AaBzCaYzZz"},
                ],
                "tail_anchor": "'",
            },
        },
        "essay_source": {
            "repository": ESSAY_REPOSITORY,
            "commit": essay_state["commit"],
            "tree": essay_state["tree"],
            "clean": True,
            "status_short": [],
            "file": _source_file_record(essay_relative, essay_bytes),
            "selection_policy": (
                "all exact rows for every extracted cohort text plus each oracle target "
                "and derived root-candidate-zero sentence; missing rows remain explicit"
            ),
            "primary_scoring_texts": primary_scoring_texts,
            "selected_rows": selected_essay_rows,
            "missing_selected_texts": [
                text for text in selected_essay_texts if not selected_essay_rows[text]
            ],
        },
        "scope": {
            "owner_atom_codes": scope["owner_atom_codes"],
            "control_codes": scope["control_codes"],
            "one_letter_root_codes": scope["one_letter_root_codes"],
            "all_requested_codes": requested_codes,
            "cohort_policy": (
                "complete exact-code rows from cangjie5.base, cangjie5.stem, and "
                "cangjie5.extended in declared import/file order"
            ),
        },
        "competing_segmentations": segmentations,
        "exact_code_cohorts": cohorts,
    }


def _arguments(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--oracle", type=Path, required=True)
    parser.add_argument("--cangjie-dir", type=Path, required=True)
    parser.add_argument("--essay", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    arguments = _arguments(sys.argv[1:] if argv is None else argv)
    try:
        if arguments.output.exists():
            raise FixtureBuildError(f"output already exists: {arguments.output.resolve()}")
        document = build_fixture(arguments.oracle, arguments.cangjie_dir, arguments.essay)
        write_canonical_json_create_new(arguments.output, document)
    except (FixtureBuildError, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    output = arguments.output.resolve()
    print(
        f"wrote {output} ({output.stat().st_size} bytes, "
        f"sha256={sha256_bytes(output.read_bytes())})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

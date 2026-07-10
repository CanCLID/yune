#!/usr/bin/env python3
"""Create a provenance-complete M59 Cangjie oracle fixture.

The raw input is the untouched librime probe capture written by
``capture-upstream-cangjie5.ps1``. This curator validates the complete capture
contract, derives the three owner targets from candidate zero, verifies their
ASCII U+ declarations, and embeds only pinned upstream dictionary rows.

The raw input and curated output are always different files. The output is
published atomically with create-new semantics and is never written over an
existing fixture.
"""

from __future__ import annotations

import argparse
import collections
import copy
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Iterable


SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
GIT_ID_RE = re.compile(r"^[0-9a-f]{40}$")
CODEPOINT_SPEC_RE = re.compile(r"U\+[0-9A-F]{4,6}(?: U\+[0-9A-F]{4,6})*")

LIBRIME_COMMIT = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
RIME_DLL_SHA256 = "86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b"
RIME_DEPLOYER_SHA256 = (
    "3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071"
)

SOURCE_REPOSITORIES = collections.OrderedDict(
    (
        (
            "rime/rime-cangjie",
            {
                "directory": "rime-cangjie",
                "commit": "52d90a1b1312e74042b38c1cbc8142defbc53171",
                "tree": "db11cf6ffd382ada3087e9765c0ba2e636a8b68d",
            },
        ),
        (
            "rime/rime-prelude",
            {
                "directory": "rime-prelude",
                "commit": "082425ea0684bca36474415d4a0e8db9b016487e",
                "tree": "d7e128f09ce6b1f920729ef2f848ca1294c9cb31",
            },
        ),
        (
            "rime/rime-luna-pinyin",
            {
                "directory": "rime-luna-pinyin",
                "commit": "18a80335c37522311f7cff02886cd81cec3b460a",
                "tree": "0d5efcb75aa40689bf3be210a4f056db6d77b49d",
            },
        ),
        (
            "rime/rime-essay",
            {
                "directory": "rime-essay",
                "commit": "48c7538f0b760fcc8c9d6bf08711f82cfbd2e9ed",
                "tree": "4769c4ef6c5f93f450c5f36c2c9ac5e6845d37bc",
            },
        ),
        (
            "rime/rime-stroke",
            {
                "directory": "rime-stroke",
                "commit": "3a4b0f4013e2b4c14b1e80c92b1d4723eb65f39c",
                "tree": "d60c793d8d68154847923f21aa73ba90441dab32",
            },
        ),
        (
            "rime/rime-cantonese",
            {
                "directory": "rime-cantonese",
                "commit": "c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0",
                "tree": "eb193fb80675ffa60df3c32bf24afa7d7f68617a",
            },
        ),
        (
            "CanCLID/rime-loengfan",
            {
                "directory": "rime-loengfan",
                "commit": "987ac95b02f957e8764a2f45222a4006c188ed50",
                "tree": "0858d1087046b8d1c3d36c36000ece5630b09cb3",
            },
        ),
    )
)

EXPECTED_INPUTS = (
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
)

OWNER_TARGET_SPECS = (
    {
        "input": "hwmvsqtt",
        "target_codepoints": "U+7CB5 U+62FC",
        "atomic_codes": ["hwmvs", "qtt"],
        "declaration_source": "owner-signed D-47/D-48 Cangjie composition row",
    },
    {
        "input": "ebcnyripm",
        "target_codepoints": "U+6E2C U+8A66",
        "atomic_codes": ["ebcn", "yripm"],
        "declaration_source": "owner-signed D-47/D-48 Cangjie composition row",
    },
    {
        "input": "takohaeosk",
        "target_codepoints": "U+83AB U+4F2F U+6D22",
        "atomic_codes": ["tak", "oha", "eosk"],
        "declaration_source": "owner-signed D-47/D-48 Cangjie composition row",
    },
)

CONTROL_TARGET_SPECS = (
    {
        "input": "hdaetcu",
        "target_codepoints": "U+9999 U+6E2F",
        "candidate_index": 0,
    },
    {
        "input": "lyk",
        "target_codepoints": "U+4E2D U+6587",
        "candidate_index": 1,
    },
)

ATOMIC_CODES = tuple(
    code for specification in OWNER_TARGET_SPECS for code in specification["atomic_codes"]
)
DICT_FILES = (
    "cangjie5.base.dict.yaml",
    "cangjie5.stem.dict.yaml",
    "cangjie5.extended.dict.yaml",
)

RUNTIME_OPTIONS = collections.OrderedDict(
    (
        ("ascii_mode", False),
        ("full_shape", False),
        ("ascii_punct", False),
        ("zh_hans", False),
    )
)
RUNTIME_OPTIONS_SOURCE = "RimeProbe.CaptureWithIdentity/CaptureRuntimeOptionPolicy"
PAGE_POLICY = (
    "RimeProbe.Capture all pages; incomplete or non-advancing pagination is fatal"
)
SERIALIZATION = {
    "encoding": "utf-8",
    "bom": False,
    "line_endings": "lf",
    "terminal_newline": "exactly_one",
}
WRITE_POLICY = "canonical_utf8_no_bom_lf_one_terminal_lf_create_new"
CURATOR_VERSION = 2

COMMIT_PARAMETER_NAMES = {
    "rime/rime-cangjie": "ExpectedCangjieCommit",
    "rime/rime-prelude": "ExpectedPreludeCommit",
    "rime/rime-luna-pinyin": "ExpectedLunaPinyinCommit",
    "rime/rime-essay": "ExpectedEssayCommit",
    "rime/rime-stroke": "ExpectedStrokeCommit",
    "rime/rime-cantonese": "ExpectedCantoneseCommit",
    "CanCLID/rime-loengfan": "ExpectedLoengfanCommit",
}


def _file_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _evidence_path(path: Path, role: str) -> str:
    resolved = path.resolve(strict=path.exists())
    repo_root = Path(__file__).resolve().parent.parent
    try:
        return resolved.relative_to(repo_root).as_posix()
    except ValueError:
        return f"external/{role}"


def _require_sha256(value: Any, label: str) -> str:
    if not isinstance(value, str) or SHA256_RE.fullmatch(value) is None:
        raise ValueError(f"{label} must be a lowercase SHA-256")
    return value


def _require_git_id(value: Any, label: str) -> str:
    if not isinstance(value, str) or GIT_ID_RE.fullmatch(value) is None:
        raise ValueError(f"{label} must be a lowercase 40-character Git id")
    return value


def _decode_codepoints(specification: str) -> str:
    if not isinstance(specification, str) or CODEPOINT_SPEC_RE.fullmatch(specification) is None:
        raise ValueError(f"invalid ASCII codepoint specification: {specification!r}")
    characters = []
    for token in specification.split(" "):
        value = int(token[2:], 16)
        if value > 0x10FFFF or 0xD800 <= value <= 0xDFFF:
            raise ValueError(f"invalid Unicode scalar in specification: {token}")
        characters.append(chr(value))
    return "".join(characters)


def _codepoint_specification(text: str) -> str:
    return " ".join(f"U+{ord(character):04X}" for character in text)


def _canonical_json_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, indent=2, separators=(",", ": ")) + "\n"
    ).encode("utf-8")


def _quote_command_arg(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


def _expected_commands(parameters: dict[str, Any]) -> dict[str, str]:
    capture = [
        "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/capture-upstream-cangjie5.ps1",
        f"-OracleRoot {_quote_command_arg(parameters['oracle_root'])}",
        f"-RawOutput {_quote_command_arg(parameters['raw_output'])}",
        f"-Output {_quote_command_arg(parameters['output'])}",
    ]
    if parameters["work_root_source"] == "explicit":
        capture.append(f"-WorkRoot {_quote_command_arg(parameters['work_root'])}")
    elif parameters["work_root_source"] != "generated_unique_temp":
        raise ValueError("work_root_source is not canonical")
    if parameters["keep_work_root"] is True:
        capture.append("-KeepWorkRoot")
    elif parameters["keep_work_root"] is not False:
        raise ValueError("keep_work_root must be boolean")
    capture.extend(
        (
            "-ExpectedRimeDllSha256 "
            + _quote_command_arg(parameters["expected_rime_dll_sha256"]),
            "-ExpectedRimeDeployerSha256 "
            + _quote_command_arg(parameters["expected_rime_deployer_sha256"]),
        )
    )
    for repository, parameter_name in COMMIT_PARAMETER_NAMES.items():
        capture.append(
            f"-{parameter_name} "
            + _quote_command_arg(parameters["expected_repository_commits"][repository])
        )
    curate = " ".join(
        (
            "python -B scripts/curate-upstream-cangjie5.py",
            f"--raw-input {_quote_command_arg(parameters['raw_output'])}",
            f"--output {_quote_command_arg(parameters['output'])}",
            f"--oracle-root {_quote_command_arg(parameters['oracle_root'])}",
            f"--cangjie-dir {_quote_command_arg(parameters['cangjie_dir'])}",
            f"--essay {_quote_command_arg(parameters['essay'])}",
        )
    )
    return {
        "deploy": (
            "rime_deployer.exe --build disposable/user disposable/shared "
            "disposable/user/build"
        ),
        "capture": " ".join(capture),
        "curate": curate,
    }


def _read_canonical_json(path: Path, label: str) -> tuple[Any, str]:
    raw = path.read_bytes()
    if not raw:
        raise ValueError(f"{label} is empty")
    if raw.startswith(b"\xef\xbb\xbf"):
        raise ValueError(f"{label} must not contain a UTF-8 BOM")
    if b"\x00" in raw or b"\r" in raw:
        raise ValueError(f"{label} must use canonical UTF-8 LF bytes")
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        raise ValueError(f"{label} must have exactly one terminal LF")
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError(f"{label} must be valid UTF-8") from error
    try:
        document = json.loads(text)
    except json.JSONDecodeError as error:
        raise ValueError(f"{label} must contain valid JSON") from error
    return document, hashlib.sha256(raw).hexdigest()


def _load_canonical_json(path: Path, label: str) -> Any:
    document, _ = _read_canonical_json(path, label)
    return document


def _write_atomic_create_new(path: Path, content: bytes) -> None:
    if path.exists() or path.is_symlink():
        raise FileExistsError(f"refusing to overwrite existing output: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temp_name = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    temp_path = Path(temp_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.link(temp_path, path)
        temp_path.unlink()
    except Exception:
        temp_path.unlink(missing_ok=True)
        raise


def _git(path: Path, *arguments: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(path), *arguments],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip()
        raise ValueError(f"git {' '.join(arguments)} failed for {path}: {detail}")
    return completed.stdout.strip()


def _live_git_state(path: Path) -> dict[str, Any]:
    status = _git(path, "status", "--short").splitlines()
    return {
        "commit": _git(path, "rev-parse", "HEAD").lower(),
        "tree": _git(path, "rev-parse", "HEAD^{tree}").lower(),
        "clean": status == [],
        "status_short": status,
    }


def _validate_repository_record(record: Any, repository: str) -> dict[str, Any]:
    if not isinstance(record, dict) or set(record) != {
        "commit",
        "tree",
        "clean",
        "status_short",
    }:
        raise ValueError(f"source repository record is incomplete: {repository}")
    expected = SOURCE_REPOSITORIES[repository]
    commit = _require_git_id(record["commit"], f"{repository} commit")
    tree = _require_git_id(record["tree"], f"{repository} tree")
    if commit != expected["commit"] or tree != expected["tree"]:
        raise ValueError(f"source repository is not at the pinned identity: {repository}")
    if record["clean"] is not True or record["status_short"] != []:
        raise ValueError(f"source repository must be exactly clean: {repository}")
    return record


def _validate_candidate(candidate: Any, input_value: str, global_index: int) -> None:
    if not isinstance(candidate, dict):
        raise ValueError(f"candidate {global_index} for {input_value} must be an object")
    if not isinstance(candidate.get("text"), str) or not candidate["text"]:
        raise ValueError(f"candidate {global_index} for {input_value} has no text")
    if candidate.get("global_index") != global_index:
        raise ValueError(f"candidate global order is incomplete for {input_value}")


def _validate_case(case: Any, expected_input: str) -> tuple[int, list[dict[str, Any]]]:
    if not isinstance(case, dict):
        raise ValueError(f"capture case for {expected_input} must be an object")
    if case.get("input") != expected_input or case.get("schema_id") != "cangjie5":
        raise ValueError(f"capture case identity mismatch for {expected_input}")
    if case.get("captured_all_pages") is not True or case.get("pagination_error"):
        raise ValueError(f"capture case is not complete for {expected_input}")
    pages = case.get("pages")
    candidates = case.get("all_candidates")
    if not isinstance(pages, list) or not pages:
        raise ValueError(f"capture case has no pages for {expected_input}")
    if not isinstance(candidates, list) or not candidates:
        raise ValueError(f"capture case has no candidates for {expected_input}")
    case_page_size = case.get("page_size")
    if not isinstance(case_page_size, int) or case_page_size <= 0:
        raise ValueError(f"page size is invalid for {expected_input}")
    flattened: list[dict[str, Any]] = []
    for page_number, page in enumerate(pages):
        if not isinstance(page, dict) or page.get("page_no") != page_number:
            raise ValueError(f"page order is incomplete for {expected_input}")
        page_candidates = page.get("candidates")
        if not isinstance(page_candidates, list):
            raise ValueError(f"page candidates are malformed for {expected_input}")
        if page.get("page_size") != case_page_size:
            raise ValueError(f"page-size settings changed within {expected_input}")
        if page.get("is_last_page") is not (page_number == len(pages) - 1):
            raise ValueError(f"last-page state is inconsistent for {expected_input}")
        if len(page_candidates) > case_page_size or (
            page_number < len(pages) - 1 and len(page_candidates) != case_page_size
        ):
            raise ValueError(f"page candidate count is inconsistent for {expected_input}")
        for local_index, candidate in enumerate(page_candidates):
            if candidate.get("index") != local_index:
                raise ValueError(f"local candidate order is incomplete for {expected_input}")
            flattened.append(candidate)
    if flattened != candidates:
        raise ValueError(f"pages do not reconstruct all_candidates for {expected_input}")
    for global_index, candidate in enumerate(candidates):
        _validate_candidate(candidate, expected_input, global_index)
    if case.get("selected_candidates") != pages[0]["candidates"]:
        raise ValueError(f"selected_candidates does not match page zero for {expected_input}")
    return case_page_size, candidates


def _validate_target_linkage(
    cases: dict[str, dict[str, Any]],
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    composition_rows = []
    for specification in OWNER_TARGET_SPECS:
        owner_case = cases[specification["input"]]
        target = owner_case["all_candidates"][0]["text"]
        declared_target = _decode_codepoints(specification["target_codepoints"])
        if target != declared_target:
            raise ValueError(
                f"candidate zero for {specification['input']} does not match its owner U+ declaration"
            )
        if owner_case.get("commit_text_preview") != target:
            raise ValueError(f"commit preview does not link to candidate zero for {specification['input']}")
        if owner_case.get("highlighted_candidate_index") != 0:
            raise ValueError(f"owner target is not highlighted at zero for {specification['input']}")
        atomic_characters = []
        char_codes = []
        for code in specification["atomic_codes"]:
            atomic_target = cases[code]["all_candidates"][0]["text"]
            if len(atomic_target) != 1:
                raise ValueError(f"atomic candidate zero for {code} must be one Unicode scalar")
            atomic_characters.append(atomic_target)
            char_codes.append(f"{atomic_target}={code}")
        if "".join(atomic_characters) != target:
            raise ValueError(f"atomic candidates do not reconstruct owner target {specification['input']}")
        composition_rows.append(
            {
                "input": specification["input"],
                "target": target,
                "target_codepoints": _codepoint_specification(target),
                "char_codes": char_codes,
                "provenance": (
                    f"derived from cases[{specification['input']}].all_candidates[0].text; "
                    "verified against the owner U+ declaration and linked atomic candidate-zero cases"
                ),
            }
        )

    controls = []
    for specification in CONTROL_TARGET_SPECS:
        candidate_index = specification["candidate_index"]
        candidates = cases[specification["input"]]["all_candidates"]
        if candidate_index >= len(candidates):
            raise ValueError(f"control candidate is absent for {specification['input']}")
        target = candidates[candidate_index]["text"]
        if target != _decode_codepoints(specification["target_codepoints"]):
            raise ValueError(f"control target changed for {specification['input']}")
        controls.append(
            {
                "input": specification["input"],
                "candidate_index": candidate_index,
                "target": target,
                "target_codepoints": _codepoint_specification(target),
                "provenance": (
                    f"derived from cases[{specification['input']}].all_candidates"
                    f"[{candidate_index}].text"
                ),
            }
        )
    return composition_rows, controls


def _validate_raw_document(raw: Any) -> dict[str, Any]:
    if not isinstance(raw, dict):
        raise ValueError("raw Cangjie capture must be an object")
    if raw.get("milestone") != "M59" or raw.get("canonical") is not True:
        raise ValueError("raw Cangjie capture is not canonical M59 evidence")
    if raw.get("status") != "raw_cangjie5_capture_complete":
        raise ValueError("raw Cangjie capture status is incomplete")

    capture = raw.get("capture")
    if not isinstance(capture, dict):
        raise ValueError("raw Cangjie capture metadata is absent")
    if (
        capture.get("engine") != "rime/librime"
        or capture.get("version") != "1.17.0"
        or capture.get("librime_commit") != LIBRIME_COMMIT
        or capture.get("schema_id") != "cangjie5"
        or capture.get("modules") != ["default"]
    ):
        raise ValueError("raw Cangjie oracle identity changed")
    _require_git_id(capture.get("source_commit"), "Yune source commit")
    _require_git_id(capture.get("source_tree"), "Yune source tree")
    if capture.get("source_clean") is not True or capture.get("source_status_short") != []:
        raise ValueError("Yune capture tooling source must be exactly clean")

    if capture.get("inputs") != list(EXPECTED_INPUTS):
        raise ValueError("capture metadata does not preserve the required 12-input order")
    if capture.get("inputs_source") != "fixed_m59_cangjie_order_lane":
        raise ValueError("capture input provenance is not the fixed M59 Cangjie lane")
    if capture.get("input_count") != len(EXPECTED_INPUTS):
        raise ValueError("capture metadata input count is incomplete")
    if capture.get("captured_all_pages") is not True or capture.get("page_policy") != PAGE_POLICY:
        raise ValueError("capture metadata does not bind complete paging")
    if (
        capture.get("runtime_options") != RUNTIME_OPTIONS
        or list(capture["runtime_options"]) != list(RUNTIME_OPTIONS)
    ):
        raise ValueError("capture runtime options do not match the shared four-false policy")
    if capture.get("runtime_options_source") != RUNTIME_OPTIONS_SOURCE:
        raise ValueError("capture runtime option source is not canonical")
    if capture.get("additional_runtime_option_patches") != []:
        raise ValueError("capture must not apply undeclared runtime option patches")

    if capture.get("rime_dll_sha256") != RIME_DLL_SHA256:
        raise ValueError("raw capture used the wrong rime.dll")
    if capture.get("rime_deployer_sha256") != RIME_DEPLOYER_SHA256:
        raise ValueError("raw capture used the wrong rime_deployer.exe")
    for name in (
        "source_shared_tree_sha256",
        "staged_shared_tree_sha256",
        "deployed_build_tree_sha256",
    ):
        _require_sha256(capture.get(name), name)

    repository_records = capture.get("source_repositories")
    if not isinstance(repository_records, dict) or list(repository_records) != list(
        SOURCE_REPOSITORIES
    ):
        raise ValueError("source repository inventory is incomplete or reordered")
    for repository in SOURCE_REPOSITORIES:
        _validate_repository_record(repository_records[repository], repository)

    tool_hashes = capture.get("tool_hashes")
    if not isinstance(tool_hashes, dict) or set(tool_hashes) != {
        "capture_script_sha256",
        "probe_sha256",
        "curator_sha256",
    }:
        raise ValueError("capture tool hash inventory is incomplete")
    for name, value in tool_hashes.items():
        _require_sha256(value, name)

    parameters = capture.get("effective_parameters")
    required_parameter_fields = {
        "oracle_root",
        "cangjie_dir",
        "essay",
        "raw_output",
        "output",
        "work_root",
        "work_root_source",
        "keep_work_root",
        "inputs",
        "schema_id",
        "modules",
        "expected_rime_dll_sha256",
        "expected_rime_deployer_sha256",
        "expected_repository_commits",
        "expected_repository_trees",
        "runtime_options",
        "runtime_options_source",
        "additional_runtime_option_patches",
        "page_policy",
        "serialization",
        "path_serialization_policy",
    }
    if not isinstance(parameters, dict) or set(parameters) != required_parameter_fields:
        raise ValueError("capture effective parameters are incomplete or contain unknown fields")
    for path_field in ("oracle_root", "cangjie_dir", "essay", "raw_output", "output"):
        if not isinstance(parameters[path_field], str) or not parameters[path_field]:
            raise ValueError(f"effective parameter {path_field} must be a non-empty path token")
    if parameters["work_root_source"] == "explicit":
        if not isinstance(parameters["work_root"], str) or not parameters["work_root"]:
            raise ValueError("explicit work_root must be a non-empty path token")
    elif (
        parameters["work_root_source"] != "generated_unique_temp"
        or parameters["work_root"] != "generated_disposable"
    ):
        raise ValueError("generated work_root provenance is not canonical")
    if (
        parameters["inputs"] != list(EXPECTED_INPUTS)
        or parameters["schema_id"] != "cangjie5"
        or parameters["modules"] != ["default"]
        or parameters["expected_rime_dll_sha256"] != RIME_DLL_SHA256
        or parameters["expected_rime_deployer_sha256"] != RIME_DEPLOYER_SHA256
        or parameters["runtime_options"] != RUNTIME_OPTIONS
        or list(parameters["runtime_options"]) != list(RUNTIME_OPTIONS)
        or parameters["runtime_options_source"] != RUNTIME_OPTIONS_SOURCE
        or parameters["additional_runtime_option_patches"] != []
        or parameters["page_policy"] != PAGE_POLICY
        or parameters["serialization"] != SERIALIZATION
        or parameters["path_serialization_policy"]
        != "repo-relative forward-slash paths; external paths replaced with external/<role>"
    ):
        raise ValueError("capture effective parameters contradict the raw evidence")
    expected_commits = {
        repository: identity["commit"]
        for repository, identity in SOURCE_REPOSITORIES.items()
    }
    expected_trees = {
        repository: identity["tree"] for repository, identity in SOURCE_REPOSITORIES.items()
    }
    if parameters["expected_repository_commits"] != expected_commits:
        raise ValueError("effective repository commits are not the pinned M59 identities")
    if parameters["expected_repository_trees"] != expected_trees:
        raise ValueError("effective repository trees are not the pinned M59 identities")

    output_provenance = capture.get("output_provenance")
    if not isinstance(output_provenance, dict) or set(output_provenance) != {
        "raw",
        "curated",
    }:
        raise ValueError("raw/curated output provenance is incomplete")
    expected_outputs = (
        (
            "raw",
            parameters["raw_output"],
            "scripts/capture-upstream-cangjie5.ps1",
        ),
        (
            "curated",
            parameters["output"],
            "scripts/curate-upstream-cangjie5.py",
        ),
    )
    for name, expected_path, generator in expected_outputs:
        record = output_provenance[name]
        if record != {
            "path": expected_path,
            "existed_before_capture": False,
            "write_policy": WRITE_POLICY,
            "generated_by": generator,
        }:
            raise ValueError(f"{name} output provenance does not match create-new policy")

    commands = capture.get("commands")
    if not isinstance(commands, dict) or set(commands) != {"deploy", "capture", "curate"}:
        raise ValueError("capture commands are incomplete")
    if commands != _expected_commands(parameters):
        raise ValueError("capture commands do not match the exact effective parameters")

    oracle = raw.get("oracle")
    if not isinstance(oracle, dict) or (
        oracle.get("engine") != "rime/librime"
        or oracle.get("version") != "1.17.0"
        or oracle.get("commit") != LIBRIME_COMMIT
        or oracle.get("dll_sha256") != RIME_DLL_SHA256
        or oracle.get("deployer_sha256") != RIME_DEPLOYER_SHA256
    ):
        raise ValueError("raw oracle block contradicts capture identity")
    expected_oracle_paths = (
        {
            "dll": "external/rime-dll",
            "deployer": "external/rime-deployer",
        }
        if parameters["oracle_root"].startswith("external/")
        else {
            "dll": f"{parameters['oracle_root'].rstrip('/')}/extract/dist/lib/rime.dll",
            "deployer": (
                f"{parameters['oracle_root'].rstrip('/')}/extract/dist/bin/"
                "rime_deployer.exe"
            ),
        }
    )
    if oracle.get("dll") != expected_oracle_paths["dll"] or oracle.get(
        "deployer"
    ) != expected_oracle_paths["deployer"]:
        raise ValueError("raw oracle binary paths contradict the effective oracle root")

    schema = raw.get("schema")
    if not isinstance(schema, dict) or (
        schema.get("yune_facing_schema_id") != "cangjie5"
        or schema.get("source_repo") != "rime/rime-cangjie"
        or schema.get("source_commit")
        != SOURCE_REPOSITORIES["rime/rime-cangjie"]["commit"]
        or schema.get("source_tree")
        != SOURCE_REPOSITORIES["rime/rime-cangjie"]["tree"]
    ):
        raise ValueError("raw schema identity is incomplete")
    if schema.get("dependency_commits") != expected_commits:
        raise ValueError("raw schema dependency commits are incomplete")
    if schema.get("dependency_trees") != expected_trees:
        raise ValueError("raw schema dependency trees are incomplete")

    if raw.get("owner_target_specs") != list(OWNER_TARGET_SPECS):
        raise ValueError("owner target declarations are incomplete or altered")
    if raw.get("control_target_specs") != list(CONTROL_TARGET_SPECS):
        raise ValueError("control target declarations are incomplete or altered")
    if raw.get("inputs") != list(EXPECTED_INPUTS):
        raise ValueError("top-level input order is incomplete")
    cases_value = raw.get("cases")
    if not isinstance(cases_value, list) or len(cases_value) != len(EXPECTED_INPUTS):
        raise ValueError("raw capture must contain exactly 12 cases")
    if [case.get("input") if isinstance(case, dict) else None for case in cases_value] != list(
        EXPECTED_INPUTS
    ):
        raise ValueError("raw case order does not match the declared input order")

    cases: dict[str, dict[str, Any]] = {}
    page_sizes = set()
    for expected_input, case in zip(EXPECTED_INPUTS, cases_value):
        page_size, _ = _validate_case(case, expected_input)
        page_sizes.add(page_size)
        cases[expected_input] = case
    observed_page_sizes = sorted(page_sizes)
    if capture.get("page_sizes_observed") != observed_page_sizes:
        raise ValueError("capture metadata page sizes do not match raw cases")
    options = raw.get("options")
    if not isinstance(options, dict) or (
        options.get("runtime_options") != RUNTIME_OPTIONS
        or list(options["runtime_options"]) != list(RUNTIME_OPTIONS)
        or options.get("runtime_options_source") != RUNTIME_OPTIONS_SOURCE
        or options.get("additional_runtime_option_patches") != []
        or options.get("page_sizes_observed") != observed_page_sizes
        or options.get("custom_yaml") != "default.custom.yaml only selects cangjie5"
    ):
        raise ValueError("raw options block contradicts capture metadata")

    composition_rows, control_rows = _validate_target_linkage(cases)
    return {
        "capture": capture,
        "cases": cases,
        "composition_rows": composition_rows,
        "control_rows": control_rows,
    }


def _dict_body_rows(path: Path) -> Iterable[tuple[str, str, str]]:
    in_body = False
    with path.open(encoding="utf-8", newline="") as handle:
        for physical_line in handle:
            line = physical_line.rstrip("\n").rstrip("\r")
            if not in_body:
                if line.strip() == "...":
                    in_body = True
                continue
            if not line or line.startswith("#"):
                continue
            fields = line.split("\t")
            if len(fields) >= 2 and fields[0] and fields[1]:
                yield fields[0], fields[1], "\t".join(fields)
    if not in_body:
        raise ValueError(f"dictionary is missing its YAML body marker: {path}")


def _build_source_slice(
    cangjie_dir: Path,
    essay_path: Path,
    cases: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    import_rows: dict[str, list[str]] = {}
    cohort_text_by_code: dict[str, set[str]] = {code: set() for code in ATOMIC_CODES}
    cohort_characters = set()
    for file_name in DICT_FILES:
        path = cangjie_dir / file_name
        if not path.is_file():
            raise ValueError(f"pinned Cangjie dictionary file is absent: {path}")
        rows = []
        for text, code, raw_line in _dict_body_rows(path):
            if code in cohort_text_by_code:
                rows.append(raw_line)
                cohort_text_by_code[code].add(text)
                cohort_characters.update(text)
        if rows:
            import_rows[file_name] = rows

    missing_cohorts = [code for code, texts in cohort_text_by_code.items() if not texts]
    if missing_cohorts:
        raise ValueError(f"atomic dictionary cohorts are absent: {missing_cohorts}")
    for code in ATOMIC_CODES:
        captured = cases[code]["all_candidates"][0]["text"]
        if captured not in cohort_text_by_code[code]:
            raise ValueError(f"atomic candidate zero for {code} is absent from its source cohort")

    required_vocabulary_characters = set()
    for specification in (*OWNER_TARGET_SPECS, *CONTROL_TARGET_SPECS):
        required_vocabulary_characters.update(_decode_codepoints(specification["target_codepoints"]))
    required_vocabulary_characters.update(cohort_characters)

    essay_weights: dict[str, str] = {}
    with essay_path.open(encoding="utf-8", newline="") as handle:
        for physical_line in handle:
            line = physical_line.rstrip("\n").rstrip("\r")
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) >= 2 and parts[0] not in essay_weights:
                essay_weights[parts[0]] = parts[1]
    required_owner_and_control = {
        character
        for specification in (*OWNER_TARGET_SPECS, *CONTROL_TARGET_SPECS)
        for character in _decode_codepoints(specification["target_codepoints"])
    }
    missing_vocabulary = sorted(required_owner_and_control - set(essay_weights), key=ord)
    if missing_vocabulary:
        missing_specs = [_codepoint_specification(character) for character in missing_vocabulary]
        raise ValueError(f"required preset-vocabulary rows are absent: {missing_specs}")

    vocabulary_rows = [
        f"{character}\t{essay_weights[character]}"
        for character in sorted(required_vocabulary_characters, key=ord)
        if character in essay_weights
    ]
    return {
        "policy": "d48_cangjie5_exact_code_cohorts_for_char_by_char_composition",
        "schema_data": "rime/rime-cangjie",
        "schema_data_commit": SOURCE_REPOSITORIES["rime/rime-cangjie"]["commit"],
        "schema_data_tree": SOURCE_REPOSITORIES["rime/rime-cangjie"]["tree"],
        "vocabulary": "essay",
        "essay_vocabulary_file": "rime-essay/essay.txt",
        "atomic_codes": list(ATOMIC_CODES),
        "note": (
            "Complete exact-code cohorts for the seven candidate-zero composition "
            "constituents plus pinned rime-essay weights. Expected target text is "
            "derived only from the validated raw oracle cases."
        ),
        "import_rows": import_rows,
        "vocabulary_rows": vocabulary_rows,
    }


def _validate_live_inputs(
    raw: dict[str, Any],
    oracle_root: Path,
    cangjie_dir: Path,
    essay_path: Path,
) -> dict[str, dict[str, Any]]:
    capture = raw["capture"]
    repository_records = capture["source_repositories"]
    schema_root = cangjie_dir.parent.resolve(strict=True)
    expected_cangjie = (schema_root / SOURCE_REPOSITORIES["rime/rime-cangjie"]["directory"]).resolve(
        strict=True
    )
    if cangjie_dir.resolve(strict=True) != expected_cangjie:
        raise ValueError("cangjie-dir is not the pinned schema-root repository")
    expected_essay = (
        schema_root / SOURCE_REPOSITORIES["rime/rime-essay"]["directory"] / "essay.txt"
    ).resolve(strict=True)
    if essay_path.resolve(strict=True) != expected_essay:
        raise ValueError("essay path is not the pinned rime-essay file")

    live_states = {}
    for repository, identity in SOURCE_REPOSITORIES.items():
        path = (schema_root / identity["directory"]).resolve(strict=True)
        state = _live_git_state(path)
        if state != repository_records[repository]:
            raise ValueError(f"source repository changed since raw capture: {repository}")
        live_states[repository] = state

    extract = oracle_root.resolve(strict=True) / "extract"
    dll = extract / "dist" / "lib" / "rime.dll"
    deployer = extract / "dist" / "bin" / "rime_deployer.exe"
    if _file_sha256(dll) != capture["rime_dll_sha256"]:
        raise ValueError("rime.dll changed since raw capture")
    if _file_sha256(deployer) != capture["rime_deployer_sha256"]:
        raise ValueError("rime_deployer.exe changed since raw capture")

    repo_root = Path(__file__).resolve().parent.parent
    tools = {
        "capture_script_sha256": repo_root / "scripts" / "capture-upstream-cangjie5.ps1",
        "probe_sha256": repo_root / "scripts" / "oracle-rime-probe.cs",
        "curator_sha256": Path(__file__).resolve(),
    }
    for hash_name, path in tools.items():
        if _file_sha256(path) != capture["tool_hashes"][hash_name]:
            raise ValueError(f"capture tool changed since raw capture: {path.name}")
    tool_state = _live_git_state(repo_root)
    expected_tool_state = {
        "commit": capture["source_commit"],
        "tree": capture["source_tree"],
        "clean": capture["source_clean"],
        "status_short": capture["source_status_short"],
    }
    if tool_state != expected_tool_state:
        raise ValueError("Yune capture source changed since raw capture")
    return live_states


def _parse_args(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--raw-input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--oracle-root", type=Path, required=True)
    parser.add_argument("--cangjie-dir", type=Path, required=True)
    parser.add_argument("--essay", type=Path, required=True)
    return parser.parse_args(argv)


def _curate(argv: list[str] | None = None) -> int:
    args = _parse_args(argv)
    raw_input = args.raw_input.resolve(strict=True)
    output = args.output.resolve(strict=False)
    if raw_input == output:
        raise ValueError("raw input and curated output must be different files")
    raw, raw_input_sha256 = _read_canonical_json(
        raw_input, "raw Cangjie oracle capture"
    )
    validated = _validate_raw_document(raw)
    parameters = raw["capture"]["effective_parameters"]
    cli_bindings = {
        "raw_output": _evidence_path(raw_input, "raw-output"),
        "output": _evidence_path(output, "output"),
        "oracle_root": _evidence_path(args.oracle_root, "oracle-root"),
        "cangjie_dir": _evidence_path(args.cangjie_dir, "cangjie-dir"),
        "essay": _evidence_path(args.essay, "essay"),
    }
    for parameter_name, observed_path in cli_bindings.items():
        if observed_path != parameters[parameter_name]:
            raise ValueError(
                f"CLI {parameter_name} does not match raw effective parameters"
            )
    _validate_live_inputs(raw, args.oracle_root, args.cangjie_dir, args.essay)
    source_slice = _build_source_slice(
        args.cangjie_dir, args.essay, validated["cases"]
    )

    curated = copy.deepcopy(raw)
    curated["status"] = "cangjie5_capture_curated_complete"
    curated["composition_rows"] = validated["composition_rows"]
    curated["control_rows"] = validated["control_rows"]
    curated["source_slice"] = source_slice
    curated["curation"] = {
        "version": CURATOR_VERSION,
        "raw_input_sha256": raw_input_sha256,
        "curator_sha256": _file_sha256(Path(__file__).resolve()),
        "target_policy": (
            "owner targets are derived from captured candidate zero, verified against "
            "ASCII U+ declarations, and reconstructed from atomic candidate-zero cases"
        ),
        "source_slice_policy": source_slice["policy"],
        "serialization": SERIALIZATION,
        "write_policy": WRITE_POLICY,
    }
    content = _canonical_json_bytes(curated)
    # Revalidate all mutable external identities immediately before publication.
    _validate_live_inputs(raw, args.oracle_root, args.cangjie_dir, args.essay)
    if _file_sha256(raw_input) != raw_input_sha256:
        raise ValueError("raw Cangjie oracle input changed before curated publication")
    _write_atomic_create_new(output, content)
    print(
        f"curated {output}: {len(validated['composition_rows'])} owner targets, "
        f"{sum(len(rows) for rows in source_slice['import_rows'].values())} source rows"
    )
    return 0


def main() -> int:
    try:
        return _curate()
    except (FileExistsError, OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

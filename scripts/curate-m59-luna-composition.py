#!/usr/bin/env python3
"""Curate the M59 luna leading-single composition oracle fixture.

Reads the raw librime probe captures emitted by
scripts/capture-m59-luna-composition.ps1 (complete candidate lists + the
moboyi -> mo/bo/yi partial-selection composition chain) and writes the
checked-in fixture consumed by
crates/yune-core/tests/upstream_luna_leading_single_composition.rs.

Usage: curate-m59-luna-composition.py <pages.json> <compose.json> <metadata.json> <output.json>
"""
import collections
import hashlib
import json
import os
import re
import sys
import tempfile
from pathlib import Path


SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
SCHEMA_REPO = "rime/rime-luna-pinyin"
DEPENDENCY_PARAMETER_NAMES = {
    "rime/rime-prelude": "expected_prelude_commit",
    "rime/rime-essay": "expected_essay_commit",
    "rime/rime-stroke": "expected_stroke_commit",
}
SOURCE_REPOSITORIES = (SCHEMA_REPO, *DEPENDENCY_PARAMETER_NAMES)
SOURCE_REPOSITORY_SET = frozenset(SOURCE_REPOSITORIES)
TREE_HASH_ALGORITHM = (
    "sha256 of ordinal path<TAB>file-sha256 rows joined by LF with final LF"
)
QUERY_MUTATION_POLICY = "raw shared/build hashes must remain identical before/after capture"
DEPLOYMENT_POLICY = (
    "clean disposable deploy from pinned tracked source files plus pinned upstream OpenCC"
)
TIMESTAMP_NORMALIZATION_POLICY = (
    "all staged files use fixed half-second LastWriteTimeUtc verified by exact FileTimeUtc "
    "readback before deployment"
)
STAGED_TIMESTAMP_UTC = "2000-01-01T00:00:00.500Z"
CURATOR_VERSION = 5
SOURCE_ROW_POLICY = "m59_lane_b_complete_order_and_partial_selection_composition"
ORDER_HASH_ALGORITHM = (
    "sha256 of repeated u64be utf8-byte-length followed by utf8 candidate text"
)
REQUIRED_EFFECTIVE_PARAMETERS = {
    "oracle_root",
    "output",
    "expected_rime_dll_sha256",
    "expected_rime_deployer_sha256",
    "expected_luna_pinyin_commit",
    *DEPENDENCY_PARAMETER_NAMES.values(),
}

# The reachable leading single each input composes toward. These are the
# targets the M59 acceptance rows cite; their POSITIONS come from the oracle.
TARGETS = {
    "moboyi": "莫",  # PRIMARY non-lexicon case: moboyi -> 莫伯洢
    "boyi": "伯",
    "yi": "洢",
    "zhonggao": "中",
    "zhongguo": "中",
    "gao": "高",
    "guo": "國",
}

# Composition scenarios: (scenario name in the compose capture, human input,
# role note).
COMPOSITIONS = [
    ("moboyi_compose", "moboyi", "PRIMARY non-lexicon phrase", "莫伯洢"),
]


def _require_shape(value, pattern, label):
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        raise ValueError(f"{label} has an invalid pinned identity shape")
    return value


def _quote_command_arg(value):
    return "'" + value.replace("'", "''") + "'"


def _ordered_text_sha256(candidates):
    digest = hashlib.sha256()
    for candidate in candidates:
        encoded = candidate["text"].encode("utf-8")
        digest.update(len(encoded).to_bytes(8, "big"))
        digest.update(encoded)
    return digest.hexdigest()


def _is_plain_int(value):
    return isinstance(value, int) and not isinstance(value, bool)


def validate_metadata(metadata):
    if not isinstance(metadata, dict):
        raise ValueError("metadata must be an object")
    dll_hash = _require_shape(metadata.get("rime_dll_sha256"), SHA256_RE, "rime_dll_sha256")
    deployer_hash = _require_shape(
        metadata.get("rime_deployer_sha256"), SHA256_RE, "rime_deployer_sha256"
    )
    if metadata.get("schema_source_repo") != SCHEMA_REPO:
        raise ValueError(f"schema_source_repo must be {SCHEMA_REPO}")
    source_commit = _require_shape(
        metadata.get("schema_source_commit"), COMMIT_RE, "schema_source_commit"
    )
    dependencies = metadata.get("dependency_commits")
    if not isinstance(dependencies, dict) or set(dependencies) != set(DEPENDENCY_PARAMETER_NAMES):
        raise ValueError(
            "dependency_commits must identify exactly rime-prelude, rime-essay, and rime-stroke"
        )
    normalized_dependencies = {
        repo: _require_shape(dependencies[repo], COMMIT_RE, f"dependency_commits[{repo}]")
        for repo in DEPENDENCY_PARAMETER_NAMES
    }
    clean_repositories = metadata.get("source_repositories_clean")
    if (
        not isinstance(clean_repositories, dict)
        or set(clean_repositories) != SOURCE_REPOSITORY_SET
    ):
        raise ValueError("source_repositories_clean must identify every pinned source repository")
    if any(clean_repositories[repo] is not True for repo in SOURCE_REPOSITORIES):
        raise ValueError("all pinned source repositories must be recorded clean")
    source_git_trees = metadata.get("source_git_trees")
    if (
        not isinstance(source_git_trees, dict)
        or set(source_git_trees) != SOURCE_REPOSITORY_SET
    ):
        raise ValueError("source_git_trees must identify every pinned source repository")
    normalized_git_trees = {
        repo: _require_shape(source_git_trees[repo], COMMIT_RE, f"source_git_trees[{repo}]")
        for repo in SOURCE_REPOSITORIES
    }
    queried_data = metadata.get("queried_data")
    required_queried_fields = {
        "shared_path",
        "build_path",
        "shared_tree_sha256",
        "build_tree_sha256",
        "tree_hash_algorithm",
        "mutation_policy",
        "deployment_policy",
        "timestamp_normalization_policy",
        "staged_timestamp_utc",
        "default_custom_sha256",
        "opencc_tree_sha256",
    }
    if not isinstance(queried_data, dict) or set(queried_data) != required_queried_fields:
        raise ValueError("queried_data must bind the exact shared/build trees used by capture")
    expected_logical_paths = {
        "shared_path": "disposable/shared",
        "build_path": "disposable/user/build",
    }
    for path_field, expected_path in expected_logical_paths.items():
        if queried_data[path_field] != expected_path:
            raise ValueError(
                f"queried_data.{path_field} must use stable logical path {expected_path}"
            )
    for hash_field in (
        "shared_tree_sha256",
        "build_tree_sha256",
        "default_custom_sha256",
        "opencc_tree_sha256",
    ):
        _require_shape(queried_data[hash_field], SHA256_RE, f"queried_data.{hash_field}")
    if queried_data["tree_hash_algorithm"] != TREE_HASH_ALGORITHM:
        raise ValueError("queried_data.tree_hash_algorithm is not canonical")
    if queried_data["mutation_policy"] != QUERY_MUTATION_POLICY:
        raise ValueError("queried_data.mutation_policy is not canonical")
    if queried_data["deployment_policy"] != DEPLOYMENT_POLICY:
        raise ValueError("queried_data.deployment_policy is not canonical")
    if queried_data["timestamp_normalization_policy"] != TIMESTAMP_NORMALIZATION_POLICY:
        raise ValueError("queried_data.timestamp_normalization_policy is not canonical")
    if queried_data["staged_timestamp_utc"] != STAGED_TIMESTAMP_UTC:
        raise ValueError("queried_data.staged_timestamp_utc is not canonical")
    parameters = metadata.get("effective_parameters")
    if not isinstance(parameters, dict) or set(parameters) != REQUIRED_EFFECTIVE_PARAMETERS:
        raise ValueError(
            "effective_parameters must contain exactly the capture script's effective parameters"
        )
    for key, value in parameters.items():
        if not isinstance(value, str) or not value:
            raise ValueError(f"effective_parameters.{key} must be a non-empty string")
    expected_bindings = {
        "expected_rime_dll_sha256": dll_hash,
        "expected_rime_deployer_sha256": deployer_hash,
        "expected_luna_pinyin_commit": source_commit,
    }
    expected_bindings.update(
        {
            parameter_name: normalized_dependencies[repo]
            for repo, parameter_name in DEPENDENCY_PARAMETER_NAMES.items()
        }
    )
    for key, expected in expected_bindings.items():
        if parameters[key] != expected:
            raise ValueError(
                f"effective_parameters.{key} does not match the captured identity"
            )
    expected_invocation = " ".join(
        [
            "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/capture-m59-luna-composition.ps1",
            f"-OracleRoot {_quote_command_arg(parameters['oracle_root'])}",
            f"-Output {_quote_command_arg(parameters['output'])}",
            f"-ExpectedRimeDllSha256 {_quote_command_arg(parameters['expected_rime_dll_sha256'])}",
            f"-ExpectedRimeDeployerSha256 {_quote_command_arg(parameters['expected_rime_deployer_sha256'])}",
            f"-ExpectedLunaPinyinCommit {_quote_command_arg(parameters['expected_luna_pinyin_commit'])}",
            f"-ExpectedPreludeCommit {_quote_command_arg(parameters['expected_prelude_commit'])}",
            f"-ExpectedEssayCommit {_quote_command_arg(parameters['expected_essay_commit'])}",
            f"-ExpectedStrokeCommit {_quote_command_arg(parameters['expected_stroke_commit'])}",
        ]
    )
    if metadata.get("actual_invocation") != expected_invocation:
        raise ValueError("actual_invocation does not match effective_parameters")
    return {
        "dll_hash": dll_hash,
        "deployer_hash": deployer_hash,
        "source_commit": source_commit,
        "dependencies": normalized_dependencies,
        "clean_repositories": clean_repositories,
        "source_git_trees": normalized_git_trees,
        "queried_data": queried_data,
        "parameters": parameters,
        "invocation": expected_invocation,
    }


def _write_atomic(path, content):
    output = Path(path)
    output.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temp_name = tempfile.mkstemp(
        prefix=f".{output.name}.", suffix=".tmp", dir=output.parent
    )
    temp_path = Path(temp_name)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp_path, output)
    except Exception:
        temp_path.unlink(missing_ok=True)
        output.unlink(missing_ok=True)
        raise


def _curate(argv=None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if len(args) != 4:
        raise SystemExit(
            "usage: curate-m59-luna-composition.py "
            "<pages.json> <compose.json> <metadata.json> <output.json>"
        )
    pages_path, compose_path, metadata_path, out_path = args
    pages = json.load(open(pages_path, encoding="utf-8-sig"))
    compose = json.load(open(compose_path, encoding="utf-8-sig"))
    metadata = json.load(open(metadata_path, encoding="utf-8-sig"))
    validated_metadata = validate_metadata(metadata)

    if not isinstance(pages, list) or len(pages) != len(TARGETS):
        raise ValueError(f"oracle capture must contain exactly {len(TARGETS)} cases")
    case_names = [case.get("input") for case in pages if isinstance(case, dict)]
    if len(case_names) != len(pages) or len(set(case_names)) != len(case_names):
        raise ValueError("oracle capture cases must be objects with unique input values")
    if case_names != list(TARGETS):
        raise ValueError(
            f"oracle capture case order mismatch: expected={list(TARGETS)}, actual={case_names}"
        )

    inputs = {}
    for case in pages:
        name = case["input"]
        if case.get("rime_get_input") != name:
            raise ValueError(f"oracle capture for {name} does not preserve rime_get_input")
        if not _is_plain_int(case.get("page_no")) or case["page_no"] != 0:
            raise ValueError(f"oracle capture for {name} must start at page_no 0")
        processed = case.get("processed")
        if (
            not isinstance(processed, list)
            or len(processed) != len(name)
            or any(not _is_plain_int(value) or value != 1 for value in processed)
        ):
            raise ValueError(f"oracle capture for {name} has invalid processed-key results")
        if case.get("captured_all_pages") is not True:
            raise ValueError(f"oracle capture for {name} is incomplete")
        page_size = case.get("page_size")
        if not isinstance(page_size, int) or isinstance(page_size, bool) or page_size <= 0:
            raise ValueError(f"oracle capture for {name} has invalid page_size")
        candidates = case.get("all_candidates")
        if not isinstance(candidates, list) or not candidates:
            raise ValueError(f"oracle capture for {name} has no all_candidates rows")
        ordered = []
        for index, candidate in enumerate(candidates):
            if not isinstance(candidate, dict) or not isinstance(candidate.get("text"), str):
                raise ValueError(f"oracle capture for {name} candidate {index} has no string text")
            if (
                not _is_plain_int(candidate.get("global_index"))
                or candidate["global_index"] != index
            ):
                raise ValueError(
                    f"oracle capture for {name} candidate {index} has a non-contiguous global_index"
                )
            ordered.append(candidate["text"])
        target = TARGETS[name]
        if target not in ordered:
            raise ValueError(f"oracle capture for {name} does not contain target {target}")
        selected = case.get("selected_candidates")
        if not isinstance(selected, list):
            raise ValueError(f"oracle capture for {name} has no selected_candidates page")
        case_pages = case.get("pages")
        if not isinstance(case_pages, list) or not case_pages:
            raise ValueError(f"oracle capture for {name} has no pages")
        flattened = []
        for page_index, page in enumerate(case_pages):
            if (
                not isinstance(page, dict)
                or not _is_plain_int(page.get("page_no"))
                or page["page_no"] != page_index
            ):
                raise ValueError(
                    f"oracle capture for {name} page {page_index} is missing or non-contiguous"
                )
            if (
                not _is_plain_int(page.get("page_size"))
                or page["page_size"] != page_size
            ):
                raise ValueError(
                    f"oracle capture for {name} page {page_index} changed page_size"
                )
            should_be_last = page_index == len(case_pages) - 1
            if page.get("is_last_page") is not should_be_last:
                raise ValueError(
                    f"oracle capture for {name} page {page_index} has an invalid last-page marker"
                )
            page_candidates = page.get("candidates")
            if not isinstance(page_candidates, list):
                raise ValueError(
                    f"oracle capture for {name} page {page_index} has no candidate array"
                )
            if (
                (not should_be_last and len(page_candidates) != page_size)
                or (should_be_last and not 1 <= len(page_candidates) <= page_size)
            ):
                raise ValueError(
                    f"oracle capture for {name} page {page_index} has invalid candidate cardinality"
                )
            for local_index, candidate in enumerate(page_candidates):
                if (
                    not isinstance(candidate, dict)
                    or not _is_plain_int(candidate.get("index"))
                    or candidate["index"] != local_index
                    or not _is_plain_int(candidate.get("global_index"))
                    or candidate["global_index"] != len(flattened)
                    or not isinstance(candidate.get("text"), str)
                ):
                    raise ValueError(
                        f"oracle capture for {name} page {page_index} candidate {local_index} "
                        "has invalid local/global position or text"
                    )
                flattened.append(candidate)
        if flattened != candidates:
            raise ValueError(
                f"oracle capture for {name} pages do not flatten exactly to all_candidates"
            )
        if (
            not isinstance(case.get("is_last_page"), bool)
            or case["is_last_page"] is not case_pages[0]["is_last_page"]
        ):
            raise ValueError(
                f"oracle capture for {name} top-level is_last_page disagrees with page 0"
            )
        if selected != case_pages[0]["candidates"]:
            raise ValueError(
                f"oracle capture for {name} selected_candidates does not equal page 0"
            )
        page_0 = []
        for index, candidate in enumerate(selected):
            if not isinstance(candidate, dict) or not isinstance(candidate.get("text"), str):
                raise ValueError(
                    f"oracle capture for {name} selected candidate {index} has no string text"
                )
            page_0.append(candidate["text"])
        inputs[name] = {
            "input": name,
            "target_single": target,
            "target_global_index": ordered.index(target) if target in ordered else None,
            "page_0": page_0,
            "page_size": page_size,
            "captured_all_pages": True,
            "total_candidates_captured": len(ordered),
            "total_unique_captured": len(set(ordered)),
            "ordered_text_sha256": _ordered_text_sha256(candidates),
        }

    compose_by_scenario = collections.OrderedDict()
    for snap in compose:
        compose_by_scenario.setdefault(snap["scenario"], []).append(snap)

    compositions = {}
    for scenario, human_input, note, expected_commit in COMPOSITIONS:
        snaps = compose_by_scenario.get(scenario, [])
        if not snaps:
            raise ValueError(f"composition capture is missing scenario {scenario}")
        chain = [
            {"step": s.get("label"), "preedit": s.get("preedit"), "commit_text": s.get("commit_text")}
            for s in snaps
        ]
        final_commit = next((s["commit_text"] for s in reversed(chain) if s["commit_text"]), None)
        if final_commit != expected_commit:
            raise ValueError(
                f"composition {scenario} commit mismatch: expected {expected_commit}, got {final_commit!r}"
            )
        compositions[human_input] = {
            "role": note,
            "description": "librime composes the phrase by partial single-character selection "
            "(preedit accumulation, single commit at end).",
            "final_commit": final_commit,
            "chain": chain,
        }

    fixture = {
        "oracle": {
            "engine": "rime/librime",
            "engine_tag": "1.17.0",
            "engine_commit": "33e78140250125871856cdc5b42ddc6a5fcd3cd4",
            "canonical_repository": "https://github.com/rime/librime",
            "release_url": "https://github.com/rime/librime/releases/tag/1.17.0",
            "schema": "luna_pinyin",
            "dll_sha256": validated_metadata["dll_hash"],
            "deployer_sha256": validated_metadata["deployer_hash"],
        },
        "schema": {
            "source_repo": SCHEMA_REPO,
            "source_commit": validated_metadata["source_commit"],
            "dependency_commits": validated_metadata["dependencies"],
            "source_repositories_clean": validated_metadata["clean_repositories"],
            "source_git_trees": validated_metadata["source_git_trees"],
        },
        "capture": {
            "method": "scripts/capture-m59-luna-composition.ps1 + scripts/curate-m59-luna-composition.py "
            "via scripts/oracle-rime-probe.cs (DllImport rime.dll)",
            "modules": ["default"],
            "actual_invocation": validated_metadata["invocation"],
            "effective_parameters": validated_metadata["parameters"],
            "page_policy": "RimeProbe.Capture all pages; incomplete pagination is fatal",
            "source_row_policy": SOURCE_ROW_POLICY,
            "curator_version": CURATOR_VERSION,
            "order_hash_algorithm": ORDER_HASH_ALGORITHM,
            "queried_data": validated_metadata["queried_data"],
            "note": "Complete Lane B candidate text/order/position capture plus partial-selection "
            "composition provenance for M59 D-48. PRIMARY case: moboyi -> the non-lexicon "
            "phrase 莫伯洢. Current Yune order divergences remain open until the owning closure "
            "increments land.",
        },
        "inputs": inputs,
        "cases": pages,
        "compositions": compositions,
        "composition_snapshots": compose,
    }
    _write_atomic(out_path, json.dumps(fixture, ensure_ascii=False, indent=2) + "\n")
    sys.stderr.write("wrote " + out_path + "\n")
    return 0


def main(argv=None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if len(args) != 4:
        raise SystemExit(
            "usage: curate-m59-luna-composition.py "
            "<pages.json> <compose.json> <metadata.json> <output.json>"
        )
    output = Path(args[3])
    try:
        return _curate(args)
    except Exception:
        output.unlink(missing_ok=True)
        if output.parent.is_dir():
            for stale_temp in output.parent.glob(f".{output.name}.*.tmp"):
                stale_temp.unlink(missing_ok=True)
        raise


if __name__ == "__main__":
    raise SystemExit(main())

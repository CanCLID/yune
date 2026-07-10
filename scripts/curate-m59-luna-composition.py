#!/usr/bin/env python3
"""Curate the M59 luna leading-single composition oracle fixture.

Reads the raw librime probe captures emitted by
scripts/capture-m59-luna-composition.ps1 (complete candidate lists + the
moboyi -> mo/bo/yi partial-selection composition chain) and create-new writes a
fresh curated oracle for byte/diff review before a separate fixture import.
The checked-in fixture is consumed by
crates/yune-core/tests/upstream_luna_leading_single_composition.rs and is never
overwritten directly by this tool.

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
CURATOR_VERSION = 6
SOURCE_ROW_POLICY = "m59_lane_b_complete_order_and_partial_selection_composition"
ORDER_HASH_ALGORITHM = (
    "sha256 of repeated u64be utf8-byte-length followed by utf8 candidate text"
)
PAGE_POLICY = "RimeProbe.Capture all pages; incomplete pagination is fatal"
RUNTIME_OPTIONS = collections.OrderedDict(
    (
        ("ascii_mode", False),
        ("full_shape", False),
        ("ascii_punct", False),
        ("zh_hans", False),
    )
)
RUNTIME_OPTIONS_SOURCE = (
    "RimeProbe.CaptureWithIdentity+CaptureScenariosWithIdentity/"
    "CaptureRuntimeOptionPolicy"
)
CANONICAL_JSON_SERIALIZATION = {
    "encoding": "utf-8",
    "bom": False,
    "line_endings": "lf",
    "terminal_newline": "exactly_one",
}
WRITE_POLICY = "canonical_utf8_no_bom_lf_one_terminal_lf_create_new"
RAW_PATHS = {
    "pages": "disposable/raw/pages.json",
    "composition": "disposable/raw/compose.json",
    "metadata": "disposable/raw/metadata.json",
}
REQUIRED_CLI_EFFECTIVE_PARAMETERS = {
    "oracle_root",
    "output",
    "expected_rime_dll_sha256",
    "expected_rime_deployer_sha256",
    "expected_luna_pinyin_commit",
    *DEPENDENCY_PARAMETER_NAMES.values(),
}
REQUIRED_EFFECTIVE_PARAMETERS = REQUIRED_CLI_EFFECTIVE_PARAMETERS | {
    "schema_id",
    "modules",
    "inputs",
    "page_policy",
    "runtime_options",
    "runtime_options_source",
    "additional_runtime_option_patches",
    "serialization",
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
EXPECTED_INPUTS = tuple(TARGETS)

# Composition scenarios: (scenario name in the compose capture, human input,
# role note).
COMPOSITIONS = [
    ("moboyi_compose", "moboyi", "PRIMARY non-lexicon phrase", "莫伯洢"),
]
COMPOSITION_LABELS = (
    "moboyi_page0",
    "after_select_mo",
    *(f"bo_pd{index}" for index in range(1, 4)),
    "after_select_bo",
    *(f"yi_pd{index}" for index in range(1, 32)),
    "after_select_yi",
)


def _require_shape(value, pattern, label):
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        raise ValueError(f"{label} has an invalid pinned identity shape")
    return value


def _quote_command_arg(value):
    return "'" + value.replace("'", "''") + "'"


def _load_canonical_json(path, label):
    raw = Path(path).read_bytes()
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
        return json.loads(text)
    except json.JSONDecodeError as error:
        raise ValueError(f"{label} must contain valid JSON") from error


def _ordered_text_sha256(candidates):
    digest = hashlib.sha256()
    for candidate in candidates:
        encoded = candidate["text"].encode("utf-8")
        digest.update(len(encoded).to_bytes(8, "big"))
        digest.update(encoded)
    return digest.hexdigest()


def _is_plain_int(value):
    return isinstance(value, int) and not isinstance(value, bool)


def _file_sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def _expected_curator_invocation(parameters):
    return " ".join(
        [
            "python scripts/curate-m59-luna-composition.py",
            _quote_command_arg(RAW_PATHS["pages"]),
            _quote_command_arg(RAW_PATHS["composition"]),
            _quote_command_arg(RAW_PATHS["metadata"]),
            _quote_command_arg(parameters["output"]),
        ]
    )


def validate_metadata(metadata):
    if not isinstance(metadata, dict):
        raise ValueError("metadata must be an object")
    required_metadata_fields = {
        "rime_dll_sha256",
        "rime_deployer_sha256",
        "schema_source_repo",
        "schema_source_commit",
        "dependency_commits",
        "source_repositories_clean",
        "source_git_trees",
        "queried_data",
        "tool_source",
        "tool_hashes",
        "schema_id",
        "modules",
        "inputs",
        "input_count",
        "page_sizes_observed",
        "captured_all_pages",
        "page_policy",
        "runtime_options",
        "runtime_options_source",
        "additional_runtime_option_patches",
        "serialization",
        "commands",
        "actual_invocation",
        "effective_parameters",
        "curator_effective_parameters",
        "output_provenance",
    }
    if set(metadata) != required_metadata_fields:
        raise ValueError("metadata must contain exactly the Lane B capture contract fields")
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

    tool_source = metadata.get("tool_source")
    required_tool_source_fields = {
        "repository",
        "commit",
        "git_tree",
        "clean",
        "dirty",
        "status_short",
    }
    if not isinstance(tool_source, dict) or set(tool_source) != required_tool_source_fields:
        raise ValueError("tool_source must bind the exact clean Yune source state")
    if tool_source["repository"] != "yune":
        raise ValueError("tool_source.repository must be yune")
    _require_shape(tool_source["commit"], COMMIT_RE, "tool_source.commit")
    _require_shape(tool_source["git_tree"], COMMIT_RE, "tool_source.git_tree")
    if (
        tool_source["clean"] is not True
        or tool_source["dirty"] is not False
        or tool_source["status_short"] != []
    ):
        raise ValueError("canonical Lane B evidence requires a clean Yune tool source")

    tool_hashes = metadata.get("tool_hashes")
    required_tool_hashes = {
        "capture_script_sha256",
        "curator_sha256",
        "probe_sha256",
    }
    if not isinstance(tool_hashes, dict) or set(tool_hashes) != required_tool_hashes:
        raise ValueError("tool_hashes must identify the exact Lane B capture tools")
    for name in required_tool_hashes:
        _require_shape(tool_hashes[name], SHA256_RE, f"tool_hashes.{name}")
    scripts_root = Path(__file__).resolve().parent
    current_tool_hashes = {
        "capture_script_sha256": _file_sha256(
            scripts_root / "capture-m59-luna-composition.ps1"
        ),
        "curator_sha256": _file_sha256(Path(__file__).resolve()),
        "probe_sha256": _file_sha256(scripts_root / "oracle-rime-probe.cs"),
    }
    if tool_hashes != current_tool_hashes:
        raise ValueError("tool_hashes do not match the capture tools executing curation")

    if metadata.get("schema_id") != "luna_pinyin":
        raise ValueError("schema_id must be luna_pinyin")
    if metadata.get("modules") != ["default"]:
        raise ValueError("modules must be exactly ['default']")
    if metadata.get("inputs") != list(EXPECTED_INPUTS):
        raise ValueError("inputs must preserve the ordered seven-input Lane B set")
    if metadata.get("input_count") != len(EXPECTED_INPUTS):
        raise ValueError("input_count must match the ordered Lane B input set")
    page_sizes = metadata.get("page_sizes_observed")
    if (
        not isinstance(page_sizes, list)
        or not page_sizes
        or any(not _is_plain_int(size) or size <= 0 for size in page_sizes)
        or len(set(page_sizes)) != len(page_sizes)
    ):
        raise ValueError("page_sizes_observed must be a non-empty unique positive-integer list")
    if metadata.get("captured_all_pages") is not True:
        raise ValueError("captured_all_pages must be true")
    if metadata.get("page_policy") != PAGE_POLICY:
        raise ValueError("page_policy is not canonical")
    runtime_options = metadata.get("runtime_options")
    if (
        not isinstance(runtime_options, dict)
        or list(runtime_options) != list(RUNTIME_OPTIONS)
        or runtime_options != RUNTIME_OPTIONS
    ):
        raise ValueError("runtime_options must be the ordered four-false RimeProbe policy")
    if metadata.get("runtime_options_source") != RUNTIME_OPTIONS_SOURCE:
        raise ValueError("runtime_options_source is not the pinned RimeProbe policy source")
    if metadata.get("additional_runtime_option_patches") != []:
        raise ValueError("additional_runtime_option_patches must be empty")
    serialization = metadata.get("serialization")
    if serialization != CANONICAL_JSON_SERIALIZATION:
        raise ValueError("serialization must be canonical UTF-8 without BOM and LF-only")

    parameters = metadata.get("effective_parameters")
    if not isinstance(parameters, dict) or set(parameters) != REQUIRED_EFFECTIVE_PARAMETERS:
        raise ValueError(
            "effective_parameters must contain exactly the capture script's effective parameters"
        )
    for key in REQUIRED_CLI_EFFECTIVE_PARAMETERS:
        value = parameters[key]
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
    behavior_bindings = {
        "schema_id": metadata["schema_id"],
        "modules": metadata["modules"],
        "inputs": metadata["inputs"],
        "page_policy": metadata["page_policy"],
        "runtime_options": metadata["runtime_options"],
        "runtime_options_source": metadata["runtime_options_source"],
        "additional_runtime_option_patches": metadata[
            "additional_runtime_option_patches"
        ],
        "serialization": metadata["serialization"],
    }
    for key, expected in behavior_bindings.items():
        if parameters[key] != expected:
            raise ValueError(f"effective_parameters.{key} does not match capture metadata")
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

    curator_parameters = metadata.get("curator_effective_parameters")
    expected_curator_parameters = {
        "pages": RAW_PATHS["pages"],
        "composition": RAW_PATHS["composition"],
        "metadata": RAW_PATHS["metadata"],
        "output": parameters["output"],
    }
    if curator_parameters != expected_curator_parameters:
        raise ValueError("curator_effective_parameters are not canonical")
    expected_curator_invocation = _expected_curator_invocation(parameters)
    commands = metadata.get("commands")
    expected_commands = {
        "deploy": (
            "rime_deployer.exe --build disposable/user disposable/shared "
            "disposable/user/build"
        ),
        "capture": expected_invocation,
        "curate": expected_curator_invocation,
    }
    if commands != expected_commands:
        raise ValueError("commands do not match the effective capture and curation parameters")

    output_provenance = metadata.get("output_provenance")
    required_output_fields = {
        "path",
        "existed_before_capture",
        "write_policy",
        "generated_by",
        "raw_paths",
    }
    if (
        not isinstance(output_provenance, dict)
        or set(output_provenance) != required_output_fields
    ):
        raise ValueError("output_provenance must bind raw and curated JSON writes")
    if (
        output_provenance["path"] != parameters["output"]
        or output_provenance["existed_before_capture"] is not False
        or output_provenance["write_policy"] != WRITE_POLICY
        or output_provenance["generated_by"]
        != "scripts/curate-m59-luna-composition.py"
        or output_provenance["raw_paths"] != RAW_PATHS
    ):
        raise ValueError("output_provenance does not match the canonical write contract")
    return {
        "dll_hash": dll_hash,
        "deployer_hash": deployer_hash,
        "source_commit": source_commit,
        "dependencies": normalized_dependencies,
        "clean_repositories": clean_repositories,
        "source_git_trees": normalized_git_trees,
        "queried_data": queried_data,
        "tool_source": tool_source,
        "tool_hashes": tool_hashes,
        "inputs": metadata["inputs"],
        "page_sizes": page_sizes,
        "captured_all_pages": True,
        "runtime_options": runtime_options,
        "runtime_options_source": metadata["runtime_options_source"],
        "additional_runtime_option_patches": [],
        "serialization": serialization,
        "commands": commands,
        "curator_parameters": curator_parameters,
        "output_provenance": output_provenance,
        "parameters": parameters,
        "invocation": expected_invocation,
    }


def _write_atomic(path, content):
    output = Path(path)
    if output.exists() or output.is_symlink():
        raise FileExistsError(f"refusing to overwrite existing output: {output}")
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
        os.link(temp_path, output)
        temp_path.unlink()
    except Exception:
        temp_path.unlink(missing_ok=True)
        raise


def _curate(argv=None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if len(args) != 4:
        raise SystemExit(
            "usage: curate-m59-luna-composition.py "
            "<pages.json> <compose.json> <metadata.json> <output.json>"
        )
    pages_path, compose_path, metadata_path, out_path = args
    pages = _load_canonical_json(pages_path, "raw pages JSON")
    compose = _load_canonical_json(compose_path, "raw composition JSON")
    metadata = _load_canonical_json(metadata_path, "raw metadata JSON")
    validated_metadata = validate_metadata(metadata)

    if not isinstance(pages, list) or len(pages) != len(TARGETS):
        raise ValueError(f"oracle capture must contain exactly {len(TARGETS)} cases")
    case_names = [case.get("input") for case in pages if isinstance(case, dict)]
    if len(case_names) != len(pages) or len(set(case_names)) != len(case_names):
        raise ValueError("oracle capture cases must be objects with unique input values")
    if case_names != list(EXPECTED_INPUTS):
        raise ValueError(
            "oracle capture case order mismatch: "
            f"expected={list(EXPECTED_INPUTS)}, actual={case_names}"
        )
    if case_names != validated_metadata["inputs"]:
        raise ValueError("raw case order does not match metadata.inputs")

    inputs = {}
    observed_page_sizes = []
    for case in pages:
        name = case["input"]
        if case.get("schema_id") != "luna_pinyin":
            raise ValueError(f"oracle capture for {name} changed schema")
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
        if case.get("termination_reason") != "last_page" or "pagination_error" in case:
            raise ValueError(
                f"oracle capture for {name} did not terminate cleanly at the last page"
            )
        page_size = case.get("page_size")
        if not isinstance(page_size, int) or isinstance(page_size, bool) or page_size <= 0:
            raise ValueError(f"oracle capture for {name} has invalid page_size")
        if page_size not in observed_page_sizes:
            observed_page_sizes.append(page_size)
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
    if observed_page_sizes != validated_metadata["page_sizes"]:
        raise ValueError(
            "raw page sizes do not match metadata.page_sizes_observed: "
            f"raw={observed_page_sizes}, metadata={validated_metadata['page_sizes']}"
        )

    if not isinstance(compose, list) or not compose:
        raise ValueError("composition capture must be a non-empty array")
    compose_by_scenario = collections.OrderedDict()
    for index, snap in enumerate(compose):
        if not isinstance(snap, dict) or not isinstance(snap.get("scenario"), str):
            raise ValueError(f"composition snapshot {index} has no scenario")
        compose_by_scenario.setdefault(snap["scenario"], []).append(snap)
    expected_scenarios = {row[0] for row in COMPOSITIONS}
    if set(compose_by_scenario) != expected_scenarios:
        raise ValueError(
            "composition capture must contain exactly the declared scenarios: "
            f"expected={sorted(expected_scenarios)}, actual={sorted(compose_by_scenario)}"
        )

    compositions = {}
    for scenario, human_input, note, expected_commit in COMPOSITIONS:
        snaps = compose_by_scenario.get(scenario, [])
        labels = [snap.get("label") for snap in snaps]
        if labels != list(COMPOSITION_LABELS):
            raise ValueError(
                f"composition {scenario} snapshot chain is incomplete or reordered: "
                f"expected={list(COMPOSITION_LABELS)}, actual={labels}"
            )
        expected_preedits = {
            "moboyi_page0": "mo bo yi",
            "after_select_mo": expected_commit[0] + "bo yi",
            "bo_pd1": expected_commit[0] + "bo yi",
            "bo_pd2": expected_commit[0] + "bo yi",
            "bo_pd3": expected_commit[0] + "boyi",
            "after_select_bo": expected_commit[:2] + "yi",
            **{
                f"yi_pd{index}": expected_commit[:2] + "yi"
                for index in range(1, 32)
            },
            "after_select_yi": None,
        }
        expected_page_numbers = {
            "moboyi_page0": 0,
            "after_select_mo": 0,
            **{f"bo_pd{page_no}": page_no for page_no in range(1, 4)},
            "after_select_bo": 0,
            **{f"yi_pd{page_no}": page_no for page_no in range(1, 32)},
            "after_select_yi": 0,
        }
        selection_targets = {
            "moboyi_page0": (2, expected_commit[0]),
            "bo_pd3": (4, expected_commit[1]),
            "yi_pd31": (0, expected_commit[2]),
        }
        for index, snap in enumerate(snaps):
            label = COMPOSITION_LABELS[index]
            final = label == "after_select_yi"
            if snap.get("schema_id") != "luna_pinyin":
                raise ValueError(f"composition {scenario} snapshot {label} changed schema")
            if snap.get("rime_get_input") != ("" if final else human_input):
                raise ValueError(
                    f"composition {scenario} snapshot {label} changed rime_get_input"
                )
            if snap.get("is_composing") is not (not final):
                raise ValueError(
                    f"composition {scenario} snapshot {label} has invalid composing state"
                )
            for option_state in (
                "is_ascii_mode",
                "is_full_shape",
                "is_simplified",
                "is_ascii_punct",
            ):
                if snap.get(option_state) is not False:
                    raise ValueError(
                        f"composition {scenario} snapshot {label} changed {option_state}"
                    )
            if index == 0:
                if "processed" in snap:
                    raise ValueError(
                        f"composition {scenario} initial snapshot must not be a key action"
                    )
            elif not _is_plain_int(snap.get("processed")) or snap["processed"] != 1:
                raise ValueError(
                    f"composition {scenario} snapshot {label} was not processed"
                )
            if (
                not _is_plain_int(snap.get("page_no"))
                or snap["page_no"] != expected_page_numbers[label]
            ):
                raise ValueError(
                    f"composition {scenario} snapshot {label} changed page_no"
                )
            expected_page_size = 0 if final else 5
            if (
                not _is_plain_int(snap.get("page_size"))
                or snap["page_size"] != expected_page_size
            ):
                raise ValueError(
                    f"composition {scenario} snapshot {label} changed page_size"
                )
            if snap.get("is_last_page") is not False:
                raise ValueError(
                    f"composition {scenario} snapshot {label} changed last-page state"
                )
            if (
                not _is_plain_int(snap.get("highlighted_candidate_index"))
                or snap["highlighted_candidate_index"] != 0
            ):
                raise ValueError(
                    f"composition {scenario} snapshot {label} changed highlight"
                )
            selected = snap.get("selected_candidates")
            if not isinstance(selected, list) or len(selected) != expected_page_size:
                raise ValueError(
                    f"composition {scenario} snapshot {label} has an invalid candidate page"
                )
            for local_index, candidate in enumerate(selected):
                if (
                    not isinstance(candidate, dict)
                    or not _is_plain_int(candidate.get("index"))
                    or candidate["index"] != local_index
                    or not isinstance(candidate.get("text"), str)
                ):
                    raise ValueError(
                        f"composition {scenario} snapshot {label} candidate {local_index} "
                        "has invalid local position or text"
                    )
            if label in selection_targets:
                selection_index, selection_text = selection_targets[label]
                if selected[selection_index]["text"] != selection_text:
                    raise ValueError(
                        f"composition {scenario} snapshot {label} does not expose the "
                        "declared selection target"
                    )
            if snap.get("preedit") != expected_preedits[label]:
                raise ValueError(
                    f"composition {scenario} snapshot {label} changed preedit"
                )
            expected_step_commit = expected_commit if final else None
            if snap.get("commit_text") != expected_step_commit:
                raise ValueError(
                    f"composition {scenario} snapshot {label} changed commit_text"
                )
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
            "inputs": validated_metadata["inputs"],
            "input_count": len(validated_metadata["inputs"]),
            "page_sizes_observed": validated_metadata["page_sizes"],
            "captured_all_pages": validated_metadata["captured_all_pages"],
            "actual_invocation": validated_metadata["invocation"],
            "effective_parameters": validated_metadata["parameters"],
            "curator_effective_parameters": validated_metadata["curator_parameters"],
            "commands": validated_metadata["commands"],
            "page_policy": PAGE_POLICY,
            "runtime_options": validated_metadata["runtime_options"],
            "runtime_options_source": validated_metadata["runtime_options_source"],
            "additional_runtime_option_patches": validated_metadata[
                "additional_runtime_option_patches"
            ],
            "serialization": validated_metadata["serialization"],
            "output_provenance": validated_metadata["output_provenance"],
            "tool_source": validated_metadata["tool_source"],
            "tool_hashes": validated_metadata["tool_hashes"],
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
    if output.exists() or output.is_symlink():
        raise FileExistsError(f"refusing to overwrite existing output: {output}")
    return _curate(args)


if __name__ == "__main__":
    raise SystemExit(main())

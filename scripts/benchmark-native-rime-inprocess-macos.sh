#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

output_root=""
librime_source="/Users/laufei/Documents/GitHub/librime"
iterations=9
session_iterations=60
key_iterations=80
track_a_inputs="n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru"
track_b_inputs="neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung"
track_a_storage_mode="production-default"
track_a_storage_mode_explicit=0
skip_track_b=0
deploy_product_before_benchmark=1

oracle_commit="33e78140250125871856cdc5b42ddc6a5fcd3cd4"

schema_repo_spec() {
  case "$1" in
    rime-prelude) echo "https://github.com/rime/rime-prelude.git 082425ea0684bca36474415d4a0e8db9b016487e" ;;
    rime-essay) echo "https://github.com/rime/rime-essay.git 48c7538f0b760fcc8c9d6bf08711f82cfbd2e9ed" ;;
    rime-luna-pinyin) echo "https://github.com/rime/rime-luna-pinyin.git 18a80335c37522311f7cff02886cd81cec3b460a" ;;
    rime-stroke) echo "https://github.com/rime/rime-stroke.git 3a4b0f4013e2b4c14b1e80c92b1d4723eb65f39c" ;;
    *) echo "unknown schema repo: $1" >&2; exit 1 ;;
  esac
}

usage() {
  cat <<'EOF'
Usage: scripts/benchmark-native-rime-inprocess-macos.sh [options]

Options:
  --output-root PATH       External evidence directory to create.
                           Default: ~/.yune/evidence/native-rime-inprocess-macos/<UTC>.
  --librime-source PATH    Existing local rime/librime clone to copy from.
  --iterations N           Startup iterations. Default: 9.
  --session-iterations N   Session lifecycle iterations. Default: 60.
  --key-iterations N       Key workload iterations. Default: 80.
  --track-a-inputs CSV     Track A input list.
  --track-a-storage-mode MODE
                           Track A Yune storage mode: production-default,
                           owned, or byte-backed. Default: production-default.
  --track-b-inputs CSV     Track B input list.
  --skip-track-b           Skip the TypeDuck product guard lane.
  --no-product-deploy      Do not deploy Track B before benchmarking.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output-root)
      output_root="$2"
      shift 2
      ;;
    --librime-source)
      librime_source="$2"
      shift 2
      ;;
    --iterations)
      iterations="$2"
      shift 2
      ;;
    --session-iterations)
      session_iterations="$2"
      shift 2
      ;;
    --key-iterations)
      key_iterations="$2"
      shift 2
      ;;
    --track-a-inputs)
      track_a_inputs="$2"
      shift 2
      ;;
    --track-a-storage-mode)
      track_a_storage_mode="$2"
      track_a_storage_mode_explicit=1
      shift 2
      ;;
    --track-b-inputs)
      track_b_inputs="$2"
      shift 2
      ;;
    --skip-track-b)
      skip_track_b=1
      shift
      ;;
    --no-product-deploy)
      deploy_product_before_benchmark=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

case "$track_a_storage_mode" in
  production-default|owned|byte-backed) ;;
  *)
    echo "invalid --track-a-storage-mode: $track_a_storage_mode (expected production-default, owned, or byte-backed)" >&2
    exit 2
    ;;
esac

inherited_poet_byte_backed_present=0
inherited_poet_byte_backed_value=""
if [[ "${YUNE_POET_BYTE_BACKED+x}" == "x" ]]; then
  inherited_poet_byte_backed_present=1
  inherited_poet_byte_backed_value="$YUNE_POET_BYTE_BACKED"
fi
if [[ "$track_a_storage_mode_explicit" == "1" && "$inherited_poet_byte_backed_present" == "1" ]]; then
  echo "YUNE_POET_BYTE_BACKED must be absent when --track-a-storage-mode is explicit." >&2
  exit 1
fi
if [[ "$track_a_storage_mode_explicit" == "0" && "$inherited_poet_byte_backed_value" == "1" ]]; then
  echo "The signed Track A gate requires default-owned poet measurement; unset YUNE_POET_BYTE_BACKED before running it." >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required" >&2
  exit 1
fi
if ! command -v git >/dev/null 2>&1; then
  echo "git is required" >&2
  exit 1
fi

evidence_path_tool="$repo_root/scripts/evidence-output-path.py"
if [[ -z "$output_root" ]]; then
  output_root="$(
    python3 "$evidence_path_tool" default \
      --repo-root "$repo_root" \
      --kind native-rime-inprocess-macos
  )"
fi
output_root="$(
  python3 "$evidence_path_tool" validate \
    --repo-root "$repo_root" \
    --path "$output_root"
)"

abs_path() {
  python3 -c 'import os,sys; print(os.path.abspath(sys.argv[1]))' "$1"
}

clear_dir_under() {
  local root path resolved_root resolved_path
  root="$(abs_path "$1")"
  path="$(abs_path "$2")"
  resolved_root="${root%/}"
  resolved_path="${path%/}"
  case "$resolved_path" in
    "$resolved_root"/*) ;;
    *)
      echo "refusing to clear directory outside $resolved_root: $resolved_path" >&2
      exit 1
      ;;
  esac
  rm -rf "$resolved_path"
  mkdir -p "$resolved_path"
}

copy_dir_contents() {
  local source="$1"
  local destination="$2"
  mkdir -p "$destination"
  cp -R "$source"/. "$destination"/
}

require_path() {
  local path="$1"
  local label="$2"
  if [[ ! -e "$path" ]]; then
    echo "missing $label: $path" >&2
    exit 1
  fi
}

log() {
  printf '[macos-bench] %s\n' "$*"
}

die() {
  echo "$*" >&2
  exit 1
}

assert_poet_byte_backed_restored() {
  local phase="$1"
  if [[ "$inherited_poet_byte_backed_present" == "1" ]]; then
    if [[ "${YUNE_POET_BYTE_BACKED+x}" != "x" || "$YUNE_POET_BYTE_BACKED" != "$inherited_poet_byte_backed_value" ]]; then
      die "YUNE_POET_BYTE_BACKED was not restored to its inherited value $phase."
    fi
  elif [[ "${YUNE_POET_BYTE_BACKED+x}" == "x" ]]; then
    die "YUNE_POET_BYTE_BACKED must be absent $phase."
  fi
}

sha256_file() {
  python3 - "$1" <<'PY'
import hashlib
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
digest = hashlib.sha256()
with path.open("rb") as handle:
    for chunk in iter(lambda: handle.read(1024 * 1024), b""):
        digest.update(chunk)
print(digest.hexdigest())
PY
}

yune_git_head="$(git -C "$repo_root" rev-parse HEAD)"
yune_git_tree="$(git -C "$repo_root" rev-parse 'HEAD^{tree}')"
yune_git_status_at_start="$(git -C "$repo_root" status --porcelain=v1 --untracked-files=all)"
if [[ -n "$yune_git_status_at_start" ]]; then
  die "macOS source-bound measurement requires a clean Yune checkout."
fi

librime_source="$(abs_path "$librime_source")"
work_base="$repo_root/target/macos-performance-verification"
tool_root="$work_base/tools"
oracle_root="$work_base/oracle-1.17.0"
librime_src="$work_base/librime-src"
run_work_root="$work_base/native-inprocess/$(basename "$output_root")"
schema_root="$oracle_root/schema-src"
shared_source="$oracle_root/rime-shared"
user_source="$oracle_root/rime-user"
build_source="$user_source/build"

if [[ -e "$output_root" ]]; then
  echo "output root must be a new external path; refusing to clear or reuse: $output_root" >&2
  exit 1
fi
mkdir -p "$output_root"
clear_dir_under "$work_base/native-inprocess" "$run_work_root"
mkdir -p "$tool_root" "$oracle_root" "$schema_root"

if [[ -x "$tool_root/python/bin/cmake" ]]; then
  export PATH="$tool_root/python/bin:$PATH"
  export PYTHONPATH="$tool_root/python${PYTHONPATH:+:$PYTHONPATH}"
fi
if ! command -v cmake >/dev/null 2>&1; then
  log "cmake not found; installing cmake into $tool_root/python"
  python3 -m pip install --upgrade --target "$tool_root/python" cmake ninja
  export PATH="$tool_root/python/bin:$PATH"
  export PYTHONPATH="$tool_root/python${PYTHONPATH:+:$PYTHONPATH}"
fi
mkdir -p "$tool_root/python/bin"
python3_path="$(command -v python3)"
rm -f "$tool_root/python/bin/python"
printf '#!/usr/bin/env bash\nexec "%s" "$@"\n' "$python3_path" > "$tool_root/python/bin/python"
chmod +x "$tool_root/python/bin/python"
command -v cmake >/dev/null 2>&1 || { echo "cmake is still unavailable" >&2; exit 1; }

run_with_local_tools() {
  PATH="$tool_root/python/bin:$PATH" \
    PYTHON="$tool_root/python/bin/python" \
    PYTHON_EXECUTABLE="$tool_root/python/bin/python" \
    "$@"
}

prepare_librime_source() {
  if [[ ! -d "$librime_src/.git" ]]; then
    log "cloning isolated librime source from $librime_source"
    git clone --recursive "$librime_source" "$librime_src"
  fi
  git -C "$librime_src" fetch --tags "$librime_source"
  git -C "$librime_src" checkout --detach "$oracle_commit"
  git -C "$librime_src" reset --hard "$oracle_commit"
  git -C "$librime_src" submodule update --init --recursive
}

find_boost_root() {
  find "$librime_src/deps" -maxdepth 1 -type d -name 'boost-*' | sort | tail -n 1
}

build_librime() {
  prepare_librime_source
  if [[ -z "$(find_boost_root)" ]]; then
    log "downloading Boost headers into isolated librime source"
    (cd "$librime_src" && bash install-boost.sh --download)
  fi
  local boost_root
  boost_root="$(find_boost_root)"
  require_path "$boost_root" "isolated Boost root"
  log "building librime dependencies"
  (cd "$librime_src" && run_with_local_tools env BOOST_ROOT="$boost_root" NOPARALLEL=1 make deps)
  log "building librime $oracle_commit"
  (cd "$librime_src" && run_with_local_tools env BOOST_ROOT="$boost_root" NOPARALLEL=1 make)
}

find_librime_dylib() {
  find "$librime_src/build" "$librime_src/lib" -type f -name 'librime*.dylib' 2>/dev/null | sort | head -n 1
}

find_rime_deployer() {
  find "$librime_src/build" "$librime_src/bin" -type f -name 'rime_deployer' 2>/dev/null | sort | head -n 1
}

prepare_schema_repo() {
  local name="$1"
  local spec url commit destination
  spec="$(schema_repo_spec "$name")"
  url="${spec%% *}"
  commit="${spec##* }"
  destination="$schema_root/$name"
  if [[ ! -d "$destination/.git" ]]; then
    log "cloning $name"
    git clone "$url" "$destination"
  fi
  git -C "$destination" fetch origin
  git -C "$destination" checkout --detach "$commit"
  git -C "$destination" reset --hard "$commit"
}

prepare_oracle_schema_bundle() {
  for repo in rime-prelude rime-essay rime-luna-pinyin rime-stroke; do
    prepare_schema_repo "$repo"
  done
  clear_dir_under "$oracle_root" "$shared_source"
  clear_dir_under "$oracle_root" "$user_source"
  for repo in rime-prelude rime-essay rime-luna-pinyin rime-stroke; do
    find "$schema_root/$repo" -maxdepth 1 -type f \( -name '*.yaml' -o -name 'essay.txt' \) \
      -exec cp '{}' "$shared_source"/ \;
  done
  local opencc_source=""
  for candidate in "$librime_src/share/opencc" "$librime_src/dist/share/opencc"; do
    if [[ -d "$candidate" ]]; then
      opencc_source="$candidate"
      break
    fi
  done
  if [[ -n "$opencc_source" ]]; then
    mkdir -p "$shared_source/opencc"
    copy_dir_contents "$opencc_source" "$shared_source/opencc"
  fi
  cat > "$shared_source/default.custom.yaml" <<'EOF'
patch:
  schema_list:
    - schema: luna_pinyin
EOF
  mkdir -p "$build_source"
}

write_identity() {
  {
    echo "date_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "repo_root=$repo_root"
    echo "yune_git_head=$yune_git_head"
    echo "yune_git_tree=$yune_git_tree"
    echo "yune_git_status_short="
    echo "source_clean=true"
    echo "librime_source=$librime_source"
    echo "isolated_librime_source=$librime_src"
    echo "librime_git_head=$librime_git_head"
    echo "librime_git_tree=$librime_git_tree"
    echo "librime_git_status_short="
    echo "librime_source_clean=true"
    echo "librime_target_commit=$oracle_commit"
    echo "oracle_root=$oracle_root"
    echo "transient_work_root=$run_work_root"
    echo "platform=$(uname -a)"
    sw_vers 2>/dev/null | sed 's/:[[:space:]]*/=/' || true
    echo "cmake=$(command -v cmake)"
    echo "track_a_inputs=$track_a_inputs"
    echo "track_a_storage_mode=$track_a_storage_mode"
    echo "track_a_storage_mode_explicit=$track_a_storage_mode_explicit"
    echo "inherited_poet_byte_backed_present=$inherited_poet_byte_backed_present"
    echo "track_b_inputs=$track_b_inputs"
    echo "iterations=$iterations"
    echo "session_iterations=$session_iterations"
    echo "key_iterations=$key_iterations"
    echo "skip_track_b=$skip_track_b"
    echo "deploy_product_before_benchmark=$deploy_product_before_benchmark"
  } > "$output_root/environment.txt"
}

run_cargo_bench() {
  local output_name="$1"
  local engine_name="$2"
  local track="$3"
  local schema="$4"
  local dylib="$5"
  local shared="$6"
  local user="$7"
  local build="$8"
  local inputs="$9"
  local extra_dyld="${10}"
  local deploy_before="${11}"
  local deploy_only="${12:-0}"
  local poet_byte_backed="${13:-0}"
  local engine_output="$output_root/$output_name"
  case "$poet_byte_backed" in
    0|1) ;;
    *) die "invalid scoped YUNE_POET_BYTE_BACKED selector for $output_name: $poet_byte_backed" ;;
  esac
  assert_poet_byte_backed_restored "before $output_name"
  clear_dir_under "$output_root" "$engine_output"
  local args=(
    bench -p yune-rime-api --bench native_inprocess_benchmark --
    --engine "$engine_name"
    --track "$track"
    --schema "$schema"
    --dll "$dylib"
    --shared "$shared"
    --user "$user"
    --build "$build"
    --output "$engine_output"
    --inputs "$inputs"
    --track-a-storage-mode "$track_a_storage_mode"
    --iterations "$iterations"
    --session-iterations "$session_iterations"
    --key-iterations "$key_iterations"
  )
  if [[ "$deploy_before" == "1" ]]; then
    args+=(--deploy-before-benchmark)
  fi
  if [[ "$deploy_only" == "1" ]]; then
    args+=(--deploy-only)
  fi
  {
    if [[ "$poet_byte_backed" == "1" ]]; then
      printf 'YUNE_POET_BYTE_BACKED=1 '
    fi
    printf 'cargo'
    printf ' %q' "${args[@]}"
    printf '\n'
  } >> "$output_root/commands.txt"
  log "running $output_name"
  local status=0
  if [[ "$poet_byte_backed" == "1" ]]; then
    if (
      cd "$repo_root"
      YUNE_POET_BYTE_BACKED=1 \
        DYLD_LIBRARY_PATH="$extra_dyld${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}" \
        cargo "${args[@]}"
    ) 2>&1 | tee "$engine_output/cargo-bench-native-inprocess.log"; then
      status=0
    else
      status=$?
    fi
  else
    if (
      cd "$repo_root"
      DYLD_LIBRARY_PATH="$extra_dyld${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}" \
        cargo "${args[@]}"
    ) 2>&1 | tee "$engine_output/cargo-bench-native-inprocess.log"; then
      status=0
    else
      status=$?
    fi
  fi
  assert_poet_byte_backed_restored "after $output_name"
  return "$status"
}

prepare_upstream_run() {
  local engine_name="$1"
  local dylib="$2"
  local run_root="$run_work_root/$engine_name"
  clear_dir_under "$run_work_root" "$run_root"
  cp "$dylib" "$run_root/$(basename "$dylib")"
  copy_dir_contents "$shared_source" "$run_root/shared"
  mkdir -p "$run_root/user"
  copy_dir_contents "$build_source" "$run_root/user/build"
  printf '%s\n' "$run_root"
}

prepare_product_run() {
  local engine_name="$1"
  local dylib="$2"
  local product_schema_root="$repo_root/apps/yune-web/public/schema"
  local run_root="$run_work_root/$engine_name"
  clear_dir_under "$run_work_root" "$run_root"
  cp "$dylib" "$run_root/$(basename "$dylib")"
  copy_dir_contents "$product_schema_root" "$run_root/shared"
  mkdir -p "$run_root/user"
  copy_dir_contents "$product_schema_root/build" "$run_root/user/build"
  printf '%s\n' "$run_root"
}

combine_outputs() {
  python3 - "$output_root" <<'PY'
import csv
import os
import sys
from collections import defaultdict
from pathlib import Path

root = Path(sys.argv[1])
files = [
    "summary.csv",
    "samples.csv",
    "m37_metrics.csv",
    "product_path_status.csv",
    "startup_session_trace.csv",
    "candidate_snapshots.csv",
    "raw_lookup_microbench.csv",
    "memory-owner-profile.csv",
]

def read_csv(path):
    with path.open(newline="", encoding="utf-8-sig") as handle:
        return list(csv.DictReader(handle))

def write_csv(path, rows, fieldnames=None):
    if fieldnames is None:
        keys = []
        for row in rows:
            for key in row:
                if key not in keys:
                    keys.append(key)
        fieldnames = keys
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows:
            writer.writerow({key: row.get(key, "") for key in fieldnames})

combined = {}
for name in files:
    rows = []
    fieldnames = None
    for path in sorted(root.glob(f"*/{name}")):
        with path.open(newline="", encoding="utf-8-sig") as handle:
            reader = csv.DictReader(handle)
            if fieldnames is None and reader.fieldnames:
                fieldnames = reader.fieldnames
            rows.extend(reader)
    combined[name] = rows
    if fieldnames is not None:
        write_csv(root / name, rows, fieldnames)

summary = combined["summary.csv"]
yune_rows = {}
librime_rows = {}
for row in summary:
    if row.get("track") != "track-a-comparison" or row.get("schema_id") != "luna_pinyin":
        continue
    key = (row.get("workload", ""), row.get("input", ""))
    if row.get("engine") == "yune":
        yune_rows[key] = row
    elif row.get("engine") == "librime-1.17.0":
        librime_rows[key] = row

comparison = []
for key in sorted(yune_rows):
    if key not in librime_rows:
        continue
    yune = yune_rows[key]
    librime = librime_rows[key]
    yune_median = float(yune["median_us"])
    librime_median = float(librime["median_us"])
    ratio = yune_median / librime_median if librime_median else float("inf")
    comparison.append({
        "track": yune["track"],
        "schema_id": yune["schema_id"],
        "workload": key[0],
        "input": key[1],
        "yune_median_us": f"{yune_median:.3f}",
        "librime_median_us": f"{librime_median:.3f}",
        "yune_librime_median_ratio": f"{ratio:.3f}",
        "absolute_gap_us": f"{yune_median - librime_median:.3f}",
        "yune_max_peak_working_set_bytes": yune.get("max_peak_working_set_bytes", ""),
        "librime_max_peak_working_set_bytes": librime.get("max_peak_working_set_bytes", ""),
        "yune_median_private_bytes": yune.get("median_private_bytes", ""),
        "librime_median_private_bytes": librime.get("median_private_bytes", ""),
    })
write_csv(root / "summary-comparison.csv", comparison, [
    "track", "schema_id", "workload", "input", "yune_median_us",
    "librime_median_us", "yune_librime_median_ratio", "absolute_gap_us",
    "yune_max_peak_working_set_bytes", "librime_max_peak_working_set_bytes",
    "yune_median_private_bytes", "librime_median_private_bytes",
])

expected_slower = {
    "n",
    "ni",
    "hao",
    "ceshiyixiachangjushuruxingnengzenyang",
    "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong",
}
expected_faster = {"zhongguo", "cszysmsrsd", "zybfshmsru"}
claim_rows = []
for row in comparison:
    if row["workload"] != "key_sequence_process_with_context":
        continue
    ratio = float(row["yune_librime_median_ratio"])
    input_value = row["input"]
    if input_value in expected_slower:
        status = "confirmed" if ratio > 1.0 else "contradicted"
        expected = "yune slower than librime"
    elif input_value in expected_faster:
        status = "confirmed" if ratio < 1.0 else "contradicted"
        expected = "yune faster than librime"
    else:
        status = "not-classified"
        expected = ""
    claim_rows.append({
        "kind": "latency_direction",
        "workload": row["workload"],
        "input": input_value,
        "observed_ratio": row["yune_librime_median_ratio"],
        "expected": expected,
        "status": status,
    })

for workload in ["startup_warm_shared_assets_runtime_ready", "session_create_select_destroy"]:
    match = next((row for row in comparison if row["workload"] == workload), None)
    if not match:
        continue
    ratio = float(match["yune_librime_median_ratio"])
    claim_rows.append({
        "kind": "latency_noise_caveat",
        "workload": workload,
        "input": "",
        "observed_ratio": match["yune_librime_median_ratio"],
        "expected": "report says measured faster on Windows but run-noisy/platform-specific",
        "status": "confirmed-on-this-run" if ratio < 1.0 else "platform-specific",
    })

snapshots = combined["candidate_snapshots.csv"]
pages = defaultdict(list)
for row in snapshots:
    if row.get("track") == "track-a-comparison" and row.get("schema_id") == "luna_pinyin":
        key = (row.get("engine"), row.get("input"))
        pages[key].append((int(row.get("candidate_index") or 0), row.get("text", ""), row.get("comment", "")))
for key in list(pages):
    pages[key].sort()
same_expected = {"ni", "hao", "cszysmsrsd", "zybfshmsru"}
diff_allowed = expected_slower | {"zhongguo"}
for input_value in sorted(same_expected | diff_allowed):
    # The reports classify first-page candidate text/ranking. Keep comments in
    # the raw snapshots, but do not fail the disclosure check only because one
    # engine supplies pinyin comments and the other leaves comments empty.
    yune_page = [text for _, text, _ in pages.get(("yune", input_value), [])]
    librime_page = [text for _, text, _ in pages.get(("librime-1.17.0", input_value), [])]
    same = yune_page == librime_page
    if input_value in same_expected:
        status = "confirmed" if same else "contradicted"
        expected = "first page matches"
    else:
        status = "confirmed" if not same else "allowed-but-unexpected-match"
        expected = "first page may differ as disclosed"
    claim_rows.append({
        "kind": "candidate_snapshot",
        "workload": "candidate_first_page",
        "input": input_value,
        "observed_ratio": "",
        "expected": expected,
        "status": status,
    })

track_a_key_rows = [row for row in comparison if row["workload"] == "key_sequence_process_with_context"]
memory_status = "not-available"
if track_a_key_rows:
    yune_peak = max((int(row["yune_max_peak_working_set_bytes"]) for row in track_a_key_rows if row["yune_max_peak_working_set_bytes"].isdigit()), default=0)
    librime_peak = max((int(row["librime_max_peak_working_set_bytes"]) for row in track_a_key_rows if row["librime_max_peak_working_set_bytes"].isdigit()), default=0)
    if yune_peak and librime_peak:
        memory_status = "confirmed" if yune_peak > librime_peak else "contradicted"
    claim_rows.append({
        "kind": "memory_direction",
        "workload": "track-a-peak-resident",
        "input": "",
        "observed_ratio": f"{(yune_peak / librime_peak):.3f}" if yune_peak and librime_peak else "",
        "expected": "Yune Track A peak resident memory remains above librime on native peer lane",
        "status": memory_status,
    })

claim_rows.append({
    "kind": "browser",
    "workload": "browser-peer-dashboard",
    "input": "",
    "observed_ratio": "",
    "expected": "browser rows are carried evidence",
    "status": "not-rerun",
})
write_csv(root / "claim-shape-check.csv", claim_rows, [
    "kind", "workload", "input", "observed_ratio", "expected", "status"
])

def md_table(headers, rows):
    lines = ["| " + " | ".join(headers) + " |", "| " + " | ".join(["---"] * len(headers)) + " |"]
    for row in rows:
        lines.append("| " + " | ".join(row) + " |")
    return "\n".join(lines)

verdict = [
    "# macOS Native Performance Verification",
    "",
    "This run uses the Rust `native_inprocess_benchmark` harness on macOS and reads context after every keypress.",
    "Absolute timings and memory are platform-specific; the verdict checks the report's directional claim shape.",
    "",
    "## Track A Latency",
    "",
    md_table(
        ["Input/workload", "Yune us", "librime us", "ratio", "status"],
        [[
            row["input"] or row["workload"],
            row["yune_median_us"],
            row["librime_median_us"],
            row["yune_librime_median_ratio"],
            next((claim["status"] for claim in claim_rows if claim["workload"] == row["workload"] and claim["input"] == row["input"]), "info"),
        ] for row in comparison]
    ),
    "",
    "## Claim Shape",
    "",
    md_table(
        ["Kind", "Input", "Expected", "Status"],
        [[row["kind"], row["input"] or row["workload"], row["expected"], row["status"]] for row in claim_rows]
    ),
    "",
    "## Notes",
    "",
    "- macOS memory sampling uses resident size from `proc_pidinfo(PROC_PIDTASKINFO)` and peak resident size from `getrusage(RUSAGE_SELF).ru_maxrss`; Windows private/pagefile counters are not available on this platform.",
    "- Browser rows were not re-run by this native verification lane.",
]
(root / "macos-verdict.md").write_text("\n".join(verdict) + "\n", encoding="utf-8")
PY
}

write_readme() {
  cat > "$output_root/README.md" <<EOF
# macOS Native In-Process Benchmark

This run uses the Rust \`native_inprocess_benchmark\` bench and loads Yune plus
a locally built upstream librime 1.17.0 dylib in process. It mirrors the
corrective Windows benchmark shape: every keypress is followed by
\`get_context/free_context\`.

- Track A: \`luna_pinyin\`, Yune versus upstream librime 1.17.0.
- Track A storage mode: \`$track_a_storage_mode\`.
- Track B: $([[ "$skip_track_b" == "1" ]] && echo "skipped." || echo "Yune TypeDuck jyut6ping3_mobile product guard.")
- Claim verdict: \`macos-verdict.md\`.
- Summary comparison: \`summary-comparison.csv\`.
- Claim-shape check: \`claim-shape-check.csv\`.
EOF
}

build_librime
librime_git_head="$(git -C "$librime_src" rev-parse HEAD)"
librime_git_tree="$(git -C "$librime_src" rev-parse 'HEAD^{tree}')"
librime_git_status_at_build="$(git -C "$librime_src" status --porcelain=v1 --untracked-files=all)"
if [[ "$librime_git_head" != "$oracle_commit" ]]; then
  die "isolated librime source mismatch: expected $oracle_commit, got $librime_git_head"
fi
if [[ -n "$librime_git_status_at_build" ]]; then
  die "macOS source-bound measurement requires a clean isolated librime checkout."
fi
librime_dylib="$(find_librime_dylib)"
rime_deployer="$(find_rime_deployer)"
require_path "$librime_dylib" "librime dylib"
require_path "$rime_deployer" "rime_deployer"

prepare_oracle_schema_bundle
librime_dyld="$(dirname "$librime_dylib"):$librime_src/lib:$librime_src/bin"
log "building luna_pinyin oracle data with rime_deployer"
DYLD_LIBRARY_PATH="$librime_dyld${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}" \
  "$rime_deployer" --build "$user_source" "$shared_source" "$build_source"
require_path "$build_source/luna_pinyin.schema.yaml" "built luna_pinyin schema"
require_path "$build_source/luna_pinyin.table.bin" "built luna_pinyin table"
require_path "$build_source/luna_pinyin.prism.bin" "built luna_pinyin prism"

write_identity

{
  echo "scripts/benchmark-native-rime-inprocess-macos.sh --output-root $output_root --librime-source $librime_source --iterations $iterations --session-iterations $session_iterations --key-iterations $key_iterations --track-a-inputs $track_a_inputs --track-a-storage-mode $track_a_storage_mode --track-b-inputs $track_b_inputs"
  echo "cargo build --release -p yune-rime-api"
  echo "DYLD_LIBRARY_PATH=$librime_dyld $rime_deployer --build $user_source $shared_source $build_source"
} > "$output_root/commands.txt"

log "building Yune release dylib"
(cd "$repo_root" && cargo build --release -p yune-rime-api) 2>&1 | tee "$output_root/cargo-build-release-yune-rime-api.log"
yune_dylib="$repo_root/target/release/libyune_rime_api.dylib"
require_path "$yune_dylib" "Yune release dylib"

track_a_yune_run="$(prepare_upstream_run track-a-yune "$yune_dylib")"
track_a_librime_run="$(prepare_upstream_run track-a-librime-1.17.0 "$librime_dylib")"
if [[ "$skip_track_b" == "0" ]]; then
  track_b_product_run="$(prepare_product_run track-b-yune-product "$yune_dylib")"
fi

track_a_yune_dylib="$track_a_yune_run/$(basename "$yune_dylib")"
track_a_librime_dylib="$track_a_librime_run/$(basename "$librime_dylib")"
source_yune_dylib_sha256="$(sha256_file "$yune_dylib")"
source_librime_dylib_sha256="$(sha256_file "$librime_dylib")"
track_a_yune_dylib_sha256="$(sha256_file "$track_a_yune_dylib")"
track_a_librime_dylib_sha256="$(sha256_file "$track_a_librime_dylib")"
if [[ "$track_a_yune_dylib_sha256" != "$source_yune_dylib_sha256" ]]; then
  die "Track A Yune dylib copy hash does not match the built source dylib."
fi
if [[ "$track_a_librime_dylib_sha256" != "$source_librime_dylib_sha256" ]]; then
  die "Track A librime dylib copy hash does not match the pinned source dylib."
fi
track_b_yune_dylib_sha256="skipped"
if [[ "$skip_track_b" == "0" ]]; then
  track_b_yune_dylib="$track_b_product_run/$(basename "$yune_dylib")"
  track_b_yune_dylib_sha256="$(sha256_file "$track_b_yune_dylib")"
  if [[ "$track_b_yune_dylib_sha256" != "$source_yune_dylib_sha256" ]]; then
    die "Track B Yune dylib copy hash does not match the built source dylib."
  fi
fi
{
  echo "source_commit=$yune_git_head"
  echo "source_tree=$yune_git_tree"
  echo "source_clean_at_start=true"
  echo "librime_commit=$librime_git_head"
  echo "librime_tree=$librime_git_tree"
  echo "librime_source_clean_at_build=true"
  echo "benchmark_script_sha256=$(sha256_file "${BASH_SOURCE[0]}")"
  echo "source_yune_dylib_sha256=$source_yune_dylib_sha256"
  echo "measured_yune_dylib_sha256=$track_a_yune_dylib_sha256"
  echo "track_a_yune_dylib_sha256=$track_a_yune_dylib_sha256"
  echo "source_librime_dylib_sha256=$source_librime_dylib_sha256"
  echo "upstream_librime_dylib_sha256=$track_a_librime_dylib_sha256"
  echo "track_a_librime_dylib_sha256=$track_a_librime_dylib_sha256"
  echo "track_b_yune_dylib_sha256=$track_b_yune_dylib_sha256"
  echo "track_a_storage_mode=$track_a_storage_mode"
  echo "track_a_storage_mode_explicit=$track_a_storage_mode_explicit"
} > "$output_root/external-provenance.txt"
track_a_original_build="$run_work_root/track-a-yune-original-build"
track_a_generated_poet="$run_work_root/track-a-yune-luna_pinyin.poet.bin"
clear_dir_under "$run_work_root" "$track_a_original_build"
copy_dir_contents "$track_a_yune_run/user/build" "$track_a_original_build"
run_cargo_bench "track-a-yune-deploy-prep" "yune" "track-a-comparison" "luna_pinyin" \
  "$track_a_yune_dylib" "$track_a_yune_run/shared" "$track_a_yune_run/user" "$track_a_yune_run/user/build" \
  "deploy-prep" "$(dirname "$track_a_yune_dylib")" 1 1 1
require_path "$track_a_yune_run/user/build/luna_pinyin.poet.bin" "Track A Yune poet artifact"
cp "$track_a_yune_run/user/build/luna_pinyin.poet.bin" "$track_a_generated_poet"
clear_dir_under "$run_work_root" "$track_a_yune_run/user/build"
copy_dir_contents "$track_a_original_build" "$track_a_yune_run/user/build"
cp "$track_a_generated_poet" "$track_a_yune_run/user/build/luna_pinyin.poet.bin"
{
  echo "track_a_deploy_prep=separate_process"
  echo "track_a_storage_mode=$track_a_storage_mode"
  echo "poet_generation_environment=YUNE_POET_BYTE_BACKED=1 (deploy-prep invocation only)"
  if [[ "$track_a_storage_mode" == "byte-backed" ]]; then
    echo "benchmark_poet_environment=YUNE_POET_BYTE_BACKED=1 (Track A Yune timing only)"
  elif [[ "$inherited_poet_byte_backed_present" == "1" ]]; then
    echo "benchmark_poet_environment=inherited non-byte-backed value preserved"
  else
    echo "benchmark_poet_environment=absent"
  fi
  echo "restored_oracle_build_artifacts=true"
  echo "poet_artifact=$track_a_yune_run/user/build/luna_pinyin.poet.bin"
  stat -f "poet_bytes=%z" "$track_a_yune_run/user/build/luna_pinyin.poet.bin"
  echo "table_artifact=$track_a_yune_run/user/build/luna_pinyin.table.bin"
  stat -f "table_bytes=%z" "$track_a_yune_run/user/build/luna_pinyin.table.bin"
} > "$output_root/track-a-yune-deploy-prep-artifacts.txt"

if [[ "$track_a_storage_mode" == "byte-backed" ]]; then
  run_cargo_bench "track-a-yune" "yune" "track-a-comparison" "luna_pinyin" \
    "$track_a_yune_dylib" "$track_a_yune_run/shared" "$track_a_yune_run/user" "$track_a_yune_run/user/build" \
    "$track_a_inputs" "$(dirname "$track_a_yune_dylib")" 0 0 1
else
  run_cargo_bench "track-a-yune" "yune" "track-a-comparison" "luna_pinyin" \
    "$track_a_yune_dylib" "$track_a_yune_run/shared" "$track_a_yune_run/user" "$track_a_yune_run/user/build" \
    "$track_a_inputs" "$(dirname "$track_a_yune_dylib")" 0 0 0
fi
assert_poet_byte_backed_restored "before Track A librime timing"
run_cargo_bench "track-a-librime-1.17.0" "librime-1.17.0" "track-a-comparison" "luna_pinyin" \
  "$track_a_librime_dylib" "$track_a_librime_run/shared" "$track_a_librime_run/user" "$track_a_librime_run/user/build" \
  "$track_a_inputs" "$(dirname "$track_a_librime_dylib"):$librime_dyld" 0 0 0
if [[ "$skip_track_b" == "0" ]]; then
  track_b_yune_dylib="$track_b_product_run/$(basename "$yune_dylib")"
  assert_poet_byte_backed_restored "before Track B timing"
  run_cargo_bench "track-b-yune-product" "yune" "track-b-product" "jyut6ping3_mobile" \
    "$track_b_yune_dylib" "$track_b_product_run/shared" "$track_b_product_run/user" "$track_b_product_run/user/build" \
    "$track_b_inputs" "$(dirname "$track_b_yune_dylib")" "$deploy_product_before_benchmark" 0 0
fi

combine_outputs
python3 - "$output_root/memory-owner-profile.csv" "$track_a_storage_mode" <<'PY'
import csv
import pathlib
import sys
from collections import Counter

path = pathlib.Path(sys.argv[1])
track_a_storage_mode = sys.argv[2]
if not path.is_file():
    raise SystemExit(f"missing Track A memory-owner evidence: {path}")
with path.open(encoding="utf-8-sig", newline="") as handle:
    reader = csv.DictReader(handle)
    required = {
        "engine",
        "track",
        "schema_id",
        "owner_id",
        "mapping_mode",
        "track_a_storage_mode",
    }
    missing = sorted(required.difference(reader.fieldnames or ()))
    if missing:
        raise SystemExit(f"memory-owner evidence is missing columns {missing}: {path}")
    rows = [
        row
        for row in reader
        if row["engine"] == "yune"
        and row["track"] == "track-a-comparison"
        and row["schema_id"] == "luna_pinyin"
    ]
if not rows:
    raise SystemExit(f"memory-owner evidence has no Yune Track A luna_pinyin rows: {path}")
wrong_modes = sorted(
    {
        row["track_a_storage_mode"]
        for row in rows
        if row["track_a_storage_mode"] != track_a_storage_mode
    }
)
if wrong_modes:
    raise SystemExit(
        "Track A memory-owner evidence has inconsistent storage-mode provenance: "
        f"expected {track_a_storage_mode!r}, found {wrong_modes!r}"
    )
poet_rows = [row for row in rows if row["owner_id"].startswith("poet.")]
if track_a_storage_mode in {"production-default", "owned"}:
    expected_owner_ids = {
        "poet.entries_by_code",
        "poet.lookup_index",
        "poet.vocabulary",
        "poet.abbreviation_vocabulary",
    }
    owner_counts = Counter(row["owner_id"] for row in poet_rows)
    expected_counts = Counter({owner_id: 1 for owner_id in expected_owner_ids})
    if owner_counts != expected_counts:
        raise SystemExit(
            f"{track_a_storage_mode} Track A requires exactly one row for each "
            "owned poet owner: "
            f"expected {dict(sorted(expected_counts.items()))!r}, "
            f"found {dict(sorted(owner_counts.items()))!r}"
        )
    byte_backed = [row for row in rows if row["mapping_mode"].startswith("poet_bin:")]
    if byte_backed:
        raise SystemExit(
            f"{track_a_storage_mode} Track A requires owned poet storage, "
            f"but {len(byte_backed)} memory-owner rows report poet_bin storage"
        )
elif track_a_storage_mode == "byte-backed":
    expected_owner_ids = {
        "poet.entries_by_code",
        "poet.prefix_index",
        "poet.vocabulary",
        "poet.abbreviation_vocabulary",
    }
    owner_counts = Counter(row["owner_id"] for row in poet_rows)
    expected_counts = Counter({owner_id: 1 for owner_id in expected_owner_ids})
    if owner_counts != expected_counts:
        raise SystemExit(
            "byte-backed Track A requires exactly one row for each expected poet owner: "
            f"expected {dict(sorted(expected_counts.items()))!r}, "
            f"found {dict(sorted(owner_counts.items()))!r}"
        )
    invalid_mapping_modes = sorted(
        {
            (row["owner_id"], row["mapping_mode"])
            for row in poet_rows
            if row["mapping_mode"] != "poet_bin:byte_backed:mmap"
        }
    )
    if invalid_mapping_modes:
        raise SystemExit(
            "byte-backed Track A poet owners must use "
            "mapping_mode=poet_bin:byte_backed:mmap; "
            f"found {invalid_mapping_modes!r}"
        )
    if any(row["owner_id"] == "poet.lookup_index" for row in rows):
        raise SystemExit("byte-backed Track A must not report poet.lookup_index")
PY
if [[ "$track_a_storage_mode" == "byte-backed" ]]; then
  asserted_poet_owner_shape="poet.abbreviation_vocabulary,poet.entries_by_code,poet.prefix_index,poet.vocabulary; mapping_mode=poet_bin:byte_backed:mmap; poet.lookup_index absent"
else
  asserted_poet_owner_shape="poet.abbreviation_vocabulary,poet.entries_by_code,poet.lookup_index,poet.vocabulary; mapping_mode does not begin poet_bin:"
fi
{
  echo "track_a_storage_mode=$track_a_storage_mode"
  echo "asserted_poet_owner_shape=$asserted_poet_owner_shape"
  echo "status=pass"
} > "$output_root/track-a-owner-shape.txt"
yune_git_head_after="$(git -C "$repo_root" rev-parse HEAD)"
yune_git_tree_after="$(git -C "$repo_root" rev-parse 'HEAD^{tree}')"
yune_git_status_after="$(git -C "$repo_root" status --porcelain=v1 --untracked-files=all)"
if [[ "$yune_git_head_after" != "$yune_git_head" || "$yune_git_tree_after" != "$yune_git_tree" || -n "$yune_git_status_after" ]]; then
  die "Yune source commit, tree, or clean status changed during macOS measurement."
fi
librime_git_head_after="$(git -C "$librime_src" rev-parse HEAD)"
librime_git_tree_after="$(git -C "$librime_src" rev-parse 'HEAD^{tree}')"
librime_git_status_after="$(git -C "$librime_src" status --porcelain=v1 --untracked-files=all)"
if [[ "$librime_git_head_after" != "$librime_git_head" || "$librime_git_tree_after" != "$librime_git_tree" || -n "$librime_git_status_after" ]]; then
  die "librime source commit, tree, or clean status changed during macOS measurement."
fi
{
  echo "source_commit_after=$yune_git_head_after"
  echo "source_tree_after=$yune_git_tree_after"
  echo "source_clean_after=true"
  echo "librime_commit_after=$librime_git_head_after"
  echo "librime_tree_after=$librime_git_tree_after"
  echo "librime_source_clean_after=true"
} >> "$output_root/external-provenance.txt"
write_readme
log "done: $output_root"

#!/usr/bin/env python3
"""Curate the M59 luna leading-single composition oracle fixture.

Reads the raw librime probe snapshots emitted by
scripts/capture-m59-luna-composition.ps1 (paged candidate lists + the
moboyi -> mo/bo/yi partial-selection composition chain) and writes the
checked-in fixture consumed by
crates/yune-core/tests/upstream_luna_leading_single_composition.rs.

Usage: curate-m59-luna-composition.py <pages.json> <compose.json> <output.json>
"""
import collections
import json
import sys

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
    ("moboyi_compose", "moboyi", "PRIMARY non-lexicon phrase"),
]


def main() -> int:
    pages_path, compose_path, out_path = sys.argv[1], sys.argv[2], sys.argv[3]
    pages = json.load(open(pages_path, encoding="utf-8-sig"))
    compose = json.load(open(compose_path, encoding="utf-8-sig"))

    by_scenario = collections.OrderedDict()
    for snap in pages:
        by_scenario.setdefault(snap["scenario"], []).append(snap)

    inputs = {}
    for name, snaps in by_scenario.items():
        seen = []
        for snap in snaps:
            for cand in snap.get("selected_candidates") or []:
                text = cand.get("text")
                if text not in seen:
                    seen.append(text)
        target = TARGETS[name]
        inputs[name] = {
            "input": name,
            "target_single": target,
            "target_global_index": seen.index(target) if target in seen else None,
            "page_0": [c.get("text") for c in (snaps[0].get("selected_candidates") or [])],
            "page_size": snaps[0].get("page_size"),
            "total_unique_captured": len(seen),
        }

    compose_by_scenario = collections.OrderedDict()
    for snap in compose:
        compose_by_scenario.setdefault(snap["scenario"], []).append(snap)

    compositions = {}
    for scenario, human_input, note in COMPOSITIONS:
        snaps = compose_by_scenario.get(scenario, [])
        chain = [
            {"step": s.get("label"), "preedit": s.get("preedit"), "commit_text": s.get("commit_text")}
            for s in snaps
        ]
        final_commit = next((s["commit_text"] for s in reversed(chain) if s["commit_text"]), None)
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
        },
        "capture": {
            "method": "scripts/capture-m59-luna-composition.ps1 + scripts/curate-m59-luna-composition.py "
            "via scripts/oracle-rime-probe.cs (DllImport rime.dll)",
            "modules": ["default"],
            "note": "Leading-single reachability + partial-selection composition provenance for M59. "
            "PRIMARY case: moboyi -> the non-lexicon phrase 莫伯洢. Positions are the "
            "oracle's; Yune's PRODUCT completion ordering diverges (recorded).",
        },
        "inputs": inputs,
        "compositions": compositions,
    }
    with open(out_path, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(json.dumps(fixture, ensure_ascii=False, indent=2) + "\n")
    sys.stderr.write("wrote " + out_path + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

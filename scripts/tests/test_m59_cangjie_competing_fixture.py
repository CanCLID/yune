import copy
import hashlib
import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "build-m59-cangjie-competing-fixture.py"


def load_module():
    specification = importlib.util.spec_from_file_location(
        "m59_cangjie_competing_fixture", SCRIPT
    )
    module = importlib.util.module_from_spec(specification)
    assert specification.loader is not None
    specification.loader.exec_module(module)
    return module


builder = load_module()


def run_git(repo: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *arguments],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    return result.stdout.strip()


def initialize_repo(path: Path) -> None:
    path.mkdir(parents=True)
    subprocess.run(["git", "init", "-q", str(path)], check=True)
    run_git(path, "config", "user.name", "M59 fixture test")
    run_git(path, "config", "user.email", "m59-fixture@example.invalid")


def commit_all(path: Path, message: str) -> tuple[str, str]:
    run_git(path, "add", ".")
    run_git(path, "commit", "-q", "-m", message)
    return run_git(path, "rev-parse", "HEAD"), run_git(path, "rev-parse", "HEAD^{tree}")


def canonical_bytes(value) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


class CompetingFixtureEnvironment:
    owner_rows = [
        ("hwmvsqtt", "AB", ["A=hwmvs", "B=qtt"]),
        ("ebcnyripm", "CD", ["C=ebcn", "D=yripm"]),
        ("takohaeosk", "EFG", ["E=tak", "F=oha", "G=eosk"]),
    ]
    control = ("hdaetcu", "HI")
    atom_rows = [
        ("A", "hwmvs"),
        ("B", "qtt"),
        ("C", "ebcn"),
        ("D", "yripm"),
        ("E", "tak"),
        ("F", "oha"),
        ("G", "eosk"),
        ("H", "hda"),
        ("I", "etcu"),
    ]

    def __init__(self, root: Path, missing_code: str | None = None):
        self.root = root
        self.cangjie = root / "rime-cangjie"
        self.essay_repo = root / "rime-essay"
        self.essay = self.essay_repo / "essay.txt"
        self.oracle = root / "cangjie5-composition.json"
        initialize_repo(self.cangjie)
        initialize_repo(self.essay_repo)
        inputs = [row[0] for row in self.owner_rows] + [self.control[0]]
        root_codes = builder._ordered_unique(character for value in inputs for character in value)

        main = """---
name: cangjie5
version: test
sort: by_weight
use_preset_vocabulary: true
max_phrase_length: 7
min_phrase_weight: 100
columns:
  - text
  - code
  - stem
import_tables:
  - cangjie5.base
  - cangjie5.stem
  - cangjie5.extended
encoder:
  exclude_patterns:
    - '^x.*$'
    - '^z.*$'
  rules:
    - length_equal: 2
      formula: "AaAzBaBbBz"
    - length_equal: 3
      formula: "AaAzBaBzCz"
    - length_in_range: [4, 10]
      formula: "AaBzCaYzZz"
  tail_anchor: "'"
...
"""
        (self.cangjie / "cangjie5.dict.yaml").write_text(main, encoding="utf-8")

        base_rows = [row for row in self.atom_rows if row[1] != missing_code]
        base_rows.extend((code, code) for code in root_codes if code != missing_code)
        base = """---
name: cangjie5.base
version: test
columns:
  - text
  - code
...

""" + "".join(f"{text}\t{code}\n" for text, code in base_rows)
        stem = """---
name: cangjie5.stem
version: test
columns:
  - text
  - code
  - stem
...

@\th\th'
"""
        extended = """---
name: cangjie5.extended
version: test
columns:
  - text
  - code
...

$\th
"""
        (self.cangjie / "cangjie5.base.dict.yaml").write_text(base, encoding="utf-8")
        (self.cangjie / "cangjie5.stem.dict.yaml").write_text(stem, encoding="utf-8")
        (self.cangjie / "cangjie5.extended.dict.yaml").write_text(
            extended, encoding="utf-8"
        )
        cangjie_commit, cangjie_tree = commit_all(self.cangjie, "pinned cangjie")

        scoring_texts = builder._ordered_unique(
            [text for text, _code in base_rows]
            + ["@", "$"]
            + [row[1] for row in self.owner_rows]
            + [self.control[1]]
            + inputs
        )
        self.essay.write_text(
            "".join(f"{text}\t{1000 - index}\n" for index, text in enumerate(scoring_texts)),
            encoding="utf-8",
        )
        essay_commit, essay_tree = commit_all(self.essay_repo, "pinned essay")

        cases = []
        composition_rows = []
        for input_text, target, char_codes in self.owner_rows:
            cases.append(self._case(input_text, target))
            composition_rows.append(
                {
                    "input": input_text,
                    "target": target,
                    "target_codepoints": "test",
                    "char_codes": char_codes,
                    "provenance": "test oracle candidate zero",
                }
            )
        cases.append(self._case(self.control[0], self.control[1]))
        document = {
            "status": "cangjie5_capture_curated_complete",
            "canonical": True,
            "capture": {
                "source_repositories": {
                    builder.ESSAY_REPOSITORY: {
                        "commit": essay_commit,
                        "tree": essay_tree,
                        "clean": True,
                        "status_short": [],
                    }
                }
            },
            "oracle": {"commit": builder.PINNED_LIBRIME_COMMIT},
            "schema": {
                "source_repo": builder.SCHEMA_REPOSITORY,
                "source_commit": cangjie_commit,
                "source_tree": cangjie_tree,
            },
            "cases": cases,
            "composition_rows": composition_rows,
            "control_rows": [
                {
                    "input": self.control[0],
                    "candidate_index": 0,
                    "target": self.control[1],
                    "target_codepoints": "test",
                }
            ],
            "source_slice": {
                "atomic_codes": ["hwmvs", "qtt", "ebcn", "yripm", "tak", "oha", "eosk"]
            },
        }
        self.write_oracle(document)

    @staticmethod
    def _case(input_text: str, target: str):
        return {
            "input": input_text,
            "preedit": input_text,
            "commit_text_preview": target,
            "all_candidates": [{"global_index": 0, "text": target, "comment": " oracle "}],
            "captured_all_pages": True,
        }

    def write_oracle(self, document) -> None:
        self.oracle.write_bytes(canonical_bytes(document))

    @property
    def oracle_sha256(self) -> str:
        return hashlib.sha256(self.oracle.read_bytes()).hexdigest()

    def build(self):
        return builder.build_fixture(
            self.oracle,
            self.cangjie,
            self.essay,
            expected_oracle_sha256=self.oracle_sha256,
        )


class M59CangjieCompetingFixtureTests(unittest.TestCase):
    def test_extracts_complete_ordered_cohorts_and_scoring_paths(self):
        with tempfile.TemporaryDirectory() as temporary:
            environment = CompetingFixtureEnvironment(Path(temporary))
            document = environment.build()

        self.assertEqual(
            document["scope"]["owner_atom_codes"],
            ["hwmvs", "qtt", "ebcn", "yripm", "tak", "oha", "eosk"],
        )
        self.assertEqual(document["scope"]["control_codes"], ["hda", "etcu"])
        self.assertEqual(
            document["scope"]["one_letter_root_codes"],
            ["h", "w", "m", "v", "s", "q", "t", "e", "b", "c", "n", "y", "r", "i", "p", "a", "k", "o", "d", "u"],
        )
        self.assertEqual(
            [row["text"] for row in document["exact_code_cohorts"]["h"]],
            ["h", "@", "$"],
            "cohort order must follow base, stem, then extended import order",
        )
        self.assertEqual(
            [row["source_file"] for row in document["exact_code_cohorts"]["h"]],
            [
                "cangjie5.base.dict.yaml",
                "cangjie5.stem.dict.yaml",
                "cangjie5.extended.dict.yaml",
            ],
        )
        self.assertEqual(
            document["competing_segmentations"][0]["root_candidate_zero_sentence"],
            "hwmvsqtt",
        )
        self.assertEqual(document["essay_source"]["missing_selected_texts"], [])
        self.assertTrue(
            all(
                row["essay_rows"]
                for rows in document["exact_code_cohorts"].values()
                for row in rows
            )
        )

    def test_canonical_writer_is_byte_deterministic_and_create_new(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            environment = CompetingFixtureEnvironment(root / "inputs")
            document = environment.build()
            first = root / "first.json"
            second = root / "second.json"
            builder.write_canonical_json_create_new(first, document)
            builder.write_canonical_json_create_new(second, document)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            self.assertFalse(first.read_bytes().startswith(b"\xef\xbb\xbf"))
            self.assertTrue(first.read_bytes().endswith(b"\n"))
            with self.assertRaisesRegex(builder.FixtureBuildError, "already exists"):
                builder.write_canonical_json_create_new(first, document)

    def test_competing_root_candidate_zero_uses_weight_before_source_order(self):
        with tempfile.TemporaryDirectory() as temporary:
            environment = CompetingFixtureEnvironment(Path(temporary))
            lines = environment.essay.read_text(encoding="utf-8").splitlines()
            lines = ["@\t999999" if line.startswith("@\t") else line for line in lines]
            environment.essay.write_text("\n".join(lines) + "\n", encoding="utf-8")
            essay_commit, essay_tree = commit_all(environment.essay_repo, "raise stem weight")
            oracle = json.loads(environment.oracle.read_text(encoding="utf-8"))
            oracle["capture"]["source_repositories"][builder.ESSAY_REPOSITORY].update(
                {"commit": essay_commit, "tree": essay_tree}
            )
            environment.write_oracle(oracle)
            document = environment.build()

        first = document["competing_segmentations"][0]
        self.assertEqual(first["root_code_path"][0], "h")
        self.assertEqual(first["root_candidate_zero_characters"][0], "@")
        self.assertEqual(
            document["exact_code_cohorts"]["h"][1]["effective_compiler_weight"],
            999999,
        )

    def test_fails_closed_on_oracle_sha_mismatch(self):
        with tempfile.TemporaryDirectory() as temporary:
            environment = CompetingFixtureEnvironment(Path(temporary))
            with self.assertRaisesRegex(builder.FixtureBuildError, "oracle SHA-256 mismatch"):
                builder.build_fixture(
                    environment.oracle,
                    environment.cangjie,
                    environment.essay,
                    expected_oracle_sha256="0" * 64,
                )

    def test_fails_closed_on_dirty_source_repository(self):
        with tempfile.TemporaryDirectory() as temporary:
            environment = CompetingFixtureEnvironment(Path(temporary))
            with (environment.cangjie / "cangjie5.base.dict.yaml").open(
                "a", encoding="utf-8"
            ) as stream:
                stream.write("dirty\tz\n")
            with self.assertRaisesRegex(builder.FixtureBuildError, "repository is dirty"):
                environment.build()

    def test_fails_closed_when_a_required_exact_code_cohort_is_absent(self):
        with tempfile.TemporaryDirectory() as temporary:
            environment = CompetingFixtureEnvironment(Path(temporary), missing_code="yripm")
            with self.assertRaisesRegex(builder.FixtureBuildError, "cohort is empty for yripm"):
                environment.build()

    def test_fails_closed_on_pinned_commit_mismatch(self):
        with tempfile.TemporaryDirectory() as temporary:
            environment = CompetingFixtureEnvironment(Path(temporary))
            document = json.loads(environment.oracle.read_text(encoding="utf-8"))
            document["schema"]["source_commit"] = "0" * 40
            environment.write_oracle(copy.deepcopy(document))
            with self.assertRaisesRegex(builder.FixtureBuildError, "commit/tree"):
                environment.build()


if __name__ == "__main__":
    unittest.main()

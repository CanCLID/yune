# M61 Failures And Retries

Measured reds were never retried. Setup/tooling failures and deterministic
closeout test-contract reds were preserved. Only their owning or still-unproved
test slices were rerun under explicit retry names, plus source-current
formatting and strict Clippy after each final Rust test edit.

| Classification | Source/stage | Preserved result | Disposition |
| --- | --- | --- | --- |
| setup failure | `f18b0df2` threshold creation | external threshold bytes were accidentally CRLF rather than the frozen LF form; receipt SHA-256 `f0c64b0cd001ac5adc663d364557d5b76bb653e1048d811e6e00ecb69919c904` | recreated exact LF bytes under `retry-0-crlf-threshold`; no measurement had begun |
| compile red | local `91f59696` focused correction proof | Rust `E0282` required an explicit `Vec<String>` type; red/retry receipt SHA-256 `e197f82bf7be6c464d01e0769e32a143e5f335319be9b42bbc1b8fb5b5e6c431` | fixed only the owning three-path correction; named focused retry passed |
| setup failure | restored-tree closeout runner | first detached launcher exited before creating a gate receipt; receipt SHA-256 `5bfd78865c3bbcb5908b9c9b4b77ca64df6f5dd8e672db04c31efaa8c3ce5c7a` | repeated unchanged gates under `retry-1-runner-capture` with launcher output captured |
| deterministic test-contract red | restored-tree `cargo test --workspace` | successful workspace prefix retained; `cantonese_parity` reported `37/41`; raw log SHA-256 `695dec04fc9fcc5f30b363fb4dedf2c17c31c06171427b633d8f8eee082fbd18` | corrected only the stale all-pages comparisons in one cfg(test) path; did not rerun or replace the broad receipt |
| incomplete review interruption | first serial Cantonese retry | stopped deliberately before completion while correction scope was audited; `gate_verdict=none`; interruption SHA-256 `8f90723d167c87ffb46b03169eb8086a24dac3efff7610483239c1dd4c4b2fb0` | reuse prohibited; restarted the owning slice under a new retry name |
| owning-slice recovery | second serial Cantonese retry | formatting, serial `cantonese_parity` `41/41`, and deployed profile-page guard `1/1` passed; status SHA-256 `9c19bc533a4ad5c016e272edad789eaa2a89e9860648a211a2c3c19e4aced144` | retained as the complete interrupted-target recovery |
| deterministic test-contract red | first never-reached API package attempt | strict Clippy and core tail passed; API library then reported `363 passed / 1 failed / 1 ignored`; raw log SHA-256 `3ff4a64c0515d2cec0b82b79008a5093e1c61112d4a80080522bd7381281ab29` | corrected only the stale pre-M56 lifecycle documentation lock in one cfg(test) path; retained the core-tail pass and reran the complete API library target plus still-never-reached suffix |
| disjoint recovery | final API/doc tail | source-current formatting and strict workspace Clippy passed; API library `364 passed / 1 ignored`; API bins/integrations `114 passed / 3 ignored`; API/core doc groups `0` tests each; status SHA-256 `e8817e5f73a8d3507d40f2351bc29b91a7e37dd3853191acf91d88aee1a2d1af` | complete under explicit retry 4; no broad workspace rerun and no duplicate core-integration retry |
| setup failure | public-evidence privacy preflight | the exact external `evidence-integrity` receipt was UTF-16, so the mandatory UTF-8 public-evidence scanner failed closed with `unreadable_public_file` | preserved external SHA-256 `4b597c7d767527c1abedfaeb828357023dee2032960ffc5faac2ddea92e72716`, normalized only the tracked text copy to UTF-8, and reran the owning privacy slice successfully |
| setup failure | evidence-growth path list | PowerShell's UTF-8 BOM was interpreted as part of the first external path, so zero evidence files were evaluated | rewrote the same sorted list as ASCII under `retry-1-ascii-path-list`; the owning growth gate passed |
| setup failure | compact-evidence integrity helper | a scalar pipeline count expression stopped before owner-budget evaluation | reran only the helper under `retry-1-array-count` with explicit arrays; `evidence-integrity.txt` records the pass |
| pre-review evidence-storage red | first isolated closeout candidate | working-tree manifest passed, but tree-mode verification failed because the repository LF rule normalized imported Windows receipts; discarded tree `38e7e33cbe1458e6c7de1bf70b7ec30ee8414ca0`; receipt SHA-256 `cb4db3efb3de60b4315a16e7621fdc9e3890ac1725b44a71ce4540eeee3a4993` | added only the M61 packet-subtree `-text` preservation rule, rebuilt the candidate from scratch, and required worktree plus tree-mode manifest equality before review |

The preserved `7805882d` candidate red, `a39c4d86` artifact-preparation red,
`f18b0df2` Track B red, `67d32a2b` owner-shape red, and `91f59696` owner-
reconciliation red are measurements, not setup failures. None was rerun at the
same source to replace or average away its result.

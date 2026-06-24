# M28 Follow-up Target Decision

The user selected upstream librime engine behavior as the product target for `caksijathaacoenggeoizi` long Jyutping composition. TypeDuck v1.1.2 remains the compatibility oracle for existing TypeDuck profile ABI/comment behavior, but its ranking for this case is not the desired product behavior.

Oracle constraints:

- Upstream `rime/librime 1.17.0` ships no built-in Jyutping schema.
- The M28 follow-up ranking fixture is therefore a hybrid: upstream engine plus pinned Jyutping source YAML deployed by upstream's `rime_deployer.exe`.
- `typeduck.hk/web` and `my-rime.vercel.app` are feel/comparison targets only.
- Dictionary comment payloads remain TypeDuck v1.1.2 / P2-WIN-02 scope because stock upstream lacks the TypeDuck `dictionary_lookup_filter` plugin.

Execution order:

1. Fix Space/default-confirm raw-tail commit.
2. Prove the hybrid upstream-engine Jyutping oracle can be captured from source YAML.
3. If the fixture is captured and accepted, implement only the fixture-backed ranking/generation gaps.
4. If the fixture cannot be captured or accepted, stop and request explicit sign-off for a Yune-authored ranking spec.

Captured result:

- The accepted hybrid fixture captured sentence-first ordering and default
  Space commit for `測是日下場句子`.
- The fixture did not capture a `測試` phrase-prefix row, so this follow-up does
  not invent one.
- The implementation target is the fixture-backed sentence segmentation/ranking
  delta recorded in `phrase-prefix-diagnosis.md`.

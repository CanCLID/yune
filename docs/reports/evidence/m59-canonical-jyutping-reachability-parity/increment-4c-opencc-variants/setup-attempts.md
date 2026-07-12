# Increment 4c performance setup attempts

Two setup attempts stopped before measurement or tracked-tree mutation:

1. The first invocation rejected `extract/dist` because it was a Windows
   junction. The harness correctly refuses reparse-point oracle inputs. The
   junction was removed through the verified oracle-root path and replaced by
   an ordinary byte copy of the official release distribution.
2. The next invocation rejected the oracle root because `rime-shared` did not
   yet exist. The pinned Luna schema repositories and product OpenCC assets
   were then provisioned through `capture-upstream-luna-pinyin.ps1`, producing
   ordinary `rime-shared` and `rime-user/build` trees.

Neither failed preflight created a run directory or measurement. They are not
counted among the five runs and were not retried adaptively after observing a
performance result.

The fresh oracle tree has a different content hash from the historical 4b
scratch tree. Increment 4c does not substitute or silently reuse the old hash:
all five new runs bind the current pinned tree explicitly, compare Yune and
librime against that same immutable input, and pass their end-of-run input-drift
checks. Run 1 built the source-bound benchmark executable; runs 2-5 reused its
exact bytes and receipt.

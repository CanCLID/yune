Disposition: rejected by exact WEB-03 release gate

Source commit: eb117c53f460e749a8a37fee6d978690f3c226f7
Source tree: 2649750b6edfa94dd65467872b0051c8477c2543

The preserved five-run packet passed 32/32 aggregate rows and 160/160
individual observations. It is not acceptance evidence because the subsequent
literal cargo test --workspace gate exposed an oversized uncached prism-family
scan: 442,856 prefix-fallback views against the 5,000 WEB-03 ceiling.

The d2499358 repair and all accepted behavior/performance evidence were
regenerated from a new clean source. No ceiling was changed.

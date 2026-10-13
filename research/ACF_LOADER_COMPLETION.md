# Completed ACF loader reference fixtures

Reference: supplied X-Plane.exe, SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

On 2026-10-13, the previous 60-million-instruction replay budget was found insufficient. The fixture generator had accepted results even when the emulator reported incomplete execution. It now checks reference identity, requires every replay to complete before publishing any fixture, and uses a 200-million-instruction budget. The emulator retains its exact return-sentinel guard and reports the stopping instruction on incomplete execution.

All three supplied Cessna loader calls reached the return sentinel with no replay error:

| Aircraft | Previous words | Completed words | Existing words changed | Added words |
| --- | ---: | ---: | ---: | ---: |
| Cessna_172SP | 12675 | 13690 | 424 | 1015 |
| Cessna_172SP_G1000 | 12703 | 13718 | 424 | 1015 |
| Cessna_172SP_seaplane | 14693 | 15525 | 0 | 832 |

The checked-in numerical fixtures are replaced with the completed results. No original instruction bytes or aircraft files are distributed. Synthetic publishing-contract tests cover failed later replays preserving existing fixtures, successful publication metadata, absent inputs and reference-identity rejection.

## Limits

These counts describe tracked reference words, not fields confirmed equal in the Rust implementation. `crates/xp-acf/tests/object.rs` currently reports mismatches without asserting parity and skips aircraft absent from the local reference directory. A passing cargo command therefore does not establish loader equivalence. Runtime dependencies handled by the emulator remain controlled; return completion does not establish native simulator startup, exact wing boundary construction or complete trajectories. Compatibility estimates remain unchanged. The next loader milestone is enforced comparison of explicitly supported fields against these completed fixtures.

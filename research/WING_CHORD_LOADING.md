# Original wing rib chord loading

Date: 2026-10-13. Reference X-Plane.exe SHA256:
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The original wing loader reads eleven `_chord_for_RIB` float32 values at Wing+0x70. Its final loop `0x141aae9ee..0x141aaea17` multiplies values at indices 0 through Wing+4 (`_els`), inclusive, by the float32 feet-to-metres coefficient `0x3e9c0ebf`. A negative count skips the loop. Values beyond the active count retain the unconverted reader result. Treating this property as one unconditional scaling transform is incorrect.

The object schema now stores the raw array values before the recovered conversion pass. The typed reader uses active rib chord values when supplied, scans all 48 original wing slots, and passes those chords into element calculations and the live grouping boundary inputs. Missing rib arrays use the existing approximate linear fallback. Partial supplied active arrays fail parsing. This does not recover boundary x/y/z coordinates.

## Original-code comparison

`tools/gen_wing_chord_vectors.py` executes the unmodified original instruction range, with RDI=wing, RSI=wing+0x70 and XMM6 containing the original coefficient. It checks the executable digest and exact end-range completion. No stubs, CRT substitutions, original instruction bytes or aircraft files are included in the fixture.

The synthetic corpus contains 312 cases: counts -2, -1 and 0 through 10; eleven values per case; signed zeros, subnormals, extreme finite values, positive and negative chords and deterministic random inputs. The Rust test passes those inputs through text ACF parsing and the public object loader, then requires all 3432 output words to match exactly, including untouched trailing ribs. Debug and release both passed on macOS ARM64. Unit coverage also checks nonlinear active ribs, slot 47 and rejection of a partial supplied array.

Completed-fixture diagnostics on the three supplied aircraft after the fix:

| Aircraft | Exact words | Different words | Missing words | Extra words |
| --- | ---: | ---: | ---: | ---: |
| Cessna_172SP | 10129 | 228 | 3333 | 8267 |
| Cessna_172SP_G1000 | 10147 | 229 | 3342 | 8207 |
| Cessna_172SP_seaplane | 10543 | 228 | 4754 | 9288 |

These counts describe the schema-backed object representation, including defaults and strings; they are not subsystem compatibility percentages. Remaining mismatches include pointer/string representation and unrecovered transforms and fields. No full-loader parity is claimed. The diagnostic is now explicitly ignored by default rather than silently passing when reference files are absent; execute with `OPENXPLANE_REFERENCE_ROOT=/path/to/Xplane12 cargo test -p xp-acf --test object -- --ignored --nocapture`.

## Validation limits

Workspace validation on this host: 242 passed, 0 failed, 2 explicitly ignored; five local flight tests used supplied aircraft. The separately invoked full-object diagnostic ran on all three aircraft but does not assert parity. The recovered object conversion supports counts -2/-1 and 0..10 in the corpus; counts beyond ten are outside the array domain. Exact quarter-chord coordinates, original noise and runtime reference state, complete trajectories and other hosts' results are not established by this change. Compatibility estimates remain unchanged pending broader evidence.

Reproduce the synthetic corpus with the local reference and Unicorn:

```sh
python tools/gen_wing_chord_vectors.py /path/to/X-Plane.exe > /tmp/wing_chord.txt
cmp /tmp/wing_chord.txt crates/xp-acf/tests/data/wing_chord.txt
cargo test --offline -p xp-acf --test wing_chord_exact
cargo test --release --offline -p xp-acf --test wing_chord_exact
```

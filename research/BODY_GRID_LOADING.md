# Body grid coordinate loading

2026-10-13. Baseline SHA256:
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The body mesh loader `0x141aac0a0` reads `_geo_xyz` through `0x141a92fa0` into mesh+0x2c (body+0x65c), with fixed capacities 20 stations, 18 ring points and 3 axes. Its tail `0x141aac3bf..0x141aac4aa` multiplies the entire 1080-float storage by the original float32 feet-to-metres coefficient 0x3e9c0ebf. The vector constant at 0x142614d10 contains that coefficient in all four lanes. The loop processes each 54-float station with packed SSE multiplies plus two scalar tail multiplies. It converts all twenty storage rows, rather than just the active `_s_dim` rows.

The `_body/{}/_geo_xyz/{},{},{}` object-schema entry was unknown and skipped. It now applies the recovered scaling, with body stride 0x34c8, station stride 216, ring stride 12 and axis stride 4. This is stored coordinate loading, not complete runtime mesh/normal preparation.

## Exact evidence

`tools/gen_body_grid_units_vectors.py` executes the entire original SSE loop without stubs, checks the reference digest and exact end-range completion, and verifies the bytes before/after grid storage remain untouched. Its local 64-case corpus covers all 1080 values, all 39 destination body slots through the Rust schema test, signed zeros, subnormals, extreme finite values and deterministic random inputs. All **69120 words** match exactly in debug and release on macOS ARM64. The corpus is local and not distributed.

The separate strict test `body_grid_coordinates_match_completed_original_loader` compares every supplied ACF body-coordinate field against the already published completed original-loader fixtures, including positive and negative zeros. It checks **5400 standard Cessna**, **5400 G1000**, and **4320 seaplane** coordinates: **15120 exact words**, in debug and release. All three aircraft must exist when requested; absent input fails. This test asserts actual field equality, unlike the full-object diagnostic.

Full-object diagnostics after the fix:

| Aircraft | Exact | Different | Missing | Extra |
| --- | ---: | ---: | ---: | ---: |
| Cessna_172SP | 12276 | 228 | 1186 | 11520 |
| Cessna_172SP_G1000 | 12294 | 229 | 1195 | 11460 |
| Cessna_172SP_seaplane | 14108 | 228 | 1189 | 10043 |

Extras include stored zero-valued properties absent from the nonzero reference-word map; counts are not compatibility percentages. No full-loader parity is claimed. All reference-dependent comparisons are explicitly ignored in ordinary CI, where aircraft and local corpora are absent.

## Remaining scope

Runtime inclusion flags, normals and grid update scheduling, coupled curvature/reference-state construction and live reference-bounds integration remain work. Investigation identified `0x141a4e510` and `0x141a4e8c0` as normal-generation passes, not simple coordinate rotations; they are not claimed recovered by this change. Real trajectories and UI/graphics parity remain unverified.

Reproduction:

```sh
python tools/gen_body_grid_units_vectors.py /path/to/X-Plane.exe > /tmp/body_grid_units.txt
OPENXPLANE_BODY_GRID_UNITS_VECTORS=/tmp/body_grid_units.txt cargo test --offline -p xp-acf --test body_grid_units_local -- --ignored --nocapture
OPENXPLANE_BODY_GRID_UNITS_VECTORS=/tmp/body_grid_units.txt cargo test --release --offline -p xp-acf --test body_grid_units_local -- --ignored --nocapture
OPENXPLANE_REFERENCE_ROOT=/path/to/Xplane12 cargo test --offline -p xp-acf --test object body_grid_coordinates -- --ignored --nocapture
OPENXPLANE_REFERENCE_ROOT=/path/to/Xplane12 cargo test --release --offline -p xp-acf --test object body_grid_coordinates -- --ignored --nocapture
```

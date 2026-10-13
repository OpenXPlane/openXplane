# Body and gear reference point-list producers

2026-10-13. Reference executable SHA256:
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

These are two upstream producers of the original aircraft+0x6478 point vector. They feed the already recovered bounds reducer, but are not yet assembled into complete live aircraft bounds.

## Body grid pass

`0x141a88340..0x141a88735` handles one body in the 39-slot array at aircraft+0x6040 (stride 0x34c8). It skips disabled body+0x588, station counts below two, or zero ring counts. Station/ring counts are +0x654/+0x658; the grid starts at +0x65c with a fixed eighteen-point station stride. For each station pair and ring, it emits current/current, next-station/current, next-station/next-ring, current/next-ring, wrapping the last ring to zero. All four quad vertices and duplicates are preserved. Each coordinate is a float32 sum with origin +0x600/+0x604/+0x608.

`body_bounds_points` accepts an explicit already transformed runtime grid. Its supported diagnostic domain is at most twenty stations and eighteen ring points, finite active coordinates. It does not treat raw ACF grid points as transformed native state.

The local generator executes the complete pass with no stubs and exact end-range completion. The scalar vector is preallocated, excluding allocator growth. 120 synthetic cases cover enabled/disabled bodies, stations 0/1/2/3/17/20, rings 0/1/2/7/18, all 39 body slot indices, translated origins, signed zeros and nonmonotonic random coordinates. All **47844 coordinate words** match exactly. The corpus remains local.

## Gear pass

`0x141a87e30..0x141a87fb2` scans ten gear slots at aircraft+0x6080 with stride 0x88. Any nonzero +0 kind is included; zero skips the record. Extent is tire radius (+0x58) plus leg length (+0x18), in that float32 order. It emits two opposite corners: center-extent and extent+center. Center is +0x64/+0x70/+0x7c in the stored CG-relative frame. There is no absolute-value clamp on extent and no eight-corner expansion.

`gear_bounds_points` preserves these gates, operation order and emitted-point order. The local original-code generator covers 500 synthetic ten-slot cases, zero/negative/positive kinds, signed zeros, negative extents and translated centers. All **23952 coordinate words** match exactly. It uses no stubs and preallocated vector capacity; the corpus is local.

## Checks and limitations

Debug and optimized release local comparisons are run explicitly with the corpus environment variables. Ordinary CI checks quad order/fixed row stride/ring closure and gear inclusion/signed extent, but explicitly ignores the original-code comparisons because their corpora are not distributed. No original executable bytes or aircraft files are added to the repository.

Runtime body grid preparation and inclusion state, complete original gear field population, runtime propeller radius/center preparation (the point producer is now separately recovered), the complete aircraft point-vector caller and full geometry-update scheduling remain unfinished. No live reference bounds, curvature integration or full trajectory parity is claimed. Compatibility estimates remain unchanged.

Reproduction requires Unicorn and the supplied baseline EXE:

```sh
python tools/gen_body_bounds_vectors.py /path/to/X-Plane.exe > /tmp/body_bounds.txt
python tools/gen_gear_bounds_vectors.py /path/to/X-Plane.exe > /tmp/gear_bounds.txt
OPENXPLANE_BODY_BOUNDS_VECTORS=/tmp/body_bounds.txt OPENXPLANE_GEAR_BOUNDS_VECTORS=/tmp/gear_bounds.txt cargo test --offline -p xp-airfoil --test body_bounds_local --test gear_bounds_local -- --include-ignored --nocapture
OPENXPLANE_BODY_BOUNDS_VECTORS=/tmp/body_bounds.txt OPENXPLANE_GEAR_BOUNDS_VECTORS=/tmp/gear_bounds.txt cargo test --release --offline -p xp-airfoil --test body_bounds_local --test gear_bounds_local -- --include-ignored --nocapture
```

# Aircraft reference bounds and dimensions

Reference SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
Writer block `0x141a89096..0x141a8923e` inside initializer `0x141a870c0` uses the
float-vector at Aircraft+0x6478/+0x6480, grouped as XYZ triples. It writes minima
at +0x64dc/+0x64e4/+0x64ec and maxima at +0x64e0/+0x64e8/+0x64f0.

All six bounds start at positive zero, so the origin is included even when all
points lie on one side of it. Each point uses SSE min/max semantics (the second
operand wins ties); signed-zero input order matters. The planar halfspan at
+0x64d8 is max((max_x-min_x)/2, (max_z-min_z)/2), with float32 subtraction followed
by double multiplication and float32 narrowing. The three reference-size fields
+0x64f4/+0x64f8/+0x64fc are not full widths: each is max(abs(min),abs(max)) * 1.01,
with double multiplication and final float32 narrowing. The constant at
`0x14253f7e8` is the double 1.01.

`aircraft_bounds::aircraft_bounds` reproduces the block with explicit points.
`tools/gen_aircraft_bounds_vectors.py` runs original instructions without stubs
and enforces exact endpoint completion. 1013 cases cover empty lists, origin,
signed-zero order, points in a single octant, symmetric/asymmetric bounds,
tiny/large magnitudes and randomized vectors. Ten output words per case are
compared exactly; no original data or instruction bytes are distributed.

Upstream point production remains a distinct recovery task. The same initializer
clears the vector at `0x141a87e21` and appends coordinates through loops over
Aircraft+0x6080, +0x6028 and +0x6040, plus later records at +0x6010. The wing loop
is bounded by `0x141a87fd1..0x141a8832d`. These observations do not establish
all geometry construction or inclusion rules. Renderer vertices are not assumed
to be the original vector. Accordingly the live flight model's constant reference
fields have not yet been replaced with guessed mesh bounds.

## Wing point production (2026-10-13)

`wing_bounds_points` reproduces the inclusive boundary loop
`0x141a88000..0x141a88318`: each boundary index from zero through the element
count emits a leading and trailing point. X/Y/Z are sampled through the original
clamped interpolation helper (including the last boundary's rounding), not simply
copied. Leading z subtracts 0.25 times the boundary chord in double precision;
trailing z adds 0.75 times chord in double precision. Each narrows to float32.
The constant at `0x142551ac0` is double 0.75.

`tools/gen_wing_bounds_points_vectors.py` executes the entire loop without stubs.
It explicitly preallocates vector capacity, so allocator-growth behavior is not
part of this test. 512 cases cover 1/4/10 elements, zero/rectangular/tapered chords,
signed zero and randomized nonmonotonic coordinates. Every emitted coordinate
matches exactly in `wing_bounds_points_exact`; no trig tolerance is involved.
The caller remains responsible for enabled-wing selection and additional body,
gear and other point-producing passes. This does not yet replace live reference
fields with a partial wing-only bounding vector.


## Body and gear point production (2026-10-13)

The complete one-body quad-emission pass and ten-slot gear-corner pass are now
ported with exact local original-code comparisons. See
[BODY_GEAR_REFERENCE_POINTS.md](BODY_GEAR_REFERENCE_POINTS.md) for input layouts,
word counts, local reproduction and limitations. The complete runtime vector is
still not assembled; body grid preparation and propeller points remain work.

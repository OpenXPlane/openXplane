# DSF polygon input for facade rendering

The 2026-10-26 decoder now retains polygons from command forms 12 through 15:
explicit indices, a consecutive range, nested explicit contours and nested ranges.
It retains the definition resource, unsigned 16-bit parameter and every scaled
coordinate plane of every point. Extra planes may carry wall selection or Bezier
data; they are not flattened away. Nested ranges read an initial index followed
by one end boundary per contour. Reversed ranges, missing definitions/pools,
out-of-range indices and nonfinite polygon coordinates produce errors.

This implements the published [DSF format](https://developer.x-plane.com/article/dsf-file-format-specification/)
and command alignment described by [DSFLib](https://github.com/X-Plane/xptools/blob/master/src/DSF/DSFLib.cpp).
It is not a native reference-loader replay. The existing object-only API now uses
the same feature decoder, retaining its objects while validating polygon inputs.
Stricter error handling may therefore reject a malformed polygon previously
skipped by the object-only preview; original error parity is unverified.

Run the inspection command against a raw, uncompressed DSF:

```sh
openxplane dsf-info "<X-Plane>/Global Scenery/Global Airports/Earth nav data/+40-130/+47-123.dsf"
```

The installed Global Airports tile completed decoding: 13168 object placements
and 6474 polygons. These are whole-tile counts, not only KSEA. Resource groups
include two STerm_R1 polygons with nine points and three SEA_6Panel polygons with
112 points. Other groups include forests, painted polygons, strings and fences;
a polygon is not necessarily a building.

The installed STerm_R1 and SEA_6Panel resources use type-2 FAC meshes. As the
[facade format](https://developer.x-plane.com/article/x-plane-10-facade-fac-file-format-specification/)
describes, these need segment selection and mesh fitting, plus roof/material
handling. This decoder does not draw them or substitute generic extruded walls.
The existing map captures still omit this geometry. Next: load those FAC mesh,
floor/wall/segment choices and roof definitions, then integrate their geometry
using retained DSF contours and verify against reference output.

Synthetic checks exercise all four polygon command forms, the parameter and
coordinate planes, followed by object commands to verify stream alignment.
Workspace formatting, Clippy and ordinary tests are the port checks; ignored
native replay tests do not run as part of these checks. No new visual parity,
mod compatibility or native-loader equivalence is claimed. Estimates remain
unchanged: polygon inputs are groundwork for the still-missing facade renderer.
No original content or disassembly is distributed.

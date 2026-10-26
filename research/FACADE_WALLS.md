# Installed FAC straight-wall scene integration

The 2026-10-26 scene now uses retained DSF facade contours and installed type-2
FAC meshes to build straight walls. This is a preliminary integration, not
original facade evaluator parity or complete scenery/mod support.

## Axis evidence

The prose [FAC specification](https://developer.x-plane.com/article/x-plane-10-facade-fac-file-format-specification/)
describes wall length along X. Actual KSEA segment bounds and normals instead
indicate length along negative Z, with the exterior toward positive X.
Laminar's [WED facade preview](https://github.com/X-Plane/xptools/blob/master/src/WEDEntities/WED_FacadePreview.cpp)
also scales and advances along that Z direction. This supports the preview's
axis mapping; WED is not evidence of the shipped simulator's exact algorithm.
The source was inspected only in ignored local research; no source is copied here.

REA6.7 bounded inspect-with-objdump observed original EXE addresses
141981450..141981650. The unchanged executable digest is the baseline SHA256 in
BASELINE.md. A byte-pattern candidate for SEGMENT_CURVED was confirmed as a
RIP-relative LEA at1419814bc to string14279bba8, followed by a call at1419814c8
and a conditional branch at1419814cf. The surrounding pdata function is
14197f440..141984034. This locates a candidate native FAC loading path; it does
not recover mesh fitting, prove the called helper's semantics or establish
native numerical/visual parity. The bounded adapter reports its fixed range;
raw disassembly and REA evidence stay local.

## Preview policy and limits

For nearby Global Airports FAC polygons, the scene resolves a direct or library
path, chooses the floor with nearest top roof height, takes explicit DSF wall
indices when present and uses wall0 when absent. It selects a deterministic
spelling/repetition fit and stretches along negative Z to each straight edge.
Positions and inverse-transpose normals are transformed to the airport frame.
Source vertices, UVs and base textures are retained. RING and NO_WALL_MESH affect
wall generation. Flat ground remains a preview policy.

This differs from the complete evaluator: automatic wall rule/heading selection,
random spellings, original repetition and floor tie-breaking, mitered corners,
Bezier curves, roof surfaces, attached objects, hard surfaces, LOD and glass
ordering are unfinished. Curved polygons and multiple contours are skipped;
type-1 resources and unsupported floors are reported. Exclusions and density
filters are still missing. Skipped footprint/resource totals mix those categories.
Textures remain approximate base-color shading, without original lighting.

The KSEA run built88 footprints in22 batches, with94 skipped footprint/resource
cases. The original nearby OBJ count remains5153 in157 resource batches (older
combined logging included facade batches). No paired original capture exists.
The original KSEA terminal is not considered complete while roofs, corners and
other missing geometry remain.

Tests cover negative-Z segment progression, repetition/stretch, east/south edge
orientation, normal rotation, bad wall indices and NO_WALL_MESH. Workspace fmt,
Clippy and ordinary tests cover the port. Native replay tests remain separate.
Scenery estimates are reviewed and unchanged pending broader world integration
and reference evidence. Next: recover native fitting and add corners/roofs,
curves and collision, then compare matched original/port scenes.

![KSEA preliminary FAC walls](../site/assets/ksea-fac-walls-20261026.png)

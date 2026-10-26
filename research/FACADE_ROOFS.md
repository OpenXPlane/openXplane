# Type-2 FAC roof integration

The 2026-10-26 airport scene now generates the selected floor's ROOF_HEIGHT
layers over supported straight, single-contour FAC footprints. Walls and roofs
share one floor-selection function, choosing the nearest top roof height to the
DSF parameter. The selection remains a preview implementation, not native replay.

The roof uses its own SHADER_ROOF base texture, falling back to the wall texture
when none is declared. ROOF_SCALE controls repetition in meters, with a separate
repeat sampler used only for roof meshes. Existing aircraft, terrain and wall
sampler policy is unchanged. UV axes follow the first footprint edge and its
clockwise perpendicular, anchored at that edge's midpoint. This follows the
official [WED preview](https://github.com/X-Plane/xptools/blob/master/src/WEDEntities/WED_FacadePreview.cpp)
and [FAC format](https://developer.x-plane.com/article/x-plane-10-facade-fac-file-format-specification/),
not a claim about the shipped renderer's exact phase or arithmetic.

Simple footprints use the existing ear-clipping triangulator with upward triangle
winding and normals. Incomplete triangulations are rejected. NO_ROOF_MESH and
open RING0 facades suppress roof generation. Missing textures or roof errors do
not remove an otherwise loaded wall. Multiple roof layers are retained; their
height0 layer still uses the flat airport ground rather than source mesh draping.

The actual installed KSEA run built67 roofed footprints, alongside88 wall
footprints, in42 total wall/roof batches. It reported zero roof failures and94
other skipped footprint/resource cases. This is an RTX2080/Vulkan offscreen
port render, not a paired original comparison. No original resources are copied.

![KSEA with preliminary FAC roofs](../site/assets/ksea-fac-roofs-20261026.png)

Tests check area and upward winding of a rectangle, concave triangulation,
texture-space origin, layer heights and selection, roof suppression, and
independence from NO_WALL_MESH. Workspace fmt, Clippy and ordinary tests check
the port; ignored native tests remain separate.

Still missing: native floor/roof phase verification, curved footprints, corner
miters, roof objects/attachments, type1 facades, original lighting/materials,
terrain elevation, exclusions, density and collision. The simulator's exact roof
tessellation is unverified. The separate wall limitations in FACADE_WALLS.md also
apply. The whole-game and unmodified-mod compatibility requirements remain open.

Rubric review: scenery implementation15% to17% for typed FAC input, actual
contour-based wall and multi-layer textured roof integration. This is a planning
judgement about the combined facade work, not a percentage derived from counts.
Verified scenery stays0%, because no native geometry replay or matched original
visual evidence proves parity. Other subsystem estimates are unchanged.

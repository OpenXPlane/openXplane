# Source terrain in airport and flight scenes

The 2026-10-26 airport scene builder now loads the source DSF mesh and TER base
preview used by mesh-render. The artificial central ground slab is omitted when
source terrain loads. Airport-view/render, scenery-render, controls-render and
fly/fly-render share this builder; source terrain is no longer confined to a
separate inspection command. Failed source loading retains the explicit older
flat/raster fallback and reports the error.

## Shared visible and physical surfaces

DSF patches with the physical bit contribute triangles to a local256m-cell
spatial index. Queries use barycentric height interpolation and an upward
geometric normal, selecting the highest available physical triangle. Missing
coverage returns None. The flight preview currently falls back to height0 there;
it does not load adjacent scenery as the aircraft travels.
Boundary inclusion uses a1e-5 barycentric tolerance, which can accept a small
edge margin proportional to triangle size. Broad triangles are queried from a
separate list instead of expanding an unbounded spatial grid.

Airport triangles are subdivided along long edges, up to50m or a bounded depth,
then their vertices sample source physical terrain. Existing relative layer
offsets are raised by0.21m for this preview so pavement/runway/marking geometry
appears above the terrain. Runway and pavement triangles are added as physical
surfaces too; wheel queries therefore use their visible interpolated geometry.
This offset and subdivision are explicit port policies, not native airport
draping, terrain flattening or runway-profile recovery. The first experimental
render retained negative offsets and hid the pavement; it is retained locally as
a failed visual check, not published as proof of a correct airport.

Nearby OBJ/AGP instances query the combined surface height. Graded FAC geometry
uses the first wall midpoint's ground height; draped FAC vertices query height
individually. AGP ground-point/slope rules and exact graded semantics are not yet
reproduced. Mesh normals/materials and aircraft/world projection still have the
limitations recorded in DSF_MESH.md, TERRAIN_MATERIALS.md and FACADE_ROOFS.md.

Wheel penetration and damping now use the sampled ground normal; longitudinal
and lateral friction lie in its tangent plane. Spawn/reset preserve the preview's
level resting pose shifted to the surface. Reattaching the same ground does not
double the shift. This is integration into the approximate flight model, not
recovery of native collision/gear physics. Water/surface-specific contact,
bumps, body/object collision, runway grading, slope-aligned initialization and
continuous contact geometry remain incomplete.

## Checks and scope

Synthetic tests cover negative grid cells, boundary points, sloping triangles,
overlapping surfaces and fallback to base terrain outside airport overlays.
The airport draping test checks triangle area and sampled height on a sloping
plane. A test explicitly run with the installed Cessna ACF raises a physical
surface25m, checks repeated attachment, gear rest after5s and reset. The local
flight suite reports7 passed/1 ignored; those behavioural checks are not
original-code comparisons. Ignored native replay tests remain separate.

The corrected installed KSEA fly-render completed frames at0s and5s. At5s the
port reports11.8kt,411.4ft MSL and1.2 degrees pitch. RTX2080/Vulkan,1280x800;
default scripted pilot and summer base-terrain preview. The strip remains the
approximate apt.dat solid-color runway, now visible above physical terrain.
These are two port-only snapshots, not continuous motion or original parity.

![Cessna on KSEA with source terrain at5s](../site/assets/ksea-live-terrain-20261026.jpg)

Workspace fmt, Clippy and ordinary tests check the changes. Rubric review raises
scenery implementation18% to21% for live source terrain, airport placement and
physical height integration. This is a scope judgement, not a score calculated
from screenshots or test counts. Verified scenery stays0%; recovered flight-model
kernel estimates are unchanged, since this approximate contact code does not
establish native gear/ground parity.

The whole-game, mod compatibility and 1:1 requirements remain open. Terrain
materials still lack border/composite/normal/decal effects; all patch LODs are
drawn. Exclusions, forests, roads, neighboring tiles, exact library conditions,
native contact and paired original visual/motion evidence remain unfinished.

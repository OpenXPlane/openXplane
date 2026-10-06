# Airport ground geometry from apt.dat

Date: 2026-10-06. Source: the provided `apt.dat` (format 1200) only; the original EXE was not analysed for this
part. Everything here is built from the file's rows and the public apt.dat conventions, so the rules are
labelled by how they are supported. The result is checked visually, not against the original's renderer.

## What is read and drawn

| Input row | Used for |
|---|---|
| `1302 datum_lat` / `datum_lon` | the origin of the local frame (else the first runway's midpoint) |
| `100` land runway | a strip of the runway width between its two ends, a white centreline, edge lines and threshold bars |
| `110` pavement + `111`-`114` nodes | filled polygons: taxiways, aprons |

Pavement rows are interpreted only directly after a `110` header (nodes also occur in linear features `120`
and boundaries `130`, which are not interpreted). A loop ends at a closing node (`113` plain, `114` bezier). The
first loop of a pavement is its outline and later loops are holes, drawn in the ground colour above the
pavement. A node with a control point (`112`, `114`) makes the adjacent segments cubic beziers (8 samples); the
incoming handle of the next node is its control point mirrored through the node. The loop and hole convention
and the bezier handle rule come from public apt.dat documentation and are not confirmed in the reference build.

## Projection and layers

`src/world.rs` projects latitude/longitude onto a flat local frame (x east, y up, z south) around the datum on
a sphere of radius 6,371,000 m. There is no datum model, no elevation (everything is on y = 0) and no curvature,
which is fine for an airport-sized area. Polygons are triangulated with ear clipping (either winding; a
self-intersecting loop yields only the triangles that could be clipped). Layers are drawn in order with small
height offsets just below the flight model's ground plane at height 0 (ground -0.20 m, pavement -0.12 m, holes
-0.09 m, runway -0.03 m, markings -0.01 m). Colours by surface code
(1 asphalt, 2 concrete, 3 turf, 4 dirt, 5 gravel, 12 lakebed, 13 water, 14 snow, anything else asphalt grey)
are openXplane policy; the codes `50`/`51` used by KSEA runways are not decoded and so draw as asphalt.

## Aircraft placement

`airport-render` and `airport-view` put the aircraft 150 m from the first runway's first end with the nose
(local -Z) along the runway and the lowest vertex 0.2 m above the surface. This is a static placement for
viewing, not a spawn position taken from the original.

```sh
cargo run --offline -- airport-render Xplane12 KSEA "Xplane12/Cessna 172 SP/Cessna_172SP.acf" ksea.png
cargo run --offline -- airport-view Xplane12 KSEA "Xplane12/Cessna 172 SP/Cessna_172SP.acf"
```

The first argument is an installation folder (see [INSTALL_LAYOUT.md](INSTALL_LAYOUT.md)), so the airport is
found through the scenery packs and Global Airports in priority order.

## Not done

Taxiway lines and signs, runway numbers, lights, windsocks, buildings and other objects (library objects, DSF),
terrain relief and orthophoto textures, runway shoulders and surface roughness, the apt.dat linear features
(`120`) and boundaries (`130`), and picking a spawn point from start locations (`1300`). Runway markings are a
simplified set, not the original's per-runway marking codes.

## Checks

57 unit tests pass, Clippy without warnings. New tests cover the projection (one degree of latitude is about
111.2 km towards -z), triangulation of convex and concave polygons in either winding with area checks, bezier
flattening, pavement loops with holes and bezier nodes, and the assembled layers of a small airport. On KSEA the
render shows the three parallel runways' ground layout with the taxiway and apron pavements, the holes as
green islands, and the Cessna standing on runway 16L (screenshot in `assets/screenshots/ksea-cessna.png`). The
native window opens and presents frames on Metal (`airport-view ... --smoke`).

# Installed TER base texture preview

The 2026-10-26 source-mesh inspection now resolves installed TER resources and
their base textures. It reads BASE_TEX/BASE_TEX_NOWRAP, NO_ALPHA, PROJECTED,
PROJECT_OFFSET and PROJECT_ANGLE metadata. Explicit ST uses DSF planes5/6;
vertical projection uses local meters and the declared scale/offset. Nonvertical
projection is unsupported and reported; the inspection uses a constant sample
when UV evaluation fails. Base texture alpha is forced opaque only for NO_ALPHA.
Wrapping uses the material's repeat/nowrap flag in the existing GPU sampler.

This follows the [TER specification](https://developer.x-plane.com/article/terrain-type-ter-file-format-specification/),
not a recovered native material evaluator. In particular local projection phase,
offset sign and the exact geographic anchor are unverified approximations.
Other directives are retained as unsupported names for the runtime report.
Border masks, composite/noise/auto-heading/slope/decal shaders, normal maps,
night lighting, seasons/climate blending, water shading and material collision
properties are not implemented. Patch overlay depth/alpha blending and LOD are
still incomplete, causing visible hard boundaries and repeated textures.

The first run resolved only6 textured materials: most X-Plane12 global terrain
uses EXPORT_SEASON. The library index now supports seasonal EXPORT, RATIO,
EXTEND and EXCLUDE with the same candidate priority rules as legacy commands.
The season must be explicit for those commands; no-season catalogs skip them.
The inspection defaults to token sum and reports this preview policy, with
OPENXPLANE_SCENERY_SEASON=spr/sum/fal/win for controlled comparisons. This is not
calendar, climate or snow selection. Other airport library queries still use
their previous context; full mod-wide seasonal behaviour is not established.

```sh
openxplane mesh-render "<X-Plane>" KSEA mesh.png
```

The successful summer KSEA GPU run resolved77 textured resource batches across
39303 triangles/78 terrain definitions. The remaining water batch uses the
diagnostic color. Image: RTX2080/Vulkan, port-only capture, no matched reference.

![KSEA source mesh with installed base textures](../site/assets/ksea-ter-base-20261026.jpg)

This is still a separate mesh inspection, without airport layers, objects or
aircraft. The playable airport/fly scene continues to use the earlier slab/raster.
Next: implement mask/composite coordinates, projection and original materials;
integrate source terrain, draped airport/scenery placement and physical patch
height/collision into the live scene. Whole-game and original-mod goals remain.

Tests cover explicit ST, vertical scale/offset, nowrap/NO_ALPHA, unsupported
directives/projection, invalid dimensions/numbers and seasonal export filtering,
priority extension and quantity parsing. Workspace fmt, Clippy and ordinary
tests check the port. Ignored native replays remain separate evidence.
Scenery estimates are reviewed and unchanged: incomplete base shading in an
inspection command does not establish live world rendering or original parity.
Original assets and code are read locally and not distributed.

# Installed airport DSF object preview

On 2026-10-26 the KSEA scene loaded object placements from the installed
Global Airports tile `+47-123.dsf`, reading original files in place. This is
public-format-based integration, not a recovered native loader or proof of
visual parity. The whole-game and original mod compatibility goals remain open.

The decoder reads object definitions, 16-bit coordinate pools, per-plane
scales, raw/delta/RLE encodings and object single/range commands. It walks the
other supported command records without treating their payload as objects.
Nested polygon ranges consume the initial index plus one boundary per winding.
Synthetic tests exercise scaling, range alignment, encodings and malformed data.

Sources: [DSF specification](https://developer.x-plane.com/article/dsf-file-format-specification/)
and [X-Plane DSFLib](https://github.com/X-Plane/xptools/blob/master/src/DSF/DSFLib.cpp).
No original code or assets are included in the repository.

The renderer groups nearby placements by resolved resource, parses OBJ8 geometry
and textures, and applies placement headings. The preliminary library index
searches enabled custom packs followed by default scenery; it is not yet the
original library selection algorithm.

Actual KSEA run: 13168 placements in the tile, 2370 rendered instances within
the preview's 12km square, 111 resource batches, zero unresolved nearby paths
and 23 failed resources. Failures include AGP packages and an empty placeholder
OBJ. GPU: NVIDIA GeForce RTX 2080, Vulkan, offscreen rendering.

![KSEA object preview; incomplete scenery](../site/assets/ksea-dsf-objects-20261026.png)

Reproduce with an installed aircraft path:

```sh
openxplane scenery-render "<X-Plane>" KSEA "<aircraft.acf>" scenery.png
```

Objects currently use flat airport-relative height and zero-dataref animation
state. Optional object elevation/AGL state, exclusions, density, filters,
regional/seasonal/random exports and exact BACKUP/EXTEND rules are unresolved.
Only the Global Airports overlay tile is instantiated; custom overlay ordering,
adjacent tiles, AGP, facades and networks remain missing. Terminal buildings
and terrain materials are visibly incomplete. The raster preview and central
slab limitations in [DSF_TERRAIN.md](DSF_TERRAIN.md) still apply.

Validation: workspace formatting, Clippy with warnings denied and ordinary
workspace tests passed. Ignored local native-replay tests were not executed by
that test command. The image is a port-only capture, not a paired reference.
Scenery implementation estimate rises from 12% to 15% for pool/command loading,
library lookup and integrated instances; verified scenery remains 0%.

Next: support AGP and facade structures, original library/overlay selection,
source terrain mesh/materials and consistent placement/collision heights.

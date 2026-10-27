# Installed DSF elevation preview

Enabled custom scenery packages are now searched in `scenery_packs.ini` order
for a base DSF before global scenery. The documented `sim/overlay=1` property
excludes overlays from that choice; unreadable candidates are reported and
skipped. See [DSF usage](https://developer.x-plane.com/article/dsf-usage-in-x-plane/).
Terrain definitions are also resolved relative to the selected package before
the preview library. A custom mesh without an elevation raster uses the global
raster as a preview fallback, which is not proof of correct native elevations.
Synthetic tests cover ordering, disabled packs, overlay and global fallback.
The installed `2_UHWW_Ortho` tile is an overlay, not a base mesh. Overlay DSFs
above the selected base are now read in pack order, with their object, facade and
draped polygon placements passed through the existing preview renderers. DSF
exclusion zones and airport ownership filters are not implemented; layer order,
height and resource interpretation remain approximations. No 1:1 parity is claimed.
An offscreen UHWW run selected the global `+43+132` base and rendered 16885
source triangles, with 1198 nearby object instances. The resulting
[preview](../site/assets/uhww-base-selection.jpg) is not paired with an original
capture and does not demonstrate custom base-mesh rendering.

With overlay loading enabled, an installed UHWW run reads both `1_UHWW` and
`2_UHWW_Ortho`. The latter supplies 12 draped polygons, all resolved by the
preview, while `1_UHWW` reports 652 unresolved object placements and three
failed resources, including nested animation keys and non-UTF-8 text. These
observations are local to this installation and do not establish mod parity.
The first full overlay preview exhausted GPU memory on an RTX 2080. The preview
now resizes uploaded color textures to a maximum dimension of 1024 pixels by
default (`OPENXPLANE_TEXTURE_MAX_SIZE` overrides it). This is a visual-quality
compromise, not the simulator's texture streaming or filtering behavior.
The subsequent run completed on that GPU, resizing 181 textures. Its
[UHWW screenshot](../site/assets/uhww-overlay-preview.jpg) shows the photo layer
and visible remaining seams; no corresponding original screenshot was captured.

The user's request is to load the actual game maps instead of a runway on a flat
plane. This first integration reads the installed global scenery DSF elevation
raster in memory, including 7z-compressed DSF. It does not yet load a complete map.

Source: the [official DSF specification](https://developer.x-plane.com/article/dsf-file-format-specification/).
This is a documented-format implementation, not recovered native-loader parity.
No original files or assets are distributed. The original tile remains read-only.

Executed with installed KSEA tile `+47-123.dsf` under
`Global Scenery/X-Plane 12 Global Scenery/Earth nav data/+40-130/`:
compressed size16032609bytes, decompressed size31609559bytes. The elevation layer
is a1201x1201 signed16-bit post-centered raster, scale1 and offset0. The decoder
uses sevenz-rust2 in memory; archive CRCs are checked. The DSF MD5 footer is not yet
validated. Atom lengths, dimensions, finite heights and supported formats are
checked; malformed synthetic fixtures are rejected. Signed scaled corner/center
sampling tests and workspace fmt/Clippy/tests pass.

`fly`, `airport-view`, `airport-render` and `controls-render` now attempt to read
the global base tile at the airport datum. Missing/unsupported terrain is reported
and retains the old airport view. Custom base-mesh precedence, adjacent tiles,
DSF exclusions and region/season libraries are not implemented here.

The preview generates a100m grid over30km, with129438 triangles at KSEA. Heights
are relative to airport elevation. Its center remains the existing16km airport
slab; mesh draping and physics collision still use the earlier flat model. Outside
the loaded tile the raster sampler clamps to the edge. The solid terrain color,
regular grid, central slab and interpolation are explicit preview approximations,
not the DSF's original triangle topology or terrain shaders. No terrain textures,
water classification, DSF objects, facades, roads or vegetation are drawn yet.

```sh
openxplane terrain-render "<X-Plane>" KSEA "<X-Plane>/Aircraft/Laminar Research/Cessna 172 SP/Cessna_172SP.acf" terrain.png
```

![First installed DSF elevation preview, incomplete map](../site/assets/ksea-dsf-raster-20261026.png)

This actual GPU render shows both loaded peripheral terrain and the unresolved
central slab. It is evidence of raster integration, not matching map appearance.
Next work: read coordinate pools/commands and library resources; instantiate actual
objects and terrain materials, then replace the slab with the source mesh and
consistent collisions. The whole-game goal and paired-reference criteria remain.

Rubric review: scenery implementation10% to12% for compressed DSF/raster loading
and scene integration; verified scenery remains0%. Other shares unchanged.
Weighted estimates42.56%/28.33% display42.6%/28.3%. These are coverage judgements.

# Installed DSF elevation preview

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

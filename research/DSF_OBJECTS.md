# Installed airport DSF object preview

The post-season-fix UHWW offscreen run completed on RTX 2080. AEP reports
350 excluded object placements, zero unresolved and zero failed resources.
Comparing the RGBA capture with the preceding legacy-comment run found zero
changed pixels (maximum channel difference zero). Resolving the seasonal names
therefore did not change this fixed view; it must not be presented as a visual
improvement or as proof of seasonal runtime parity. The existing
[capture](../site/assets/uhww-legacy-comments.jpg) represents the same pixels.

Texture-resolution follow-up: UHWW's `plita_PAG_14`, `_seam` and `_shadow`
POL resources declare DDS names but the package contains matching PNG files
instead. The preview currently fails those materials. Reverse DDS-to-PNG
substitution has not been confirmed in the reference loader, so this observation
is a next investigation target rather than a claim that those files are absent.

REA now captures the bounded texture-loader range `0x1419eb8c0..0x1419ebba0`
from the identified reference build. It references `.dds` at `0x1419eba01` and
`.png` at `0x1419ebb64`, with string-building calls to `0x1405eca90`.
This establishes separate filename candidates, not successful fallback or their
selection order. Trace subsequent existence checks and loader flags before
changing runtime resource resolution. Original listings remain local and ignored.
Following the original routine through `0x1419ec4df` shows vector construction
of string candidates, mode-dependent branches and optional prefix replacement.
The call at `0x1419ec3a9` compares a prefix, rather than proving filesystem
existence. No candidate-open attempt has been established in this range.
The next step is to trace consumers of the produced vector before assigning
native DDS/PNG fallback semantics.
The caller `0x1419ec820..0x1419ecac9` tests supplied records through
`0x140d062c0`, which adds `0x60` and enters `0x140d062d0`. That routine queries
a sorted string range via `0x140cec650`, compares string length, then compares
bytes via `0x1423c0180`; it returns a boolean and optionally a record index.
Thus the observed boolean is name membership, not an established disk-open
result. Trace the collection's population and subsequent file loading next.
`tools/probe_texture_name_membership.py` executes the original comparison with
a controlled search-result pointer. Four inline ASCII cases confirm an exact
match succeeds with index zero, while changed extension, case and length fail.
The original byte comparison runs without substitution; the lower-bound search
result is supplied. This is not a filesystem, full search or fallback test.

Further inspection of the remaining 13 package-relative UHWW placements found
five missing filenames: three industrial resources and two fuel-tank resources.
None of those files exists under `1_UHWW`, and no matching export was found in
installed `library.txt` files. This supports an incomplete installed dependency
set for these references; it does not establish native fallback/search parity.

The overlay renderer and resource audit now use the same explicit preview season
as terrain (`OPENXPLANE_SCENERY_SEASON`, default `sum`). Previously they omitted
seasonal exports. Repeating the installed UHWW audit resolves all 70 formerly
missing AEP leaf placements before exclusions. The main custom overlay still
has 652 unresolved placements: 115 under 3D_people_library, 330 MisterX_Library,
2 RE_Library, 138 handyobjects, 54 opensceneryx and 13 package-relative objects.
The named library folders are absent from the inspected Custom Scenery directory;
this does not prove identical original search behavior. No new rendered frame
or native seasonal-selection comparison was made for this correction.

`scenery-object-audit <installation> <ICAO>` lists unresolved object resources
within the same 6 km preview bounds, grouped by name and overlay package. Counts
are before exclusions and AGP expansion; no geometry or GPU textures are loaded.
The installed UHWW audit finds references under handyobjects and opensceneryx,
plus 70 AEP leaf placements. This identifies unresolved references only, not
whether a library is missing, disabled or interpreted incorrectly. Full native
library-resolution comparison remains outstanding.

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
regional/seasonal/random exports and native library selection remain unresolved.
Unconditional BACKUP/EXTEND/EXCLUDE candidate rules were subsequently corrected;
see [LIBRARY_PRECEDENCE.md](LIBRARY_PRECEDENCE.md).
Only the Global Airports overlay tile is instantiated; custom overlay ordering,
adjacent tiles, complete AGP, facades and networks remain missing. Terminal buildings
and terrain materials are visibly incomplete. The raster preview and central
slab limitations in [DSF_TERRAIN.md](DSF_TERRAIN.md) still apply.

Validation: workspace formatting, Clippy with warnings denied and ordinary
workspace tests passed. Ignored local native-replay tests were not executed by
that test command. The image is a port-only capture, not a paired reference.
Scenery implementation estimate rises from 12% to 15% for pool/command loading,
library lookup and integrated instances; verified scenery remains 0%.

Next: support AGP and facade structures, original library/overlay selection,
source terrain mesh/materials and consistent placement/collision heights.

## AG_POINT object annotations

The subsequent AGP preview reads object definitions, tile bounds, pixel scale,
anchor, quarter-turn tile rotation and OBJ_DRAPED/GRADED/DELTA/SCRAPER
annotations. Scale uses square pixels, including when texture width and height
differ. Referenced OBJ paths resolve relative to the AGP or through the preview
library index. DELTA adds its meter offset to the flat preview ground.

This follows the [published autogen format](https://developer.x-plane.com/article/x-plane-10-autogen-scenery-file-format/),
not a native loader comparison. First tile and full object density are explicit
preview policies. Random tile/density choices, slope screening, ground-point
sampling, base tile rendering, facade and vegetation annotations are unfinished.
Orientation and placement still require matched original runtime captures.
Tests cover noncentral anchors, square-pixel scaling, delta height, invalid
indices/nonfinite dimensions and first-tile selection. Native replay tests are
not part of this new parser's evidence. Estimates remain unchanged: this extends
the existing preliminary instance loader and does not prove mod or visual parity.

The actual KSEA GPU run expanded 192 AGP placements. Total nearby rendered
instances rose from 2370 to 4107 in 156 resource batches. One child resource
remained unresolved; one empty OBJ placeholder failed parsing. The image still
shows missing terminal/facade geometry and solid-color terrain. It cannot prove
original-equivalent placement or rendering. Workspace fmt, Clippy and tests
cover this implementation; the native/original visual comparisons remain absent.

![KSEA with AGP object annotations](../site/assets/ksea-agp-20261026.png)

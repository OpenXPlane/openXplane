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
instead. The subsequent POL-only resolver now searches DDS, PNG, BMP, then KTX2
instead of treating the missing declared DDS as a missing material. See the
native mode and ordering evidence below. KTX2 decoding remains unsupported.

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
The later original loop at `0x1419ec550..0x1419ec583` visits the constructed
string vector in order, calls `0x141620280`, then `0x1415404e0`, stopping at
`0x1419ec795` on its first nonzero result. The latter converts the path and
queries metadata, rejecting a directory mode bit. The new
`probe_texture_candidate_loop.py` checks all-miss and first/middle/last-hit
paths with normalization and metadata answers substituted. Extension order and
actual file opening are still outside this controlled comparison.
`probe_texture_extensions.py` executes the three inline-string replacement
blocks directly: DDS `0x1419eb98c..0x1419eba1f`, BMP
`0x1419eba42..0x1419ebad0` and PNG `0x1419ebaf3..0x1419ebb81`.
Twenty-one cases confirm replacement of the last four bytes, including a DDS
input producing a PNG candidate; names of zero, one or four bytes stay unchanged.
The original blocks execute without stubs. Long-name
allocation, normalization and the caller's mode-dependent ordering are untested;
this evidence alone does not establish the active POL fallback policy.

The subsequent static trace identifies the `DRAPED_POLYGON` loader at
`0x1406ba930..0x1406bb875`. It dispatches material parsing through
`0x140c32190` at `0x1406bb15f`, then `0x140c32d60`. The base `TEXTURE` branch
calls `0x1419eb8c0` at `0x140c33097` with ECX=1. Bounded REA captures cover
`0x1406bb120..0x1406bb164` and `0x140c33020..0x140c3309c`; listings remain local.
`probe_texture_candidate_order.py` executes the native call-argument setup and
the mode-1 candidate assembly at `0x1419ebce3..0x1419ec014`. Two synthetic
nonempty-prefix cases observe DDS, PNG, BMP, KTX2 in that order. Concatenation
is substituted with an empty inline result; native vector moves and cleanup run
with preallocated capacity. Candidate contents are supplied, not derived by this
probe. Earlier extension and first-hit probes cover separate stages.

The fourth candidate removes the last dotted suffix and appends `.ktx2`, unlike
the first three four-byte replacements. Its string constant is at `0x1425e126c`.
Five additional cases execute `0x1419ebb94..0x1419ebce3`, including the original
copy routine without stubs, on a prepared inline string. Dotted, undotted and
empty names match; this gives 26 extension cases total. Long-name allocation
is still outside this comparison. POL resolution stops at the first existing
file even if decoding fails.
The port enables BMP decoding; it reports an error for a selected KTX2. No claim
is made about other texture modes, optional prefix replacements, filesystem case
semantics, native GPU decoding or complete material/visual parity.
A static scan of direct `call` sites finds 83 callers of `0x1419eb8c0`; 55 resolve
to mode 1, including the OBJ loader at `0x14089c220` (call `0x14089c30a`), the
TERRAIN `BASE_TEX`/`TEXTURE` loader at `0x1419a6400` and the POL branch. The preview
therefore uses one mode-1 resolver for OBJ, TER and POL. Other modes (3, 15, the
road and COMPOSITE_TEX callers) and FAC wall textures are not traced. The installed
game ships one KTX2 file (`moon_NML.ktx2`, BC1, 1024x1024, no supercompression),
unused by scenery. The port decodes level 0 of uncompressed-container 2D KTX2 files in
BC1, BC3 and RGBA8; other formats and supercompressed files return an explicit error.
The sample decodes to the expected lunar normal map. The UHWW render is byte-identical before and after this change.
The resolver compares ECX only against 1 (`0x1419ebcfb`, prefixed four-candidate
assembly) and dispatches `ECX-1` (0..6) through a jump table at `0x1419ec804`: mode 1
at `0x1419ec051`, mode 2 at `0x1419ec20b`, modes 3..7 at `0x1419ec270`; larger values
skip it. Those branches append further candidates under `custom terrain textures/`
or `custom object textures/` (selected by EDX == 2) after the package-relative ones.
Decompilation of `0x1419eb8c0` (Ghidra 12.1.4 headless, full analysis) shows that, in mode 1,
after the four package-relative candidates it appends `<pack root>/custom object textures/`
(or `custom terrain textures/` when EDX == 2) plus the DDS, PNG and BMP names, never KTX2.
The pack root comes from `0x1419f5950`, which locates `Custom Scenery/<pack>/` or
`Global Scenery/<pack>/` in the resource path. EDX is 3 for the OBJ loader (call
`0x14089c30a`), 2 for `TERRAIN` BASE_TEX/TEXTURE and `COMPOSITE_TEX`, and a variable for POL.
The port implements this (terrain kind for TER, object kind elsewhere). The installed game
has no such folders, so rendering is unchanged. Callers with EDX other than
the OBJ/TER/POL values are likewise untraced. By call-site scan, `COMPOSITE_TEX`,
`RUNWAY_TILE` and the other `0x140c2e..0x140c35` material callers also use mode 1.
Callers with other modes are the road loader (`0x140977fa0`, derived from a loop
counter), the `effects`/`blank.png` helper at `0x140f63110` (modes 3 and 15) and
`0x1408a3914` (15). The preview renders no road networks, so no rendered path uses
them. The sorted-name search and first-hit iteration were verified separately.
Runtime observation (reference X-Plane 12.4.3 started under Frida, hooking `CreateFileW` and
`NtCreateFile`, new flight at UHWW, scenery reload): first-attempt order of texture file
opens per base name is DDS, PNG, BMP, KTX2, with candidates absent from the directory
apparently skipped. Custom Scenery bases show (dds,png,ktx2) 122 times and (dds,ktx2) 111
times; default scenery shows (dds,png,bmp,ktx2) twice. No base shows PNG before DDS except
UI bitmap callers with other modes. This agrees with the mode-1 order implemented in the
port. The UHWW package's own textures were not opened in the observed window
(only its OBJ/POL files and `2_UHWW_Ortho` DDS/PNG candidates), so no UHWW-specific file
comparison exists. Library OBJ lookups also try `<package>/custom objects/<path>` after
`<package>/<path>`; this is a resource, not texture, resolver and is not ported.

FAC wall textures use the same material parser as POL. The FAC parser `0x14197f440` (entry
seen at `0x1419802e0`) calls `0x140c32190` with kind 5 at `0x14197f736` and `0x141980f7e`;
that parser reaches `0x140c32d60`, whose TEXTURE branch calls `0x1419eb8c0` with mode 1
(Ghidra call at `0x140c33097`) and the kind as EDX, i.e. the object-texture folder.
The port therefore resolves FAC walls with the same mode-1 order. In the resolver, mode 2
adds one `custom terrain textures/` candidate and modes 3..7 build `custom objects/`
resource-file candidates (decompiled `0x1419eb8c0`); they are not texture lookups used by
the rendered preview.
`tools/probe_texture_name_membership.py` executes the original comparison with
a controlled search-result pointer. Four inline ASCII cases confirm an exact
match succeeds with index zero, while changed extension, case and length fail.
The original byte comparison runs without substitution; the lower-bound search
result is supplied. This is not a filesystem, full search or fallback test.
The extended probe removes that substitution and executes the original search
as well. It checks the four comparisons above and 27 requests across empty,
single-record and five-record sorted collections, including first/last hits,
missing prefixes and names beyond either endpoint. Hit indices match synthetic
record positions. This supersedes the supplied-search-result limitation for
these inline-ASCII cases, but still exercises no filesystem or fallback policy.

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

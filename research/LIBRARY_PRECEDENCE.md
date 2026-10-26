# Scenery library export precedence

The 2026-10-26 preview-library correction replaces a single first-match map with
an index of candidate resources. It follows the published
[library.txt specification](https://developer.x-plane.com/article/library-library-txt-file-format-specification/),
not a native loader replay. Full original mod compatibility remains unproven.

Packages are supplied in the existing installation reader's order. Within that
order, the new index applies these rules:

- EXPORT and positive EXPORT_RATIO retain sibling definitions in their pack,
  then block definitions from lower packs.
- EXPORT_EXTEND retains eligible lower-pack candidates until a replacing pack.
- EXPORT_EXCLUDE also blocks subsequent definitions in the same pack.
- EXPORT_BACKUP is used only if no ordinary candidate exists, regardless of
  whether the backup package precedes the ordinary export.

The renderer still chooses the first candidate deterministically. Ratio values
are checked for positive finite eligibility but not used for weighted sampling.
At this stage conditional region/dataref/season selection was not implemented: legacy exports
under a REGION directive still enter the preview as if unconditional; seasonal
export commands are not read. This is an explicit remaining fidelity problem,
not general library compatibility. Default directory sorting and current custom
pack order are also not native-verified library ordering.

References outside the owning package and missing physical files are excluded
from the preview index. This is lexical path validation, not symlink containment
or a claim about the original's missing-file error behaviour. Relative paths with
spaces continue to resolve. No installed resources are copied into the project.

Synthetic multi-pack tests cover higher backup versus lower export, extension
across packs, sibling variants, normal replacement, same-pack exclusion,
backup-only lookup, missing files, zero ratio and parent-directory paths.
The existing installation test checks custom priority and paths with spaces.
These tests establish the port's documented candidate rules, not original
binary-loader equivalence. Ignored native comparisons remain separate evidence.

Scenery estimates are reviewed and unchanged: this corrects part of the existing
preview loader, while native selection, conditions and matched visual evidence
remain absent. Next: resolve library conditions and randomized selection against
the reference, alongside completing terrain and facade rendering.

## Runtime integration and instance storage

The first KSEA run after correcting export selection reached 5153 instances in
157 resource batches but failed GPU validation: CPU-expanded repeated geometry
created a 4778295360-byte vertex buffer, above the device's 268435456-byte limit.
This failed run is recorded locally; it is not a successful render.

The scene now retains one vertex array per scenery resource plus translation and
Y-rotation per placement. A GPU instance vertex stream applies the same transforms
to positions and normals. Baked terrain and animated aircraft use an identity
instance. Bounds are computed from transformed resource bounding-box corners
without materializing every repeated vertex. This is our engine's storage design,
not a claim about the original renderer. Oversized individual meshes now return
an explicit error instead of proceeding to GPU buffer validation.

LOD selection, native material behaviour and sorting individual transparent
instances remain unfinished. These optimizations preserve the existing preview
scope and do not establish original-equivalent graphics.

The repeated KSEA run completed successfully on RTX 2080/Vulkan after instancing:
5153 nearby instances, 157 batches, one unresolved child path and one failed empty
OBJ placeholder. The exported image is a port-only runtime check, not a paired
original capture. Workspace formatting, Clippy with warnings denied and ordinary
workspace tests pass; ignored native tests are not included in those results.

![KSEA after library selection and instancing](../site/assets/ksea-library-20261026.png)

## Geographic DSF lookup

The subsequent geographic lookup supplies the airport datum to the library index.
REGION_DEFINE names are retained across packages; REGION_RECT unions inclusive
integer DSF cells, REGION_ALL covers the world, and REGION_BITMAP replaces the
rectangles with a 360x180 mask. Bitmap rows are inverted from image north-first
to geographic south-first indexing. Images are decoded once per definition;
nonzero grayscale pixels are accepted as inside for the documented black/white
format. REGION changes the active condition; no REGION means world-wide exports.
Conditional exports that do not match do not block the world-wide fallback.

This is still public-format implementation, not native-recovered behaviour.
REGION_DREF is not evaluated: regions containing it are ineligible in this
preview, an explicit unsupported case. Missing/invalid bitmap masks similarly
do not match. A location-free catalog retains the previous unfiltered preview;
the airport renderer now requests a geographic lookup. All nearby placements
currently share that datum, so a scene crossing a DSF border needs per-tile
lookup. Seasonal selection and random/ratio sampling remain unfinished.

Synthetic tests cover inclusive upper cell edges, negative coordinates,
outside-region fallback after EXCLUDE, bitmap orientation, replacement of
rectangles, and unsupported dataref conditions. Estimates remain unchanged;
these checks do not establish original mod compatibility or verified scenery.

KSEA rendered successfully with geographic lookup: 5153 instances, 157 batches,
one unresolved child and one failed empty OBJ, on RTX2080/Vulkan. Its counts are
unchanged from the previous capture; this does not prove every regional choice.
The capture exercised per-query bitmap decoding; the final code caches decoded
masks per region definition, with the same pixel rules covered by tests.
Workspace fmt, Clippy and ordinary tests pass; ignored native tests were not run.

![KSEA with geographic lookup](../site/assets/ksea-regions-20261026.png)

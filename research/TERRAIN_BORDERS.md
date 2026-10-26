# TER border-mask integration

The 2026-10-26 renderer now retains BORDER_TEX/BORDER_TEX_WRAP and
COMPOSITE_BORDERS. Border ST follows the documented DSF plane order: after the
five geometry/normal planes for projected base textures, or after the explicit
base ST pair for unprojected ones. Tests cover both layouts, missing coordinates,
wrap and composite flags. Missing border coordinates currently use the full-mask
sample and are reported; this is an unresolved fallback, not native error parity.

The GPU uses a separate per-vertex border ST buffer, a linear grayscale mask
texture and material flags. Ordinary borders modulate final alpha. Composite
borders compare the base texture's raw alpha against the mask; discarded fragments
reveal underlying physical terrain. NO_ALPHA is applied after that comparison,
so it does not erase the base noise used for the border test. Reference:
[published TER format](https://developer.x-plane.com/article/terrain-type-ter-file-format-specification/).
This is public-format implementation, not recovered original shader arithmetic.

Installed-file evidence: textures12/border/soft.png is128x64 luminance+alpha;
its luminance spans0..255 while alpha is255 throughout. The grayscale channel
therefore carries the mask in this resource. The installed apt_dry_01 DDS is
1024x1024 RGBA with alpha39..217, retaining a noise pattern. The renderer samples
mask red in linear space, and keeps base alpha until the border comparison.
Channel selection and exact filtering still require native reference validation.

Source triangles are grouped separately by overlay flag. Physical groups precede
overlay groups; overlays receive a0.005m visual lift. This is a preview depth
policy, not the original patch ordering/polygon-offset rules. Ordinary masked
groups use the existing transparent pipeline, with its known incomplete sorting.
Composite-mask groups keep depth writes and discard. All patch LODs still render.
Base textures use generated mipmaps; mask textures currently use a single linear
level. Preserving native mip/alpha-test coverage remains unresolved.

KSEA's unchanged39303 triangles now form143 textured batches, including67 masked
overlays. Splitting physical/overlay resources increases the batch/texture count;
it does not indicate more source geometry. The GPU generates1472 base mip levels.
The RTX2080/Vulkan render has visible changes at some boundaries but still has
hard transitions and patch-like shading. It is a port-only capture, not a paired
original comparison.

The repeated GPU process returned exit0; its report contains no missing border
coordinates or failed base UV evaluations for this KSEA view. Workspace fmt,
Clippy and ordinary tests pass. These checks do not run ignored native replays.

![KSEA source terrain with border masks](../site/assets/ksea-ter-borders-20261026.jpg)

Still missing: COMPOSITE_TEX color/noise/parameter blending, slope/heading effects,
normal/decal/night/weather/water shading, original UV phase and depth selection,
native collision/surface semantics and matched visual/motion evidence. The change
uses the shared live source-terrain builder; no original assets/code are copied.
Other limitations in TERRAIN_MATERIALS.md and LIVE_TERRAIN.md remain applicable.
Coverage estimates are reviewed and unchanged: masks alone do not establish
original rendering or mod compatibility.

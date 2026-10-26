# Installed POL base-texture integration

The airport scene now reads installed850 DRAPED_POLYGON resources referenced by
Global Airports DSF polygons. Library resolution includes enabled custom exports,
so original-format AEP pavement/ground-sign resources participate without package
conversion. This is partial visual loading, not complete original mod support.

The parser supports base texture, wrap/nowrap, NO_ALPHA, SCALE and LAYER_GROUP.
It reports unsupported directives. For a straight contour, DSF parameter65535
uses its explicit ST planes; other parameters rotate projected coordinates,
including the X-Plane12 sub-degree encoding. Texture projection is tied to this
preview's local airport coordinates; native texture phase is unverified.
References: [POL format](https://developer.x-plane.com/article/draped-polygon-polfac-file-format-specification/)
and [DSF usage](https://developer.x-plane.com/article/dsf-usage-in-x-plane/).

Contours with holes or Bezier planes are explicitly skipped. Simple contours are
triangulated and subdivided to50m or depth16, interpolating explicit ST before
sampling source terrain height. Failed/incomplete triangulations are reported.
The parser accepts both the specification's NOWRWAP spelling and NOWRAP.

Layer placement currently uses the port's existing height scheme: source terrain
plus0.02m for terrain,0.13m for taxiways and0.23m for other groups, plus a millimeter
per layer offset clamped to±30. This is an approximation, not native layer groups
or polygon offset. Material alpha uses the existing transparent pipeline unless
NO_ALPHA forces opacity. Within-group alpha order, geometric normals and layer
ordering still need original comparisons. POL SURFACE does not yet change contact
type, friction or hardness; normal/decal/night/weather shading is unsupported.
Airport contacts remain those described in LIVE_TERRAIN.md.

Tests cover projected texture scale,90-degree heading, explicit ST interpolation
through subdivision, sloping terrain height, nowrap/NO_ALPHA and malformed scale
or missing ST. Workspace formatting, Clippy and ordinary tests check the port;
ignored native replays remain separate. No original resource is copied into the
repository, and no paired reference render establishes visual parity.

The coverage estimates are reviewed and unchanged: this extends the existing
partial scene loader while holes, curves, original material/layer semantics,
surface physics and native/visual validation remain unfinished.

The installed KSEA scene build reports752 polygons in16 resource batches and46
skipped footprint/resource cases. These are nearby preview counts, not the
whole DSF's polygon counts; they include pavement and ground-sign resources.
The RTX2080/Vulkan offscreen run completed at1280x800; the image shows sharp
material transitions, repeated/grainy textures and remaining untextured areas.
It is a port-only inspection of the integrated airport scene, not paired proof.

![KSEA with installed POL resources](../site/assets/ksea-pol-20261026.jpg)

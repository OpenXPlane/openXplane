# Type-2 facade mesh input and GPU inspection

The parser reads installed version-1000 FACADE files without converting them to
OBJ. It retains floor names and roof heights, ordinary/curved segment records,
mesh group/far-LOD/cut metadata, eight-component vertices, variable-length IDX
lists, wall rules and spelling segment indices. Wall/roof base textures,
roof texture scale, ring/graded and no-mesh flags are retained. Unknown directives
are reported, not silently presented as implemented.

This follows the [published FAC format](https://developer.x-plane.com/article/x-plane-10-facade-fac-file-format-specification/).
It is not a native-loader recovery. Type-1 facades are rejected. Mesh declarations
are checked for bounded counts, complete arrays, triangle index counts, valid
indices and finite input numbers; spellings are checked against ordinary segments.
Tests cover materials, roof scale, mesh coordinates, spelling, invalid indices,
missing vertices and nonfinite input. Native error behaviour is unverified.

```sh
openxplane fac-render "<X-Plane>/Global Scenery/Global Airports/KSEA_objects/STerm_R1.fac" 0 4 segment.png
```

This renders ordinary segment 4 from floor 0 directly in file coordinates.
The installed resource loaded one mesh with bounds (-1,-1.146,-10) to
(0,10.19,0), including its actual base texture. The GPU run uses RTX2080/Vulkan.
It reports TEXTURE_LIT as unsupported. No geometry, texture or source content is
copied into the repository; the screenshot is rendered output.

![Original FAC segment rendered by openXplane](../site/assets/ksea-fac-segment-20261026.png)

The command inspects all mesh groups at once without LOD culling or original
glass/material ordering. It does not generate a facade roof, attached objects,
terrain alignment or wall fitting. It also does not implement the NO_WALL_MESH
flag as a scene: the explicit purpose is to inspect the declared mesh itself.
The airport scene still lacks these facades. This is a port-only diagnostic
capture, not paired original visual proof.

Important next investigation: the published text describes wall length along X,
but the installed KSEA segments extend predominantly along negative Z with
outward normals toward positive X. Before fitting walls, resolve the transform
against original/native behaviour; do not infer it from the documentation alone.
Then connect floor selection, DSF wall/Bezier planes, spelling fits, corners,
roof materials and collision to the running airport scene.

Workspace fmt, Clippy and ordinary tests validate the port changes. Ignored
native tests remain separate. Estimates are reviewed and unchanged: input loading
and one diagnostic mesh do not establish facade rendering or mod compatibility.

# First Cessna viewer - 2026-10-06

The reference EXE and content are described in [BASELINE.md](BASELINE.md). OBJ8 is implemented according to the
[official specification](https://developer.x-plane.com/article/obj8-file-format-specification/).

## Result

The viewer loads the chosen seven objects of Cessna_172SP.acf and the DDS textures. In the first static state
72,359 triangles are shown. A 1280×800 image and a native window were checked: three frames were presented
successfully through Apple M4 / Metal. The camera rotates with the mouse and zooms with the wheel; there is a
reset and correct handling of window resizing.

Image: `research/local/cessna-preview.png` (local content, excluded from Git). The original models and textures
were not modified by the tools.

## Diagnostics of the full aircraft

Update: both obstacles described below were removed after studying the EXE. The current audit parses 42 of 42
objects of the standard Cessna; details and the results of the other variants are in
[ACF_LOADING.md](ACF_LOADING.md). The result of the first audit is kept below.

The first audit parsed 40 of 42 attached OBJ files successfully: 191,096 declared triangles. The base textures
of all these objects were found. Two more objects need research:

- `vor1_gs_ag.obj:675`: the rotation key contains `-2.5.000000`. Strict number parsing rejects this entry; it
  cannot yet be considered known what value the original engine obtains.
- `lights.obj:9`: a static ANIM_trans with six arguments, then ANIM_rotate with five; the dataref ranges are
  absent. Further on there are unsorted rotation keys. This variant is not yet supported by our reader.

Both objects are not in the current set of the external view. The audit lists all unsupported commands and
returns code 2 on errors in individual files. Full local report: `research/local/cessna-mesh-audit.txt`.

## Start of the X-Plane.exe analysis

A search for RIP-relative LEA candidates referencing ACF strings was added. The function bounds are taken from
the `.pdata` table. These candidates must be checked with the disassembler: a byte match alone does not prove an
instruction boundary.

In the current build llvm-objdump confirmed the following instructions:

| Instruction VA | String | String address VA |
|---|---|---|
| 0x141ab9eb6 | PROPERTIES_BEGIN | 0x1427c5e50 |
| 0x141ab9eeb | PROPERTIES_END | 0x1427b4ad8 |
| 0x141aad724 | _afl_file_1 | 0x1427bead8 |

The first pair is in a function with the `.pdata` range `0x141ab9920 .. 0x141abdd3c`. After the markers are
handled a call to `0x141a972a0` is visible, in which the search found a reference to `_m_empty`. This is a point
for further research into ACF property handling; the purpose of the whole function is not recovered yet. The
aerodynamics algorithms are not yet analysed.

Local artifacts: `acf-xrefs.json`, `acf-load-disassembly.txt`, `airfoil-properties-disassembly.txt` in
`research/local/`. The names of the nearest exported symbols printed by llvm-objdump are not considered names of
the functions studied.

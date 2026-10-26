# Original terrain shader archive investigation

The installed shader archive is
Resources/shaders/bin/spv/terrain.xsa,79600866 bytes, SHA256
`80e9afb9af277567e82c26fbfd5ce410cbeb970ce0d1ee27ca3408fafddeafa4`.
REA6.7 inspect-artifact identifies it as ZIP through the safe artifact graph
provider. The retained local evidence explicitly does not execute the archive or
establish format-specific shader semantics. Original bytes remain unchanged.

The ZIP contains4614 SPIR-V members totaling204074240 uncompressed bytes.
The companion terrain_mapping.xsv starts with the four-byte signature XSV6 and contains resource names,
including base, composite, border, noise and seasonal variants. Its full mapping
format and runtime variant selection are not yet recovered.

## Recovered mapping prefix

REA bounded observations against the reference executable identified the header
check at 0x14103fafa and its call at 0x14103fb21 to reader 0x14105e350.
The reader skips four bytes and calls 0x141060320. That helper assembles a
little-endian 16-bit count, then reads one-byte lengths and the corresponding
string bytes. The reader subsequently assembles two little-endian 32-bit words,
reads one flag byte and another one-byte-length string. Their complete runtime
semantics are not established. The older description `XSV61` mistakenly included
the low byte of the name count in the signature.

The new bounded `tools/inspect_xsv_mapping.py` inspects this prefix without
interpreting the remaining variant payload. On the installed terrain mapping
(996152 bytes, SHA256
`ae0aa1633d7eef4f67699f1d89b5c391a258ea2ca46f2465e3b9a150e23cf4ca`),
it reports 49 names ending at offset803, words829440 and1, flags3 and entry
string `main`. The opaque payload starts at817. Independently inspected smaller
font and SSR-mesh mappings have matching prefix structure.
These observations do not prove that word829440 is a variant count, identify
the active KSEA shader or establish shader equivalence.

```sh
python tools/inspect_xsv_mapping.py "<X-Plane>/Resources/shaders/bin/terrain_mapping.xsv"
python -m unittest discover -s tools -p test_inspect_xsv_mapping.py
```

The inspector limits input to64MiB, checks every prefix read and restricts display
strings to ASCII. Synthetic tests cover byte order, empty and nonempty names,
payload boundaries, every truncation of a complete prefix, bad signatures and
unsupported display encoding. This is inspection tooling, not a runtime loader
or an emulation comparison. Original mappings and REA listings stay local.

The new standard-library-only inspector reproduces archive digests, member
digests, entry execution models, debug resource IDs and descriptor decorations.
It checks ZIP CRCs and instruction framing, with bounded archive/member sizes.
It does not emit shader source, extract members, specialize constants, execute
shaders or provide full SPIR-V validation. Reports from the original installation
belong under ignored local research, not version control.

```sh
python tools/inspect_spirv_archive.py "<X-Plane>/Resources/shaders/bin/spv/terrain.xsa" --resource tex_comp --limit 3
python -m unittest discover -s tools -p test_inspect_spirv_archive.py
```

There are2256 candidates naming tex_comp. The smallest is terrain_493.spv,
5736 bytes, with a fragment entry point and resources tex_base_texture,
tex_base_sampler, tex_comp, tex_comp2 and tex_bord. This only identifies an
analysis candidate; it does not show that KSEA uses it.

Local instruction tracing observes samples of base and tex_comp at the same
input coordinates, another sample of tex_comp2 and a border sample. That is not
a recovered composite-color formula. In this candidate the border sample's
alpha component participates in later arithmetic, whereas the current preview
uses grayscale red based on the inspected soft.png resource. This discrepancy
requires tracing resource upload/swizzle and variant selection; neither channel
is proven to reproduce original behaviour by these observations alone.

A local Naga29.0.4 analysis project, outside the workspace build, failed on
SpecConstantOp. A separately identified derived copy replaced the recorded
BLEND_MODE default7 and its equality-to3 expression with ordinary constants.
That copy is not the original shader. Parsing still failed with InvalidId6367
on the combined-sampler path. Neither attempt yielded a validated WGSL program
or numerical comparison, and no translated original source is distributed.

Next: decode the XSV variant mapping, identify a live KSEA shader variant and its
descriptor/uniform inputs, resolve image channel handling and trace arithmetic
to the final color. Only then integrate recovered composite behaviour and compare
controlled original/port outputs. Current COMPOSITE_TEX color blending remains
unimplemented; TERRAIN_BORDERS.md describes the existing public-format preview.

The synthetic inspector tests exercise named resources, entry models, descriptor
bindings, archive filtering and malformed/truncated instructions. They validate
the tool's scope, not shader equivalence. Estimates are reviewed and unchanged;
whole-game, original-mod and visual parity remain unproven.

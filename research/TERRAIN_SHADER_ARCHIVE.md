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

### Serialized resource layouts

The full reader's PE unwind range is0x14105e350..0x14105f675. A further REA
observation covers that range and the record helper0x14105dec0..0x14105e32e
(stopping before its jump-table data). Following the entry string, the reader
consumes a byte count limited to four groups. Each group has a kind byte, field
count byte and two bytes per field. It then reads a little-endian 32-bit layout
count. Each layout has a byte resource count and nine bytes per resource: four
raw fields, a little-endian 16-bit name index and three trailing fields.
The native helper checks the name index against the name table.

`--layouts` enables inspection through this boundary. The terrain mapping has
three groups and one layout with45 resource records, ending at offset1324.
The serialized resource names include base, composite, border and material
inputs. Field meanings are deliberately left numeric: the native helper also
changes some resource classes according to backend state and computes offsets;
the inspector does not reproduce those operations. Layouts are not a list of
active resources for a particular draw. No live variant or original-equivalent
material arithmetic has been established.

Synthetic layout tests cover field preservation, name resolution, every
truncation, out-of-range name indices, excessive group counts and impossible
layout counts. Real font, SSR-mesh and terrain layouts were read locally.
Rust workspace formatting, clippy and tests remain required; inspection results
do not justify increasing implemented or verified renderer percentages.

### Complete serialized variant traversal

The next reader stages consume a 16-bit auxiliary-record count. Each record has
a kind and item count, followed by key/operation pairs. Operations1 and3 skip
two additional bytes; operation4 skips three; others skip none. These are
reported as `conditions` for inspection; their complete runtime semantics are
not recovered. The observations are at0x14105e8ab..0x14105e9ee.

The first header word bounds the outer loop at0x14105ea88..0x14105f426. Each
slot begins with a presence byte. Present slots contain a 32-bit layout index
and four16-bit auxiliary indices, with65535 as the absent sentinel. With flags
bit0 set, a count byte and four bytes per attribute follow. For every repetition
bounded by the second header word, the reader then consumes as many stage
records as the population count of the flags byte. This happens on the bit0
path too: its native state construction falls through to the stage-ID reader.
Stage kinds1 and2 carry one32-bit word; kind4 carries four. All these words are
little-endian. The relevant reads are at0x14105eac6,0x14105ebc0,
0x14105ecbb and0x14105f1c0..0x14105f29b.

`--variants` traverses this structure; `--module ID` filters the displayed live
samples by their first stage word. Inspection of terrain consumes all996152
bytes:829440 slots,4618 present,132 auxiliary records and4614 distinct primary
stage IDs covering0..4613. The archive has4614 correspondingly numbered members.
This supports interpreting the primary stage words as archive module IDs;
it is not a live draw observation.

For candidate493, two slots reference stage kind2 ID493:13465 and39385.
They pair it with kind1 IDs103 and968 respectively and share auxiliary indices
3,67,65535,65535. Neither slot is established as active for KSEA. The inspector
does not reproduce native resource remapping, auxiliary expansion, bit0 state
construction or the caller's selection of a slot.

```sh
python tools/inspect_xsv_mapping.py "<X-Plane>/Resources/shaders/bin/terrain_mapping.xsv" --variants --module 493 --limit 3
```

A local survey traversed all95 top-level `*_mapping.xsv` files in the installed
shader bin directory without errors or trailing bytes. Font and SSR-mesh yield
six and three unique IDs respectively, matching their archive member counts and
number ranges. This is structural consistency evidence, not a shader execution
comparison. Synthetic tests now include absent/live slots, both graphics and
compute stage records, auxiliary extra-byte lengths, sample filtering and limits,
unknown stage rejection and every truncation of a complete variant fixture.
Rendering estimates remain unchanged pending runtime integration and comparison.

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

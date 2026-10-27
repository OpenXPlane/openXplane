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

### Terrain registration observation

REA confirms the constructor0x14176e120..0x14176eb6d registers the seven-byte
name `terrain` from0x1425052b8. The adjacent0x14176db40 constructor registers
`fake_terrain`; a separate BLEND_MODE reference at0x14086bd2d belongs to particle
configuration. Neither is evidence for the real terrain variant selection.

`tools/probe_terrain_shader_registration.py` runs the original terrain constructor
and record helpers0x1409168d0 and0x1417697a0 in Unicorn. It checks the executable
SHA256 before execution and guards native execution to these three unwind ranges.
The base constructor, string copy/assignment, vector assignment, destruction and
free are substituted. The synthetic base provides capacity for two stage bytes;
the three vector assignments are intercepted to inspect their temporary records.
The probe checks the returned object, registered name and all three captures.
This is a bounded synthetic observation, not a live GPU or complete constructor
equivalence test. Original instructions and resources are not emitted.

The ordered range records captured at object offset0x58 are:

| Parameter | Recorded range |
| --- | ---: |
| RENDERING_MODE | 5 |
| GEOM_MODE | 6 |
| USE_SHAD | 2 |
| TONE_MODE | 2 |
| BORDER_MODE | 3 |
| TEX_MODE | 18 |
| PROJ_BASE | 2 |
| SEASONAL | 2 |
| WANT_CSM | 2 |
| WANT_SOFT_SHADOWS | 2 |
| ALPHA_TO_COVERAGE | 2 |

Their product is207360, whereas the XSV has829440 slots. The factor of four
is unresolved; these ranges alone do not establish index encoding. A second
capture at0x1b0 contains ALPHA_MODE, HAS_DECS, HAS_NORM_DECS, NITE_MODE and
BLEND_MODE, each with a zero flag byte. The native helper stores a byte here,
unlike the 32-bit range in the first record helper. The third capture at0x308
names six attributes: a_instance_tint, a_lod_fade, a_night_level, a_color,
a_texcoord2 and a_season. These are declarations, not observed draw inputs.

```sh
python tools/probe_terrain_shader_registration.py "<X-Plane>/X-Plane.exe"
```

Next trace the caller's range expansion/index arithmetic and reconcile the
factor of four before assigning parameter values to slots13465 and39385.
No renderer behaviour or compatibility estimate changes from this observation.

#### Native base initialization follow-up

The probe now executes the base constructor0x14068e2c0..0x14068e38f and its
leaf helper0x14103cdb0..0x14103cdc8 instead of substituting base initialization.
REA bounded listings confirm the base initializes inline vectors and the leaf
clears an empty record. The scratch object remains zero-initialized before the
original code runs; container assignment and CRT substitutions listed above
remain. The probe checks the stage vector points at object+0x48 and contains
exactly two bytes,1 and2. The three captured declaration lists and207360 range
product remain unchanged. This removes one synthetic assumption but does not
resolve the factor of four or prove live draw behaviour.

The sole confirmed call reference to the terrain registration constructor is
at0x14176cce5, within0x14176cb30..0x14176ce76. REA confirms the caller passes
the registration result in RDX to an indirect manager call at0x14176ccf7 and
stores its result at0x1460ac238. The indirect target and its runtime state still
need tracing. No original disassembly or resource data is distributed.

#### Recovered slot arithmetic

REA confirms0x14104f0a0..0x14104f16a computes a mixed-radix slot. Its RCX
shader object supplies range records at0xf88..0xf90, each40 bytes with the radix
at+32. RDX supplies parameter digits through its first pointer. When the shader's
32-bit field0x112c is zero, the function first includes R8's low byte with radix2
and the state byte at RDX+0x80 with radix2. With a nonzero field, it skips both.
Their live meanings are still unknown. It then accumulates each parameter digit
times the running product of preceding radices, using32-bit wrapping arithmetic.
This explains the previously unresolved factor of four: mode0 has capacity
4*207360=829440. It does not establish the caller's actual parameter values.

`tools/shader_variant_index.py` implements the recovered arithmetic for analysis
and decodes slots under an explicit prefix assumption. Its encoder validates
valid digit ranges; the native arithmetic itself does not perform those checks.
`tools/probe_shader_variant_index.py` compares it with the original function on
1128 controlled inputs: zero, maximum, each maximum digit in isolation and128
seeded random states, for both field modes and all four binary prefix pairs.
No calls or stubs occur inside the index function. Declaration recovery still
uses the substitutions disclosed by the registration probe. Synthetic tests
exercise exhaustive small-radix round trips, invalid inputs and32-bit wrapping.

Under mode0 and the observed terrain declaration order, slots13465 and39385
both decode to prefix[1,0], RENDERING_MODE1, GEOM_MODE1, BORDER_MODE1 and
TEX_MODE9. All other digits are zero except PROJ_BASE, which is0 and1
respectively. This ties candidate493 to a precise combination of declarations;
the meanings of TEX_MODE9 and BORDER_MODE1, live selection, uniform values and
resource upload remain to be recovered before renderer integration.

```sh
python tools/probe_shader_variant_index.py "<X-Plane>/X-Plane.exe"
python -m unittest discover -s tools -p test_shader_variant_index.py
```

#### Material-to-digit preparation

A reference to the terrain shader object at0x14176ae5b leads to the material
preparation call0x14176ac83. REA confirms its callee
0x14176b3f0..0x14176b795. The new optional emulation probe executes that callee
through the exclusive boundary0x14176b757, immediately before validation. It
substitutes texture-presence queries, vector resize and one feature query,
guards native execution to the observed range and checks the resulting11 digits.
It does not claim complete material preparation or successful variant selection.

The180 checked synthetic cases span rendering digits0..4, texture digits0..17
and both projection states. Geometry digit1 and border digit1 are supplied;
other feature state is absent. The observed mapping includes material+0x44 to
TEX_MODE, material+0x224 to BORDER_MODE, material+0x1b4 to GEOM_MODE and
material+0x220 != -1 to PROJ_BASE. Rendering mode is supplied by R9. Caller stack
arguments supply USE_SHAD, TONE_MODE and shadow state. Material+0x198 is set to1
in these cases: when it is zero, rendering mode1 substitutes TEX_MODE14 and
clears the border digit. The actual meaning of this field and how original TER
loading fills these offsets remain unresolved.

The probe uses zero-initialized scratch material state and absent texture-query
results, so it does not cover texture-format-dependent alpha, decals, normal
decals, seasons, weather or the validation stage. The observed integer offsets
are reference-build-specific, not a public mod ABI or a port runtime layout.

```sh
python tools/probe_terrain_material_digits.py "<X-Plane>/X-Plane.exe"
```

Next trace the original TER loader's assignments to these fields, including
COMPOSITE_TEX and COMPOSITE_BORDERS, then verify with loaded material state.
Current COMPOSITE_TEX rendering remains unimplemented and estimates unchanged.

#### DETAIL corrects the candidate interpretation

REA bounded observations in the TER loader identify the directive check at
0x1419a84b9 targeting0x14279f2a0 (`DETAIL`). Its accepted branch at0x1419a84d1
reads one float, computes its reciprocal through double precision and stores the
rounded float twice, followed by two float1 values. At0x1419a8514 it stores
texture mode9 into the local material at frame+0xd4. This is material+0x44
relative to the material beginning at frame+0x90. The loader's creation calls at
0x1419abb6f and0x1419abc14 pass that material block onward.

Consequently candidate493, selected earlier because it names tex_comp resources,
is linked to DETAIL mode9, not established as the program for COMPOSITE_TEX.
Resource names alone were insufficient. The earlier analysis candidate remains
useful for DETAIL investigation but must not drive generic COMPOSITE_TEX
implementation. No such integration has been made.

The separate COMPOSITE_TEX check at0x1419a7380 targets0x142585910. Its texture
load branch stores mode5 at0x1419a75cd. This is an intermediate assignment:
later directives and finalization can change the mode, so mode5 alone does not
establish the final composite material's active shader.

`tools/probe_terrain_detail_mode.py` checks six positive scales in the original
arithmetic block through an exact exclusive stop. It substitutes only the float
read, supplies a synthetic frame and XMM10=double1, and confirms the reciprocal
fields and mode9 assignment. Token recognition, full loading and draw state are
outside this probe. Original bytes and listings remain local.

```sh
python tools/probe_terrain_detail_mode.py "<X-Plane>/X-Plane.exe"
```

Next identify the final mode after actual COMPOSITE_TEX/PROJECTED combinations,
then choose matching modules using the recovered index. Renderer percentages and
original-mod verification remain unchanged.

#### Projection, borders and validation narrow the candidates

REA confirms the accepted PROJECTED block at0x1419a7e23..0x1419a7e59 reads
two integers, converts them to floats at frame+0x290/+0x294 and sets
frame+0x2b0 to1. It does not overwrite the texture mode at frame+0xd4.
The extended detail probe checks two positive scale pairs and preserves a
supplied mode5 through this block. This is still an isolated block observation,
not a complete directive-order or finalization comparison. It also exposes an
unresolved loader difference: our PROJECTED parser currently accepts floats.

The COMPOSITE_BORDERS check at0x1419a8370 targets0x14279f260; its accepted
branch at0x1419a8380 stores2 into frame+0x2b4, corresponding to material+0x224.
Thus the previously chosen border digit1 is not the composite-border case.

REA also identifies the native validator0x14176eb70..0x14176ec94. The material
probe now invokes it separately on the180 prepared states, without stubs in the
validator. It accepts26 and rejects154. The older preparation-only checks were
never evidence that all180 combinations could draw. In particular geometry
digit1 does not accept composite texture modes5/6 under these conditions.

With an explicitly hypothetical mode0-prefix[1,0], GEOM_MODE0, BORDER_MODE2,
TEX_MODE5, PROJ_BASE1 and other feature digits zero, the XSV lookup gives:

| RENDERING_MODE | Slot | Fragment primary ID | Named texture observations |
| ---: | ---: | ---: | --- |
| 0 | 34081 | 1202 | base, composite, border, material and noise among others |
| 1 | 34085 | 225 | border only |
| 2 | 34089 | 1203 | base, composite, border, material, BRDF and probes among others |
| 3 | 34093 | 1204 | base, composite, border, material, BRDF and probes among others |

These are reproducible lookups using the recovered index and mapping inspector,
not observed KSEA selections. Rendering mode1's program225 does not even name
base/composite textures; mode names alone do not identify a color pass. The new
candidates1202/1203/1204 are better-scoped leads for composite color analysis
than the small DETAIL candidate493. Live geometry/rendering modes, prefix state,
uniform values, full TER finalization and resource upload remain unresolved.

#### Composite candidate1202 sample provenance

The archive inspector now reports implicit/explicit LOD image-sample offsets,
result and coordinate IDs, and named handle roots through Load, CopyObject and
SampledImage. Unknown handle operations remain unresolved; cycles/depth are
bounded. This is structural provenance, not an evaluator, control-flow reachability
analysis or confirmation that a named texture is bound at runtime. `--member`
selects an exact archive member while preserving whole-archive inventory.

Candidate terrain_1202.spv is33316 bytes, SHA256
`6c1161db1f9fd629d481b3e282c8535dc090d48e86371cb141d6edf69b6bd95b`.
It has18 sample instructions,13 implicit and5 explicit LOD. Every image handle
in this module resolves through the supported chain operations. Relevant samples:

| Byte offset | Named roots | Coordinate ID | LOD |
| --- | --- | ---: | --- |
| 0x297c | tex_base_texture + tex_base_sampler | 11951 | implicit |
| 0x2a10 | tex_comp | 16121 | implicit |
| 0x2a88 | tex_comp2 | 16122 | implicit |
| 0x2ae8 | tex_noise | 8935 | implicit |
| 0x2b9c | tex_noise | 8936 | implicit |
| 0x2c50 | tex_noise | 23247 | implicit |
| 0x2cf0 | tex_noise | 23248 | implicit |
| 0x69e8 | tex_bord | 14006 | implicit |

Local instruction inspection of0x2afc..0x30f8 shows arithmetic combining four
noise samples, an alpha extraction from the first composite sample, a smoothstep
weight and subsequent color mixing. Another weight uses material fields and an
eye-position input. This is a lead for an isolated arithmetic comparison, not a
recovered final-color claim: actual uniforms, texture channel interpretation,
sampling, specialization and downstream material/lighting remain unverified.
The earlier DETAIL493 arithmetic must not be substituted for this sequence.

```sh
python tools/inspect_spirv_archive.py "<X-Plane>/Resources/shaders/bin/spv/terrain.xsa" --member terrain_1202.spv --limit 1
```

Synthetic tests cover combined image/sampler roots, copy/load chains, descriptor
decorations, unresolved and cyclic handles, truncated samples and exact member
selection. They verify the inspector only. No original shader bytes or translated
source enter the repository; original-derived listings remain ignored locally.

#### Isolated composite arithmetic comparison

`tools/spirv_arithmetic.py` interprets only a caller-selected straight-line region
using CPU float32 arithmetic. It supports the needed constant, memory-access,
vector and GLSL.std.450 operations, rejecting unsupported executed instructions,
branches and unaligned range boundaries. It does not implement specialization,
derivatives, GPU filtering, complete SPIR-V semantics or validation. Memory and
sample callbacks are supplied inputs; this is not execution on the original GPU.

The build-identified1202 probe covers byte offsets0x2a9c..0x3100:78 instructions
and four substituted noise samples. For256 seeded synthetic cases it checks
geometric progression of noise coordinates, interprets the instruction stream,
and compares resulting RGBA and two mixing weights with an independently written
float64 formula. Maximum absolute error was1.5248492502006883e-7 with a declared
3e-6 tolerance. This establishes consistency of the CPU interpretation and the
formula under those inputs, not original sampler or rendered-pixel equivalence.

In the local material structure, let T0/T1/T2 denote tex_params/tex_params2/
tex_params3 and N_i the four supplied noise samples. The observed intermediate
calculation is:

- Coordinates start at baseUV*T1.x and multiply by T1.y for each later octave.
- Noise is the sum of (2*N_i.z-1)*T1.z*T1.w^i for i=0..3.
- Weight b is smoothstep(T2.x-T0.z, T2.x+T0.z,
  noise-2*T0.w*(firstCompositeAlpha-0.5)).
- The first RGB mixes firstCompositeRGB toward baseRGB by b, preserving the
  first composite's alpha.
- Weight d is clamp(T2.y*(-eyePosition.z)+T2.z,0,1). The intermediate RGBA then
  mixes that first result toward the second composite sample by d.
- Two later-use weights are b*(1-d) and (1-b)*(1-d).

Positive smoothstep widths were supplied. Zero/reversed edges, real uniform
population, actual texture formats/color spaces, specializations, downstream
material/lighting and live KSEA state remain outside the comparison. These are
intermediate values; they must not be presented as a recovered final pixel.

```sh
python -m unittest discover -s tools -p test_spirv_arithmetic.py
python tools/probe_terrain_composite_arithmetic.py "<X-Plane>/Resources/shaders/bin/spv/terrain.xsa"
```

Synthetic interpreter tests cover memory/sample substitution, shuffle and mix,
smoothstep clamping, and rejection of unsupported control flow/bad boundaries.
No translated original shader is distributed. Next recover uniform population
and sampling inputs before integrating this arithmetic into the port renderer.

#### Composite metadata reaches the TER loader

The port TER parser now retains COMPOSITE_TEX/NOWRAP, COMPOSITE_NOISE,
COMPOSITE_PROJECTED and COMPOSITE_PARAMS in a typed metadata record. Paths use
the existing TER separator normalization. Parameter arrays retain six mandatory
values and an optional seventh, which is zero when omitted. No missing-directive
defaults or GPU uniform mapping are inferred. Finite numeric inputs and field
counts are validated; warnings still identify all these directives as unsupported
by the renderer. This changes content ingestion, not the rendered material.

REA confirms the common parameter block0x140c2e780..0x140c2e850 reads six
floats into offsets0x20..0x34 of its output record and writes the optional float
at0x38, or zero when absent. The optional emulation probe starts after directive
recognition and substitutes numeric/whitespace queries with a synthetic cursor.
Both six- and seven-value cases preserve the recorded field order. This does not
prove full tokenizer or shader-uniform equivalence.

The installed-content test explicitly run with OPENXPLANE_REFERENCE_ROOT read
3597 TER files in the top-level terrain10 directory,1213 declaring composite
textures. All parsed and their declared composite/noise files existed locally.
It does not cover every scenery pack or shader behaviour and is ignored in
ordinary CI without the supplied installation. Synthetic tests cover optional
values, last-directive precedence, nowrap, paths and malformed/nonfinite values.

```sh
python tools/probe_composite_parameter_fields.py "<X-Plane>/X-Plane.exe"
# Set OPENXPLANE_REFERENCE_ROOT to the read-only installation before this test:
cargo test -p xp-scenery terrain::tests::installed_composite_metadata_and_resources --lib -- --ignored --nocapture
```

Next connect these inputs to the recovered original uniform packing and sampler
rules, then integrate and capture the running scene. Scenery/rendering estimates
remain unchanged: ingestion alone does not establish original mod compatibility.

#### Direct TER field-order confirmation

The earlier numeric probe covered a common composite reader whose observed
callers are0x140c32190 and0x140c322e0. That was not a complete TER-loader replay.
A direct check in the TER loader at0x1419a9f6a targets COMPOSITE_PARAMS.
REA now covers0x1419a9f64..0x1419aa016, and the extended probe executes its
accepted branch0x1419a9f82 through0x1419ab53b. It substitutes the numeric reader
and optional-token query. Six- and seven-number cases reproduce the common
reader's ordering and zero default, independently of the common-reader path.

In the TER local frame, the first six values land at0xf0,0xf4,0xf8,0xfc,0x100
and0x104; the seventh lands at0x108. The material begins at frame+0x90, so these
are material offsets0x60..0x78. The port retains these serialized inputs; an
additional regression assertion explicitly checks a supplied seventh value.
This establishes material field storage only. Uniform upload, default values
when the whole directive is absent, sampler behavior and complete live materials
remain unverified. Do not infer GPU uniform equivalence from adjacent fields.

#### Material uniform prefix

The material constant creation routine0x1417702b0 zeroes a0x1c0-byte buffer,
calls0x1417705c0 to fill it, creates a matching-sized graphics resource on a
cache miss and copies the filled bytes into it. Its helper at0x14176f680 also
caches the0x1c0-byte value. In terrain_1202.spv, the `u_material_data` member
offsets span0x000 through0x1b0, consistent with a0x1c0-byte block. The direct
original packing range0x1417705c0..0x1417709b2 is retained through REA; the
function's split PE unwind records do not cover the whole range as one entry.

The mode jump table selects0x14177090c for texture mode5. With a zeroed output,
its first three vectors are material fields:

| Uniform vector | Source material offsets |
| --- | --- |
| tex_params | +0x58, negated +0x5c, +0x60, +0x64 |
| tex_params2 | +0x68, negated +0x6c, +0x70, +0x74 |
| tex_params3 | +0x78, +0x7c, +0x80, +0x84 |

The mode5 branch also copies material+0x88..0x94 to tex_params4. Mode6 uses
different packing and leaves the third/fourth vectors untouched in this region.
`tools/probe_terrain_uniform_prefix.py` compares the first64 bytes against
original instructions on512 controlled raw-bit cases for both modes, including
signed zero, infinities and NaN bit patterns. It zeroes the destination as the
native caller does and substitutes a texture-presence query; it stops before
remaining fields and upload. These tests check copying and sign-bit operations,
not float shader results or the full constant buffer.

The direct COMPOSITE_PROJECTED branch0x1419a8316..0x1419a8360 reads two floats,
uses double-precision reciprocals rounded to float and stores them at material
offsets0x58/+0x5c. The extended parameter probe checks three positive scale pairs
against that block, substituting numeric input and supplying XMM10=double1.
COMPOSITE_PARAMS populates material+0x60..0x78 as documented above. Therefore the
port's `Composite::mode5_uniform_prefix` prepares the first three vectors when
both directives are present, taking the last three prepared material fields as
explicit caller inputs. These are not raw FAR directive arguments. Absent fields
remain unresolved rather than receiving invented defaults.

```sh
python tools/probe_terrain_uniform_prefix.py "<X-Plane>/X-Plane.exe"
python tools/probe_composite_parameter_fields.py "<X-Plane>/X-Plane.exe"
```

The helper is not connected to the renderer. Texture coordinates, remaining
uniforms, FAR conversion, shader selection and actual resource/color-space upload
still need recovery and live comparison. Composite rendering remains unsupported
and compatibility percentages remain unchanged.

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

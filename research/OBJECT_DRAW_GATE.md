# Preliminary aircraft object draw gate

Reference: X-Plane12.4.3-r2, build124311, PE SHA-256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Evidence and scope

REA bounded `inspect_with_objdump` inspected these unchanged-PE ranges:

- Gate `0x141a7b6a0..0x141a7b72c`, Evidence
  `ev_eb7c4a8420ca8e20063f4e573d811fce642ce6ff973fa7839048b867b59b5915`.
- Actual view helper `0x1407b4780..0x1407b47f7`, Evidence
  `ev_ee1b6877ade6c85f5e39e75580164505b765a814122c218c8ccf483da04831d8`.

The gate is inside `0x141a7ac80..0x141a7c4c6`. It reads the attachment array
from aircraft+0x60e0, a loaded-object pointer at record+0x90 and `_obj_flags`
at record+0x88 (record stride0x120). `tools/generate_attachment_gate_vectors.py`
executes original instructions to either the accept boundary0x141a7b72c or the
skip boundary0x141a7b931, then redirects only that observed terminal boundary to
an emulator sentinel. It does not execute the rest of the renderer.

All528 synthetic cases match the Rust accept/skip result. The original view
helper runs with no callee stubs. Inputs are controlled records/registers and
selected-cockpit pointer/state fields, not a live camera or plugin registry.

## Recovered conditions

- A null loaded-object pointer or object flag0x200 skips the object.
- Object flag0x100 requires pass flag0x100.
- Object flag0x2 requires at least one pass bit in0x18.
- Object flags in0x1800 select the actual view helper: a selected cockpit object
  must be loaded; mode0 then uses aircraft+0x888, modes1/4 accept, other modes skip.
  The helper chooses selected index+0x622c for modes0..3 and+0x6230 otherwise.
- Without0x1800, mode0 uses aircraft+0x88c; modes1/4 accept and other modes skip.

These are preliminary conditions, not a complete visibility function. Later
classification, dynamic kill-dataref evaluation, render-state setup and draws
can still reject or modify a record. Bit names beyond these observed conditions
are not claimed as recovered semantics.

## Runtime integration and actual renders

The static preview now enumerates `_obja/<index>/_v10_att_file_stl` in numeric
order and reads flags from each ACF record, replacing the seven Cessna filenames.
It applies the ported preliminary gate under an explicit preview policy: mode4,
pass0x19, zero animation/kill datarefs, main/cockpit enabled. It chooses the first
0x1800 record whose file exists as preview cockpit availability. This is a viewer
policy, not recovered original camera/selected-index production or observed pass
scheduling. A file existing is not a native loaded-object lifetime proof.

The common attachment loader now supplies translation in the ACF frame (zero CG
at this stage). `FlightModel::to_world` already subtracts the real CG, avoiding a
second subtraction. Nonzero rotation or attachment to a parent part is rejected;
the corresponding rotation/part runtime still needs recovery and integration.
Empty triangle batches are reported and skipped; absent base-texture declarations
use white for this preview. Dynamic cockpit textures, lights and materials remain
incomplete. Object bit0x2 feeds the preview transparent-mesh path; this does not
establish the original's complete transparent rendering behavior.

Executed on the installed content and NVIDIA RTX2080 Vulkan backend:

- Cirrus Vision SF50:14 triangle meshes loaded, three zero-triangle records
  skipped and three0x200 records excluded by the native gate.
- Cessna172 SP: all ACF records are considered; four0x210 records are excluded.
  Cockpit/interior objects can now contribute geometry, instead of only seats.
- A synthetic end-to-end scene test uses an arbitrary OBJ filename with a nonzero
  attachment translation. A missing0x200 object file is correctly excluded before
  file access; the accepted triangle's translated vertices are checked.

![Cirrus from ACF object records](../site/assets/cirrus-acf-20261026.png)
![Cessna from ACF object records](../site/assets/cessna-acf-20261026.png)

These are new port renders, not paired reference screenshots. The existing
original Cirrus water/cockpit capture uses a different camera and scene. GUI
control was not resumed for this work. Full materials, actual camera state,
rotation/part transforms, instruments, plugins and matched visual/motion evidence
remain required before claiming game parity.

Reproduce synthetic native evidence:

```sh
python tools/generate_attachment_gate_vectors.py "<X-Plane>/X-Plane.exe" crates/xp-acf/data/attachment_gate_vectors.tsv
cargo test -p xp-acf --lib attachment
```

## Progress rubric review

The new native loader evidence and live translation integration raise the ACF
self-assessment from32%/12% to34%/14%. Removing the Cessna-only filename list and
executing both aircraft renders raises rendering implementation from20% to22%;
its verified share remains0%. With weights8 and6, these changes add0.28 overall
implementation percentage points and0.16 verified points:41.41/28.17 becomes
41.69/28.33 (displayed41.7/28.3). These coverage judgements are not statistical
measurements or evidence of whole-game readiness. The site/generator now display
one decimal place instead of rounding both totals to integers.

Workspace formatting, Clippy with warnings denied and the full workspace tests
pass, including the synthetic scene integration test and528 native gate cases.

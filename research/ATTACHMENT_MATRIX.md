# Attachment affine matrix and scene integration

Reference: installed X-Plane12.4.3-r2, build124311, PE SHA-256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Native evidence

REA bounded `inspect_with_objdump` inspected the complete function
`0x141775d10..0x141775ece` in the unchanged original PE. Local Evidence ID:
`ev_8b73d61b40082a3a094f5e5ff4a8fe652176f7be8a5ec7a9e68897b05ae5544d`.
The transparent adapter selector is `attachment-matrix`. Listings remain local.

The function writes a row-major 3x4 affine matrix: three translation inputs and
psi/theta/phi angles in degrees. Its radians multiplier is float32 word
`0x3c8efa36`; original `sinf` is 0x14230be80 and `cosf` is 0x14230b380. Arithmetic
order and signed zeros are retained in the port's scalar matrix construction.
The draw caller 0x141a76f2e supplies the attachment runtime coordinates and angles
from the interleaved record words +0x6c..+0x80. The complete runtime producer of
those words, including moving parent parts, is not established by this check.

`tools/generate_attachment_matrix_vectors.py` executes the complete original
builder and original CRT bodies in Unicorn, with no callee stubs and an enforced
return boundary. Its 128 synthetic cases include zero, signed zero, individual
axis turns, mixed angles, negative angles and values up to 720 degrees.

- Replaying the original trig results into the port matches all 12 matrix words
  exactly for every case, including translation and signed-zero words.
- The normal Rust trig path matches rotation coefficients with absolute tolerance
  `8 * f32::EPSILON` (less than1e-6); translation remains bit-exact. This separates
  platform CRT differences from matrix arithmetic rather than claiming identical
  trigonometry on every platform. Executed on this Windows host; CI checks other
  platforms separately.
- Workspace formatting, Clippy with warnings denied and the full workspace test
  suite pass. The scene integration test checks a 90 degree attachment rotation,
  translated triangle vertices and the rotated normal through the actual loader.

Reproduce:

```sh
python tools/generate_attachment_matrix_vectors.py "<X-Plane>/X-Plane.exe" crates/xp-acf/data/attachment_matrix_vectors.tsv
cargo test -p xp-acf --lib attachment::tests::matches_original_affine_builder
cargo test -p xp-app --bin openxplane scene::tests
```

## Scene behavior and visual probe

The scene now uses the recovered affine builder for unattached object poses.
It rotates baked OBJ vertices and normals, then applies the attachment translation
in the ACF frame; the flight model subtracts the real CG when converting to world
coordinates. Nonzero rotations no longer abort a preview. Attachment to a wing,
body or gear still reports an error until that parent transform is integrated.

A local controlled Cirrus ACF fixture sets every object to psi=-15, theta=20,
phi=35degrees and changes object references to absolute paths into the original
installation. It contains only the original object property records under a new
ACF header; it stays in ignored `research/local/cirrus-rotation-probe.acf` with
metadata in `cirrus-rotation-probe.json`. No installed file or asset is modified.
This synthetic pose is deliberately not described as an original gameplay state.
Create the private fixture with the supplied generator, then render it:

```sh
python tools/make_attachment_rotation_probe.py "<X-Plane>/Aircraft/Laminar Research/Cirrus Vision SF50/CirrusSF50.acf" research/local/cirrus-rotation-probe.acf
openxplane render research/local/cirrus-rotation-probe.acf site/assets/cirrus-rotation-probe-20261026.png
```

![Controlled Cirrus attachment rotation, port only](../site/assets/cirrus-rotation-probe-20261026.png)

The unchanged installed Cirrus is also rendered as a zero-angle regression and
compared with the previous port image: all 1280x800 pixels match (zero changed pixels). These are port-side checks, not paired
original/port visual or motion evidence. No GUI control was resumed.

## Remaining fidelity work and progress assessment

Native affine arithmetic is verified on controlled inputs. Original live pose
production, parent motion, later visibility classification, actual camera state,
kill datarefs, materials and cockpit systems remain incomplete. This work does
not prove full renderer or game parity.

The rendering implementation rubric moves from 22% to 23% for integrated attachment
rotation support. ACF loading stays 34%/14%, and verified rendering stays 0% because
matched visual equivalence is still missing. This is a coverage judgement, not a
percentage derived from 128 passing cases. The weighted totals are 41.75%/28.33%,
displayed as 41.8%/28.3%.

# Common aircraft object attachment loader

Reference: installed X-Plane 12.4.3-r2, build 124311, PE SHA-256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

REA bounded `inspect_with_objdump` inspected the complete common function
`0x141aaa610..0x141aaaac1` against the unchanged original PE. Local Evidence ID:
`ev_ef1407282c32746af1bca5868b169308083958d7966c2764ca61910f9a72a48d`.
The transparent range adapter is `tools/rea_bounded_objdump.rs`, selector
`attachment`. Listings and private content fixtures stay under ignored local research.

## Recovered read-mode behavior

The caller passes an existing attachment record and a property context. The
function reads the filename into two strings, steering/internal integer payloads,
wing/body/gear indices, three coordinates and psi/theta/phi angles. Missing
properties retain prior words; constructor defaults are outside this function.
The port exposes explicit prior state through `Attachment::read_into` and only
constructs a fresh record with `from_explicit` when all common fields exist.

When the legacy `_v10_att_part` is present and **none** of the three modern target
properties is present, the target indices are first reset to -1. Legacy indices
8..55 select wing index minus8, 56..84 select body index minus56, and85..94
select gear index minus85. Other values leave all three at -1. A modern property
whose value is zero still suppresses migration: presence matters here.

All coordinates are multiplied by the reference float32 feet-to-metres constant
`0x3e9c0ebf`. When all three target indices equal -1, Y and Z then subtract the
caller-provided global CG coordinates (`0x146125cf4` and `0x146125cf8`). Attached
coordinates are only scaled. Angles are stored unscaled in psi/theta/phi order.
This establishes loader words, not the final rendering axes or rotation order.
The steering/internal integers are kept opaque rather than interpreted as flags.

## Executed verification

`tools/generate_attachment_vectors.py` executes the actual common function and
actual property readers in Unicorn, reaching the exact return boundary. Only
operator new/delete are stubbed. Context entries, initial record words and global
CG values are controlled inputs; constructors and their defaults do not run.

- 48 public synthetic cases cover missing fields with nonzero prior state,
  short/heap/empty filenames, both coordinate branches, nonzero angles and
  target values, and legacy migration boundaries and presence conditions.
- 42 installed Cessna attachment records were compared locally with the final
  48 synthetic cases (90 cases).
- 20 installed Cirrus Vision SF50 records were compared locally with the final
  48 synthetic cases (68 cases). Complete records also exercise `from_explicit`.
- All11 numeric words and filenames match exactly in these cases. Rust's
  rejection of duplicate/malformed/non-finite inputs is a defensive boundary,
  not a recovered claim about original error handling.
- `aircraft-info` was run against the installed Cirrus and now reports loaded
  target indices, metre coordinates and unscaled angles for its attachments.

Reproduce the distributable synthetic fixture:

```sh
python tools/generate_attachment_vectors.py "<X-Plane>/X-Plane.exe" crates/xp-acf/data/attachment_vectors.tsv
cargo test -p xp-acf --lib attachment
```

For private installed-content checks, add the ACF path as the third argument and
write to `research/local/attachment-<aircraft>.tsv`. Set
`OPENXPLANE_ATTACHMENT_VECTORS` to that absolute fixture path for the same test;
remove the variable afterward. These files contain original property values and
must not be committed.

## Remaining integration

`aircraft-info` and the scene renderer use this port. The renderer now reads ACF
object records and translations; [preliminary draw-gate evidence](OBJECT_DRAW_GATE.md)
records the new Cirrus/Cessna renders and remaining visibility limits. The [affine matrix port](ATTACHMENT_MATRIX.md) now handles unattached rotations;
parent-part attachment transforms remain unsupported. The enclosing object-array loader,
object visibility/flags, kill datarefs, attached-part transforms, rotation
composition and final CG integration into the renderer are still required.
Workspace formatting, Clippy with warnings denied, and the complete workspace
test suite pass. Local reference-dependent tests outside the attachment checks
retain their existing opt-in/skip conditions; this is not a whole-game parity test.
No window was controlled or new visual-parity claim made for this loader change.

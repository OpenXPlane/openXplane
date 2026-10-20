# Actual record class-string producer, labels and reached resize

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
The class-string unwind interval141a3df30..141a3e18d is605 bytes. Label
141a3cd30..141a3d05c spans812 bytes including its embedded dispatch table.
Resize's primary interval14064e290..14064e2b8 is40 bytes; its conditional
transfer reaches additional growing code outside that interval. Only the
non-growing branch actually reached here is ported and verified.

## Whole class-string caller

`ui_record_class_string::build` initializes an empty32-byte SSO result and searches
for each signed integer key0..11. At every iteration it reloads the tree head at
record+180, takes the root at head+8, follows left/right pointers0/10 using signed
DWORD+1c comparisons, and uses nonzero byte+19 as the terminal marker. The final
candidate test is retained. Key11 is searched but does not emit a label.

For each emitted key, the actual label constructor fills a physical temporary
at SP+58. The caller appends slash/NUL in place when unsigned capacity-length
wrapping64 has room; otherwise it requests growth1405eca90. The fifth stack argument1 and the raw low byte cached from its saved
output-pointer home slot are preserved. That byte is not inferred to be a
semantic application flag. Controlled providers change the home slot afterward;
subsequent growth must still consume the earlier cached byte.

The returned descriptor is moved into a second physical temporary at SP+38,
then the returned source is reset to an empty SSO string. Cached XMM values
select the append pointer and length after that reset. Append140624040 is
followed by cleanup of SP+38 and then SP+58, with fresh capacity/pointer reads.
An earlier free can change the later physical descriptor. Aligned deallocation
uses the same>=4096, data-8, +39 and unsigned offset<=31 contract as the equality
caller; allocation/free bodies and ownership remain controlled here.

After all12 searches, the caller reloads output length/capacity/storage. It
searches backward for the last slash, including when that slash is not the last
byte. If found it calls resize with that position and fill byte0; otherwise the
string is unchanged. Empty output skips the search. The actual reached resize
branch sets the new length and terminator, returning through14064e2b3. A
shortening-only source default implements those writes; growing resize and its
allocator/memset paths are explicitly unsupported by this binding.

The caller always returns its original output address, ignoring append/free
RAX values. Main-owned frame declarations are SP+20/58 and saved output home
SP+a8/8, with post-prologue SP=entrySP-0x98. The body explicitly preserves the
saved-RDX home write before initializing output. Other saved registers and
Windows exception/unwind ABI behaviour are outside this value/memory proof.

## Label constructor

The complete label caller consumes low32 of the index. Indices0..11 select
Airliner, General Aviation, Glider, Cargo, Experimental, Helicopter, Military,
Seaplane, Science Fiction, VTOL, Ultralight and Favorites. Other low32 indices
select Unclassified. These are reference-bound constants, not a claim about
localization or visible menu layout.

The16-byte General Aviation label requests32 bytes and uses capacity31. All
others fit SSO capacity15. The source retains the original pre-allocation
zeroed data/length/capacity state, then only the post-allocation pointer/metadata
and literal bytes are written. A controlled allocator changes the descriptor's
unused high8 data bytes to verify that the constructor does not erase that
change afterward. Full RAX equals the output address in all normal cases.
Allocation failure, literal modification and other reference builds are unproved.
The mapped slash constant142505110 is preserved as read-only fixture input.

## Overlap exposed an incorrect bulk snapshot

A static review of the move sequence found that the native caller loads first16
bytes, writes them at SP+38, then loads the second16 and writes them at SP+48.
The initial Rust draft loaded all32 before either store. Those are different
when a controlled growth return overlaps the physical destination.

The port now preserves the native sequence and separately caches both XMM
values.146 direct cases return a descriptor at SP+30. The first store changes
the later loaded length, producing a cached zero-length append, while subsequent
source reset/flag writes can change the physical descriptor again. All146
zero-length operands, full provider-boundary bytes and final output match the
original. The overlap is an explicit synthetic growth-provider result, not a
claim that the real growth body returns a stack alias. Ordinary caller cohorts
retain receiver/external return shapes; their non-overlap behaviour is unchanged.

## Direct and composed evidence

960 complete original class-string calls plus60 direct label calls match strict
Rust replay. Trees include balanced and left/right chains, signed MIN/-1 and
12/MAX boundary keys, two independent roots, nil markers1/255, empty/filter-only/
full/mixed key masks, and controlled root replacement between iterations.
Output may alias record+40 or vary its low address byte. Controlled append effects
exercise normal slash termination, no slash, a suffix after the last slash and
empty output. Tree topology is ordered synthetic state; balancing/rotation,
real producer invariants, malformed graphs, native faults and arbitrary lifetime
are not established. Source cycle/search bounds reject unverified profiles.

4073 actual label entries/returns match (4013 from class-string +60 direct).
381 actual short-resize entry/RET pairs match. There are435 controlled allocation,
426 growth,4013 append and2845 free requests. All960 outer original calls reach
normal return;60 label calls cover every enum/default branch, signed/unsigned
boundaries and high32-bit index aliases under three poison patterns.

1500 whole original equalities/wrappers now execute1458 actual class-string
bodies and8286 actual label bodies, with690 short resizes. Their class scope
reaches796 allocations,738 growth,8286 appends and6306 frees. Outer comparison
scope reaches6158 memcmp and988 frees; all declared memory/provider boundaries,
normal returns and frame relationships match source. Controlled derived endpoints
are absent in this new mode. The comparison production default now runs this
recovered class-string, including nested scratch lifetime management.

560 whole finish callers execute284 actual comparisons,568 class-string bodies,
3122 labels and286 short resizes. Class scope reaches280 allocations,278 growth,
3122 appends and2234 frees; comparison scope reaches1565 memcmp and424 frees.
Finish reaches198 assignments,175 shifts,63 growth requests and305 output paths.
712 whole initial callbacks execute1143 comparisons,2286 class-string bodies,
12588 labels and1138 short resizes. Class scope reaches1152 allocations,1140
growth,12588 appends and9176 frees; comparison scope reaches6279 memcmp and1698
frees. The outer callback reaches285 float requests,393 controlled finishes,
314 controlled recursive resets and139 payload copies. Actual tag/payload helper
returns are126/21; controlled-helper mode remains explicit for other profiles.
The separate768 no-stub helper pairs also complete in generation.

New caller fixtures seed their class trees at the outer boundary and preserve
all inherited regions on nested entry. `P` comparison subrows contain explicit
`D` class-string subrows. Replays verify counts (560/284/568 and712/1143/2286),
arguments, inherited bytes and the two caller-frame deltas of-a0; they execute
recovered bodies rather than replacing a whole child with its post-state.
Production defaults introduce only missing scratch and remove that new storage
on return. The two ordinary default tests now use actual empty-tree class
production, keep a comparison-provider change to live memory, and verify nested
scratch expires.

## Limits and next work

Allocation, append, string growth, free and memcmp bodies remain controlled.
Only normal successful calls with declared mapped backing are proved. Real
record initialization/copy/assignment and registry/tree producers, mutation/
ownership/lifetime, secondary14113ff10 and float1411363f0 remain unfinished.
The earlier recursive reset/whole-factory cohort still uses explicitly controlled
comparisons and derived strings: it is not relabelled by these direct caller
proofs. Bind the recovered chain there after establishing its input invariants.

Raw finish arithmetic still carries the explicit Unicorn2.1.4 NaN override;
independent production SSE evidence retains its separate scope. These string
proofs add no new FP/MXCSR evidence. Full dialog/layout/widgets/Main Menu,
Alt/Esc live integration, viewer/GPU and paired screenshots/GIF/video are still
unverified. Compatibility percentages are reviewed and unchanged.

## Reproduction

Numeric corpora, executable and resources stay local:

```sh
set -e
python tools/gen_ui_record_class_string_vectors.py /path/to/X-Plane.exe > /tmp/classes-new.txt
mv /tmp/classes-new.txt /tmp/classes.txt
OPENXPLANE_UI_RECORD_CLASS_STRING_VECTORS=/tmp/classes.txt cargo test --offline --locked -p xp-app --test ui_record_class_string_local -- --ignored
python tools/gen_ui_record_compare_vectors.py /path/to/X-Plane.exe --native-derived > /tmp/compare-class-new.txt
mv /tmp/compare-class-new.txt /tmp/compare-class.txt
OPENXPLANE_UI_RECORD_COMPARE_CLASS_VECTORS=/tmp/compare-class.txt cargo test --offline --locked -p xp-app --test ui_record_compare_local whole_equality_with_actual_class_tree_label_and_resize_callers -- --ignored
python tools/gen_ui_selection_finish_vectors.py /path/to/X-Plane.exe --native-derived > /tmp/finish-class-new.txt
mv /tmp/finish-class-new.txt /tmp/finish-class.txt
OPENXPLANE_UI_SELECTION_FINISH_CLASS_VECTORS=/tmp/finish-class.txt cargo test --offline --locked -p xp-app --test ui_selection_finish_local whole_finish_with_actual_comparison_class_tree_label_and_resize -- --ignored
python tools/gen_ui_selection_initial_vectors.py /path/to/X-Plane.exe --native-derived > /tmp/initial-class-new.txt
mv /tmp/initial-class-new.txt /tmp/initial-class.txt
OPENXPLANE_UI_SELECTION_INITIAL_CLASS_VECTORS=/tmp/initial-class.txt cargo test --offline --locked -p xp-app --test ui_selection_initial_local whole_initial_with_actual_comparison_class_tree_label_and_resize -- --ignored
```

All four explicit local replay layers passed:960 derived/60 label calls,1500
complete comparisons,560 whole finishes and712 whole callbacks.206 ordinary
xp-app tests passed,98 local-proof tests are ignored by default. Current workspace/
all-targets Clippy with warnings denied and formatting passed. The prior1500
controlled-derived comparison and712 controlled-derived callback+predicate
replays also passed. Original-dependent corpora are absent from hosted CI; its
green checks are separate from these local comparisons.

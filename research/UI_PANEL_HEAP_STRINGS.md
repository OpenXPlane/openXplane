# Common-panel heap-string construction and ownership

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Recovered copy and cleanup

String-copy constructor1405dd5c0 zeroes the destination32-byte object. It selects
the source's inline bytes or heap pointer according to source capacity. Lengths
through15 produce capacity15 and copy **all16 source data bytes**, including
residual bytes, even when the source itself is heap-backed. Longer copies use
capacity `max(length | 15, 22)`, allocate capacity+1 bytes and copy length+1 bytes.
Capacity is determined by length, not inherited from the source's spare capacity.

Allocations of at least4096 bytes request an additional39 bytes, align the user
pointer to32 bytes via `(allocation + 39) & ~31`, and store the original allocation
address at user_pointer-8. Cleanup requests capacity+1 bytes for small storage;
large cleanup checks the preceding allocation header and requests capacity+40.
After cleanup, length becomes0, capacity15 and first byte0; other raw object
bytes retain their previous values. Actual allocator/free/CRT-copy bodies are
controlled in this replay. Failure/overflow abort paths are not executed.

Common panel14129c7a0 makes identifier temporary1, temporary2, then wrapper14129fc10
makes temporary3. Input base1412a70a0 allocates its callback sentinel and copies
temporary3 into object+60, then initializes its other containers and consumes
temporary3. Wrapper tables are installed before temporary2 is consumed; the
intermediate common tables are installed before temporary1 is consumed. Final
common tables precede label/auxiliary copies into+128/+148. Flags and key mask
are stored before original label/identifier/auxiliary arguments are consumed.

`ui_native_string` now represents raw string objects and owned backing blocks.
`construct_owned` in input/common panel constructors shares this implementation
with legacy inline APIs. Successful constructors return live sentinel/string
blocks; temporary/source blocks are released through explicit services. Raw
reference pointers/tables are metadata identities, not host pointers or vtables.

## Whole original constructor comparison

`tools/gen_ui_panel_heap_vectors.py` executes complete common/wrapper/input/
callback/string constructor bodies unchanged. Only allocator, sized free and
CRT byte-copy endpoints are controlled. 324 cases combine18 length profiles,
three poison patterns, ordinary/forced-heap source capacities and three flag
triples including the dialog's1/0/1. Individual lengths include0/1/15/16,
20/22/23/31/32/63/255, and4079/4080/4095/4096 around aligned-allocation boundaries.
Profiles include a20-byte label,10-byte identifier and0/15/16-byte auxiliary.
Forced-heap sources test short strings and spare capacity. Payloads include
embedded zero bytes; copy is governed by stored length rather than C strlen.

For every case, Rust matches the entire0x170-byte panel, all three consumed
source objects, every allocated block (including temporary blocks and alignment
headers/padding), and the exact ordered allocation/free requests with full panel
snapshots at each request. Source backing bytes remain unchanged. Free endpoints
record requests without unmapping memory, so post-free buffer comparisons are
audit evidence, not a promise that freed memory remains valid in a real runtime.
All324 cases pass. Earlier64 input,256 inline common-panel and480 concrete-menu
prefix cases pass again after the shared implementation refactoring.

The ordinary regression exercises the dialog string profile: the panel keeps
its own20-byte heap label after the source argument is released, retains the
inline identifier, consumes all inputs and exposes initial key flags1/0.

Corpus remains local at `/private/tmp/openxplane-ui-panel-heap.txt`, replayed via
`OPENXPLANE_UI_PANEL_HEAP_VECTORS`. Executable/assets and corpus are not published;
source/generator/test and this report are published.

## Remaining boundaries

Successful, separate, stable string objects/backings only. Reentrant allocation,
concurrent mutation, exceptions, aliasing, allocation failure/overflow aborts and
arbitrary object destruction are unverified. The source bounds supported input
length/capacity and reports unsupported profiles rather than reproducing native
fatal failure behavior. It does not implement arbitrary MSVC string operations.

Heap strings remove a prerequisite for dialog140ec9c10, but that constructor's
translation/formatting, embedded callback copies,1410d6ca0 children, layout and
widget bindings still need composition and verification. Main-menu singleton
publication and the remaining menu constructor are incomplete. Opening ID3/Alt,
original GPU output and paired screenshots/GIF/video remain missing; Esc still
exits the viewer. Overall rubric stays41.4% implemented/28.2% verified and
Rendering/UI verified remains0. These constructor comparisons are not whole-game
or visual parity evidence.

# Callback-wrapper links needed by menu dialogs

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Recovered behavior

Base container constructor1410c7b50 creates an empty connection set.
Wrapper constructor1410c7d20 allocates its own sentinel, stores full64-bit R8/R9
parameters at+18/+20, sets custom callable+60=null, and optionally registers
against its RDX source. Registration1410c8de0 inserts each endpoint into the
other endpoint's ordered set. The two lookups are independent and repeated
registration does not allocate duplicate entries.

Copy constructor1410c7bf0 creates a fresh empty set, copies both parameters and
visits the source's connections in ascending address order, registering the new
wrapper with each peer. It does not implicitly connect the copy to its source.
A nonnull custom callable would additionally use a virtual clone; that profile
is not yet ported or verified. All compared custom callable pointers are null.

Base teardown1410c8430 visits this object's connections, removes this identity
from every peer, then destroys its own set and sentinel. Copies remain linked
when the temporary source wrapper is removed. Destroying the shared source
removes it from all surviving wrappers; it does not destroy those wrappers.

`ui_callback_links` represents these connection semantics as ordered sets with
stable identities. Allocation/free sizes and counts are audit events, not an
actual allocator. Native tree-node layout, balancing, retained padding and
free-pointer order are deliberately not represented or claimed identical.
The source is a dependency for real dialog construction, not a complete dialog.

## Original-code replay

`tools/gen_ui_callback_links_vectors.py` executes original base/wrapper/copy,
registration, red-black insertion/erase and base teardown bodies unchanged.
Only allocation and sized free endpoints are controlled; free does not unmap
memory. It inspects live sets by traversing the native trees, asserting sorted
unique keys and agreement with native stored sizes. It also reads wrapper
parameters and asserts null custom callables after every action.

324 sequences combine three poison values, three parameter pairs (including
high64-bit values), six destruction orders, forward/reversed object addresses
and initially null/first/second sources. Each sequence includes two containers,
four wrapper constructions, two wrapper copies, two explicit bindings including
a duplicate, and destruction of all eight objects:5832 original calls total.

After each action, the Rust comparison matches all live identities, wrapper
parameters, ordered peer sets and allocation/free size/count events. All324
sequences and5832 intermediate states pass. This is **logical state equality**,
not whole-object/heap byte equality. The ordinary regression checks the concrete
required lifetime: copy a bound temporary, destroy the temporary, retain the
copy's registration, remove the source and finally destroy the detached copy.

Numeric corpus stays local at `/private/tmp/openxplane-ui-callback-links.txt`;
opt-in replay uses `OPENXPLANE_UI_CALLBACK_LINKS_VECTORS`. Original executable,
assets and corpus are not published. Generator, source and tests are published.

## Dialog constructor evidence and remaining work

Concrete Main Menu ctor140ecb340, after the verified prefix stop140ecb592,
checks global singleton145783130. If absent it allocates0x320 bytes, constructs
two bound temporary wrappers against menu+48 with pairs(0,2) and(3,0), and passes
them reversed in R8/R9 to dialog ctor140ec9c10. The dialog copies these wrappers
at+1f8 and+260 through1410c7bf0 before the outer ctor destroys the temporaries.
This gives the connection-lifetime port a concrete constructor dependency.

Dialog140ec9c10 also requires a translated auxiliary string, a heap-backed
label, common-panel flags1/0/1, callback/interface/member initialization,
1410d6ca0 child construction and layout/widget binding. The existing common-panel
port accepts inline strings only, so heap-string handling is a next prerequisite.
The dialog ctor, singleton publication path, resource/UI construction and their
composition with this link model still need original-code comparisons. No claim
of complete dialog construction, callback invocation, custom callable lifetime,
self-bound teardown, invalid/dangling graphs, allocator failure or concurrency.

Opening ID3/Alt, original viewer/GPU output and matched screenshots/GIF/video
remain missing. Esc still exits the viewer. Overall estimates stay41.4%
implemented/28.2% verified; Rendering/UI verified stays0.

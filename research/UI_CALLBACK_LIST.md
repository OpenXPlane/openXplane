# Callback-list receiver141149460

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Concrete construction dependency

Child constructor1410d7490 initializes its own callback container at+20, creates
a bound temporary wrapper with parameters0/0, reloads its parent from+58, and
passes the wrapper to receiver141149460. That receiver appends a **copy** to the
parent's list before the temporary is destroyed. Dialog140ec9c10 also invokes
this receiver repeatedly when binding its children. It is not a borrowed pointer
to the stack temporary.

Receiver141149460 reads the parent's list sentinel at+8 and count at+10,
requests120 bytes, and calls wrapper copy constructor1410c7bf0 on entry+16.
Only after copy/peer registration returns does it increment count and publish
the new entry at the tail: entry.next=sentinel, entry.previous=old_tail,
sentinel.previous=entry, old_tail.next=entry. Empty-list insertion updates both
sentinel links. A native maximum-count branch aborts; that fatal path is not
reproduced or covered by these comparisons.

`ui_callback_list::List` models count/order and (identity,next,previous) link
fields using stable identities. `append` composes the recovered
`ui_callback_links::State::copy`, preserving full-width parameters and registering
the retained copy against all source peers. The source wrapper is not implicitly
a peer of its copy. Source custom callable must be null. The function returns
the new entry as a host convenience; native receiver return-register behavior
is not claimed or used.

## Combined original-code comparison

`tools/gen_ui_callback_list_vectors.py` executes the whole receiver, real wrapper
copy/registration/red-black insertion and base teardown bodies unchanged.
Only allocation/sized-free endpoints are controlled; free records a request
without unmapping memory. Native list traversal checks stored count and forward
order; callback-tree traversal checks sorted unique peer keys and stored counts.

72 sequences combine three poison patterns, unbound/single-source/multiple-source
wrappers, four eight-append source orders and forward/reversed object identities.
Each sequence performs eight appends, then destroys the three original wrappers
and two original containers while retaining all eight list copies. There are576
whole receiver calls and360 whole base teardown calls:936 compared states.

Rust matches sentinel/entry next/previous fields, list count/order, all live
callback identities, full64-bit wrapper parameters, ordered peer sets, and
allocation/free sizes/counts after every action. Copies survive source-wrapper
removal; source-container removal detaches every remaining copy. All72 sequences
and936 states pass. An ordinary regression checks a bound3/0 temporary copied
into a parent list and then destroyed, retaining the copy's registration.

This is exact comparison of list link/count fields plus **logical callback
state**, not whole120-byte node/heap equality. Native callback-tree node bytes,
balancing, padding and free-pointer order remain outside the source model.
Non-null custom callable virtual clone/destruction, list removal/destruction,
invalid/dangling/self-bound endpoints, allocator failure, fatal-length behavior,
reentrancy and concurrency are unverified. Endpoint identities and live sets are
stable throughout the declared profile.

Corpus remains local at `/private/tmp/openxplane-ui-callback-list.txt`, replayed
via `OPENXPLANE_UI_CALLBACK_LIST_VECTORS`. Executable/assets/corpus are not
published. Source, generator, opt-in test and report are published.

## Remaining work

The receiver supplies a prerequisite of actual child1410d7490 and the dialog's
later bindings. Compose that constructor's base/callback fields, refresh1411396d0,
receiver copy and temporary teardown, then connect it to outer factory1410d6ca0.
The other nested child/resource/refresh bodies and complete dialog/layout/
singleton/Main Menu construction remain unfinished. Opening ID3/Alt, original
viewer/GPU output and paired screenshots/GIF/video remain missing; Esc still
exits the viewer. Compatibility estimates stay41.4% implemented/28.2% verified
and Rendering/UI verified0. Whole-game and visual parity remain unproven.

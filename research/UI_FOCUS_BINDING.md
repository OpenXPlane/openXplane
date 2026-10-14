# Owner binding and selected-chain clearing

Reference: X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Original behavior recovered

`0x1412f6250..0x1412f6320` handles node+0x08 owner links and node+0x10 selected
links, independently of widget parent+0x60. Equal old/requested owners return
without changes. A null old owner simply accepts the requested owner.

For a nonnull old owner it walks upstream while each owner selects the current
child. If the resulting root equals global focus `0x145a0dfc8`, it walks that
root's selected links down to the last leaf and calls virtual +0x40 (blur).
It then **reloads this node's owner after blur**, clears each ancestor's selected
link before virtual +0x10 and stops on a nonzero return byte. After a declined
callback it reloads that ancestor's owner, retaining callback mutations.

Otherwise it clears only the captured old owner's selected link when that link
points to this node. Finally it stores the captured requested owner after all
callbacks. The helper does not itself store the global focus pointer; changes
to that pointer in these tests are controlled callback effects.

## Native comparison

`tools/gen_ui_focus_binding_vectors.py` executes the whole original helper
unchanged in 1045 synthetic finite graph scenarios. Only virtual blur/+0x10
callback endpoints are controlled; no allocator is needed. Cases include null/
equal/distinct ownership, complete/mismatched selected chains, absent/other/
matching focus roots, stop bytes0/1/255 and callbacks changing this/ancestor
ownership, reintroducing selected links or changing global focus.

Rust compares every owner and selected link, final global focus and complete
callback order. All 1045 cases pass. The ordinary regression confirms post-blur
owner reload and survival of callback-reintroduced selection. Corpus stays local
at `/private/tmp/openxplane-ui-focus-binding.txt`; opt-in replay requires
`OPENXPLANE_UI_FOCUS_BINDING_VECTORS`. Original files and corpus are not published.

## Limits and integration gate

Actual virtual blur/+0x10 bodies and real construction/lifetime remain unknown.
Nodes are stable and chains finite; cycles/invalid pointers/deletion during
callbacks are unsupported rather than emulated. This helper is not yet joined
to the shared window/key graph, whose preceding focus was pinned and binding
endpoint controlled. Next replace that endpoint with explicit owner/selected
state and compare changes to subsequent key routing, then recover construction,
opening ID3/Alt and renderer effects. Original layout/fonts/GPU output and paired
Alt/Esc screenshots/GIF/video remain missing; UI verified coverage stays zero.

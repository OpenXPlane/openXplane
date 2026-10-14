# Windows XPLM key-sniffer registration lifecycle

Date: 2026-10-14. Windows DLL SHA256
`706c6f1fd856c1a8c121edac2c8df12e65603e821e3f786ee8bde72543c1401f`.

## Actual exports and list behavior

`XPLMRegisterKeySniffer` is the complete function at `0x180011230..0x180011483`.
It requests thread validation (`0x1800156a0`) and lazy loading for signed plugin
-1 (`0x1800213f0`), then a 0x40-byte allocation (`0x18002f904`). It initializes
links/deleted byte and owner -1, stores callback +0x28, callback-plugin ID +0x30,
full signed before +0x34 and refcon +0x38. Callback-plugin lookup
`0x1800206e0 -> 0x18001d4f0` walks ready TLS like GetMyID, with an additional
optional assertion if a nonzero inherited ID is found below a zero top frame.
Assertions are disabled in these comparisons; initialization/diagnostics are
not claimed reproduced by that lookup.

After allocation it increments signed depth and scans the live list. The first
same callback/before/refcon tuple rejects registration with result 0. No
plugin-ID or deleted-byte filter is applied. The allocated record remains
unlinked with owner -1, and this branch requests **no free**; the port preserves
that observed request behavior, without claiming a permanent process leak.

For a new tuple, `GetMyID` supplies owner +0x20; clock/tracking lookup are
requested and meaningful tracking fields store plugin, flag 1 and elapsed float.
The record is inserted at the live head, updating old-head previous. Depth then
decrements with signed wrapping and negative-to-zero clamping. When depth is
zero and pending is nonzero, actual deferred drain `0x1800094f0` runs and pending
clears afterwards. Disabled consistency validation surrounds mutations/drain.

`XPLMUnregisterKeySniffer` is `0x1800121d0..0x180012358`. It validates the thread,
increments depth and finds the first matching tuple without deleted/plugin
filters. Finding a tuple sets result 1 **before** ownership removal succeeds.
The actual remove helper `0x18000da90..0x18000dd9a` checks membership, rejects an
already-deleted node when defer is nonzero, then checks owner (current plugin 0
bypasses owner). Rejections request diagnostics/notification/bad-free tracking;
they do not change the exported matched result.

Valid deferred removal records plugin/flag 0/elapsed, sets deleted byte 1 and
pushes the node onto deferred head through +0x18. It leaves live links intact.
The wrapper sets pending when current depth is nonzero, **even if removal was
rejected**, then closes the iteration and drains at depth zero. An original
signed initial depth -1 becomes 0 after increment and takes immediate removal;
the shared checked deletion performs tracking, unlinking and free. Repeated
unregistration may find a still-live deleted record and return 1 while logging
its rejection. Pending's noncanonical byte is normalized only where native
stores set 1 or clear 0.

`xp-app::xplm_sniffer` now connects these APIs and EndCall (`0x180008010`) to the
recovered `xplm_deferred` list, preserving callback records and explicit freed/
unlinked audit entries. It retains external service boundaries for validation,
lazy loading, allocator, clock, tracking and diagnostics. Hotkey registration
and key-hook/viewer integration remain unfinished.

## Direct original-code evidence

`tools/gen_xplm_sniffer_vectors.py` executes complete Register/Unregister,
original callback-plugin lookup/GetMyID, remove helper, EndCall, consistency
validator with its diagnostic flag disabled, and deferred drain unchanged.
No routing, list stores, duplicate matching, exported returns, depth/pending
stores or ready-TLS ID function is replaced. External services are controlled;
original DLL bytes and the numeric corpus stay local.

1126 scenarios compare return sequences, final live/deferred heads, full signed
depth/raw pending, every node's links/owner/deleted/callback/plugin/before/refcon,
free requests, meaningful tracking fields/float bits, preserved TLS stack and
ordered service traces (queue head is visible at each request). Inputs include
empty/nested/zero/signed-extreme contexts, all signed-depth edge cases including
wrapping, null/max callback words, full before values, duplicate and predeleted
records, existing pending queues, owner rejection, and register/duplicate/
unregister/repeat/EndCall sequences. Observed calls include 1218 allocations,
286 node frees, 328 missing-or-deleted rejections and 623 owner rejections.

The earlier 1704 sniffer/hotkey deletion comparisons still pass after extracting
the shared checked-deletion helper and adding the deleted byte. A normal CI
regression sequence verifies that a deleted live sniffer blocks re-registration
until EndCall, and that duplicate registration still allocates an unlinked
record. Ordinary xp-app library/sequence tests and all xp-app test clippy pass.

```sh
python tools/gen_xplm_sniffer_vectors.py /path/to/XPLM_64.dll > /private/tmp/openxplane-xplm-sniffer.txt
OPENXPLANE_XPLM_SNIFFER_VECTORS=/private/tmp/openxplane-xplm-sniffer.txt cargo test --offline -p xp-app --test xplm_sniffer_local -- --include-ignored
```

## Limits and next work

The native replay comparison is opt-in/ignored in ordinary CI. Supplied TLS is
already initialized/reserved; live links and queue are readable/acyclic, and
allocation succeeds. Allocator failure, validation/loading side effects,
tracking-map implementation/padding, enabled assertions, second-stage rejection,
recursive/concurrent mutations and real plugin lifetime are unverified. Rust
rejects invalid token/cyclic state rather than claiming unsafe native memory
behavior. This is a local CPU API/list replay, not Windows DLL loading or a
running simulator. Actual callback effects are not exercised by registration.

Next connect token-based registered callbacks and list mutation to the actual
key-hook routing, recover hotkey registration/window creation and actual
Alt/Esc menu actions, then original layout/rendering and paired screenshot/
GIF/video acceptance. Whole-game X-Plane 12 1:1 remains unproven.

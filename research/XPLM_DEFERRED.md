# Windows XPLM deferred callback deletion

Date: 2026-10-14. Windows DLL SHA256
`706c6f1fd856c1a8c121edac2c8df12e65603e821e3f786ee8bde72543c1401f`.

## Original helper flow and port

`0x1800094f0` drains sniffer deletions, `0x1800091a0` drains hotkey deletions.
RCX points to live-list head; RDX points to deferred head. Node fields are live
next +0, previous +0x10, deferred next +0x18. The helper pops deferred head
**before** asking `XPLMGetMyID` and checking whether the token belongs to the
live list. Sniffer ownership is +0x20 (distinct from its callback plugin +0x30);
hotkey ownership is +0x28. Effective plugin 0 may delete any owner; otherwise
owner must match the signed effective ID. Zero context frames inherit ownership
as recovered in `XPLM_CONTEXT.md`.

First-stage missing-token or wrong-owner rejection requests formatting
`0x1800124c0`, notification `0x1800210c0` and bad-free tracking (`0x180008610`
sniffer / `0x180008470` hotkey). It then continues from the already-popped queue,
leaving the rejected token in the live list if present. The original also
rescans membership for optional diagnostic assertions, disabled in this replay.

For an accepted entry the helper obtains `XPLMGetMyID` again and repeats
membership/ownership validation. Valid removal requests the float clock
`0x1800294f0` and tracking-entry helper `0x180007400` (map globals
`0x1800680a0` / `0x180068130`), then stores the signed plugin word and zero flag
at tracking +0x18/+0x1c and raw elapsed float at +0x20. Padding bytes are not
compared. It updates live head if needed, previous.next and next.previous,
then requests a 0x40-byte sniffer or 0x68-byte hotkey free at `0x18002f948`.
Second-stage validation rejection logs/tracks the failure and proceeds to node
free without unlinking; those branches are retained in the port but **not
reached** by the current stable-context corpus.

Hotkeys additionally destroy the string at +0x38, length +0x48, capacity +0x50.
Capacity <=15 needs no separate free. Larger capacity uses unsigned capacity+1;
if that size is >=0x1000, it reads the allocation base at data-8, adds 0x27 to
size and checks unsigned data-base-8 <=0x1f before freeing the base. Otherwise
it frees data directly. Length is reset to 0, capacity to 15 and first byte to
0 **before** node free. Unsigned overflow is preserved. The invalid-header
branch calls `0x180037bd4`; the Rust port returns an error instead of claiming
native termination behavior.

`xp-app::xplm_deferred` uses explicit linked tokens and recovered
`xplm_context::State::current_id`. Freed records are retained for audit; actual
allocation and tracking-map storage are services. It rejects unmapped/cyclic
state and service-expanded queues instead of inventing unsafe memory behavior.
It now backs the recovered key-sniffer registration APIs (see `XPLM_SNIFFER.md`); hotkey registration and viewer/hook lifetime integration remain unfinished.

## Original-code comparison

`tools/gen_xplm_deferred_vectors.py` executes both complete helper bodies,
original `XPLMGetMyID` and disabled assertion predicate unchanged. Diagnostics,
tracking-map lookup, clock and allocator endpoints are controlled. The helpers'
branches and all list/string/tracking-field stores execute in the DLL. The
allocator logger also checks that the hotkey string has already been reset
when node free is requested. The supplied DLL bytes stay local.

1704 cases (852 per kind) match exact head/queue, every node's next/previous/
deferred link, string length/capacity/first byte, node-free requests, meaningful
tracking fields and float bits, retained context frames, and ordered service
traces with queue head visible at each call. Original ownership fields are
also asserted unchanged. The cases include empty/subset/reordered lists and
queues, absent tokens, wrong/matching owners, system-owner bypass, negative IDs,
nested zero context frames, inline/heap strings, threshold capacities 4094/4095,
valid large-allocation header offsets 8..39, maximum uint64 capacity overflow,
and arbitrary float bits including nonfinite values (only copied, not computed).

Observed branch coverage: 590 node frees, 84 direct string frees, 57 base/header
string frees, 1521 first-stage absent-token rejections, 1207 first-stage owner
rejections. No claim of second-stage rejection, enabled assertions, invalid
allocation headers, queue production, recursive/concurrent mutations, OS
thread/TLS initialization or real module/plugin lifetime. Diagnostic strings,
tracking-map internals and real allocation bodies remain unported. Consistent
live links and an acyclic unique-token queue are explicit input assumptions.
This standalone CPU replay is not a live simulator or paired UI visual proof.

```sh
python tools/gen_xplm_deferred_vectors.py /path/to/XPLM_64.dll > /private/tmp/openxplane-xplm-deferred.txt
OPENXPLANE_XPLM_DEFERRED_VECTORS=/private/tmp/openxplane-xplm-deferred.txt cargo test --offline -p xp-app --test xplm_deferred_local -- --include-ignored
```

The corpus is local and its native comparison is ignored in ordinary CI. A
normal regression test covers rejected-owner queue consumption followed by an
allowed deletion and relinking. Library tests and xp-app clippy pass locally.
Sniffer registration/unregistration now has 1126 additional native scenarios;
next connect registered list lifetime to the real hook routing and recover
hotkey registration, then actual Alt/Esc menu callbacks/layout/rendering and paired
screenshots/GIF/video. Whole-game 1:1 remains unproven.

# Actual Windows XPLM keyboard hook

Date: 2026-10-14. Executable SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
DLL SHA256 `706c6f1fd856c1a8c121edac2c8df12e65603e821e3f786ee8bde72543c1401f`.

## Supplied module and identity limits

The requested `Resources/plugins/XPLM_64.dll` is now present, together with XPWidgets Windows/Linux modules and the Linux XPLM library. The DLL is AMD64 PE32+, preferred image base 0x180000000, image size 479232, PE timestamp 2026-06-24 15:46:15. It exports XPLMKeyPressHook at RVA 0x1a670 and XPLMCommandHook at 0x19b80. No valid fixed-file-version record was found in the checked DLL bytes; the EXE still has numeric version 12.4.3.11 and its original digest. The DLL's exact build pairing with that EXE is therefore not confirmed by file version.

The supplied Linux XPLM ELF has debug information and symbols such as XPLMDisplayKeyPressHook and gKeyCallbacks. Those names guide investigation only; no cross-platform behavioral equivalence is assumed or claimed. All comparisons below execute the Windows DLL identified above. Original modules remain local and are not distributed by the repository.

## Complete DLL dispatch

Export `0x18001a670` normalizes character/key bytes, invokes validation helper `0x180021e10`, then tail-calls `0x18000b6f0`. The body invokes thread-safety validation `0x1800156a0`, returns 1 for an entirely zero event, and branches on phase:

- Phase 0: increment sniffer-list iteration depth (`0x180069670`), scan list head `0x180069660`, skip deleted nodes (+8) and zero before fields (+0x34). Enter the node's plugin context (+0x30) and test activation; an active sniffer (+0x28) receives character/flags/key/refcon (+0x38). Raw return 0 consumes the event. Finish sniffer iteration before leaving context on this consumed path. Otherwise leave each context and continue. After sniffers, scan the hotkey list (`0x180069690`, depth `0x1800696a0`) for nondeleted nodes whose key byte (+0x2c) and full flags (+0x30) exactly match. Enter plugin context (+0x28); an active match calls callback (+0x58) with refcon (+0x60), leaves context and returns 0. This observed matched-hotkey path does **not** decrement hotkey depth before returning. No independent runtime bug attribution is made: concrete callback effects and registration lifecycle remain outside this comparison.
- Phase 1: require a focused plugin window (`0x1800696d8`) and either null target or matching window pointer. Enter its plugin context (+0x78). When active, reread focused-window fields, call key handler (+0x50) with window/character/flags/key/refcon (+0x80)/losing-focus 0, leave context and return 0. Inactive context returns 1 after release.
- Phase 2: scan zero-before sniffers. This branch does **not** use phase 0's deleted-node test. Raw callback return 0 consumes; other results permit continuation. Finish iteration and return 1 if unconsumed.
- Other phases return 1.

A completed list iteration decrements depth and clamps negative results to zero. When depth becomes zero and pending byte (+0x14) is nonzero, it requests list consistency validation, drains deferred deletions, clears pending, then requests list consistency validation again. In phase-0 sniffer consumption, original helper `0x180008010` supplies the same sequence. List helper bodies, thread checks and user callback effects remain service boundaries; initialized-TLS context/activation are covered by the later replay described below.

`xp-app::xplm_key_hook` preserves these paths and request order. It supports fixed ordered list topology and explicit callback services. Reentrant dispatch, actual plugin registration, node link/lifetime changes, invalid focused-window replacement and assertion handling remain unported.

## Original-code comparison

`tools/gen_xplm_key_hook_vectors.py` pins the DLL digest and executes the whole export/body and consumed-sniffer iteration helper. Context acquisition/activation/release, validation, user callbacks and deferred-list service bodies are logging endpoints. Neither DLL routing branches nor iteration counter stores are replaced.

1090 cases compare exact return, both depth/pending pairs and ordered endpoint traces. They cover phases 0/1/2 and invalid phase words, empty events/lists, deleted/before flags, activation combinations, raw negative/zero/positive sniffer returns, hotkey byte/full-flag matching, focused-window target filtering and deferred-flush request order at different initial depths. Context/lists/callback results are synthetic; the process is not a running original simulator. Ordinary tests retain the distinct after-sniffer deleted policy and matched-hotkey counter behavior.

```sh
python tools/gen_xplm_key_hook_vectors.py /path/to/XPLM_64.dll > /private/tmp/openxplane-xplm-key-hook.txt
OPENXPLANE_XPLM_KEY_HOOK_VECTORS=/private/tmp/openxplane-xplm-key-hook.txt cargo test --offline -p xp-app --test xplm_key_hook_local -- --ignored
```

The corpus stays local; this comparison is ignored in ordinary CI. The subsequent paired replay now executes this DLL export/body from the original EXE adapter (`KEYBOARD_PAIRED_REPLAY.md`). It has explicit synthetic registrations and controlled services; this separate 1090-case comparison still does not establish a running game/module loader.

## Next and visual limits

The missing-module pause is resolved. Next: recover context/activation/list lifecycle services, and use available module symbols to trace real UI registrations/actions. Actual show_menu/Alt actions, live viewer input, menu layout/rendering and paired screenshots/GIF/video remain unverified. This port is not a live plugin loader or proof of whole-game/UI parity.

## Subsequent context recovery (2026-10-14)

The current hook corpus now executes original initialized-TLS context ctor/activation/dtor and enabled query, replacing its former activation-result loggers. The mask initializes real record bytes; plugin 0 is always active. See `XPLM_CONTEXT.md` for 1108 direct context cases, stronger 1090/2152 hook/paired comparisons and the remaining validation/module/TLS-initialization limits.

## Deferred-list helper correction and ownership evidence

Inspection of the helper bodies corrects the earlier cleanup/merge labels:
`0x180008950` validates linked-list consistency when diagnostic global
`0x1800696e8` points to an enabled record; it does not merge or delete entries.
`0x1800094f0` and `0x1800091a0` drain **deleted** sniffer/hotkey queues. They pop
queue head through node+0x18 before membership/ownership checks. The first
missing-node or wrong-owner rejection logs and tracks the attempted deletion,
leaving the live list unchanged but the rejected queue entry already removed.
For a valid sniffer, the helper obtains the owner again, records deletion,
unlinks live next/previous pointers and requests a 0x40-byte free. Hotkeys have
additional string cleanup and a 0x68-byte node free. These observations of the pinned DLL now have a standalone drain port and
1704 native comparisons; see `XPLM_DEFERRED.md` for exact scope.

Deletion ownership is sniffer node+0x20 or hotkey node+0x28. Do not substitute
the sniffer callback-context ID at node+0x30. `XPLMGetMyID` now has 1657 direct
native comparisons on initialized TLS and a port in `xplm_context` (see
`XPLM_CONTEXT.md`); it inherits nonzero IDs through zero frames. String free sizing/reset and live/deferred links now have native comparisons.
Diagnostic, tracking-map, real allocator and registration/lifetime bodies remain
separate work. Current hook replay still logs all three list-service requests and does
not claim its synthetic list is physically mutated by the original helpers.

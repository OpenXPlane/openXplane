# Registered Windows XPLM sniffer dispatch and callback list mutation

Date: 2026-10-14. Pinned Windows DLL SHA256
`706c6f1fd856c1a8c121edac2c8df12e65603e821e3f786ee8bde72543c1401f`.

## Native traversal and context order

The actual `XPLMKeyPressHook` export `0x18001a670` delegates to
`0x18000b6f0`. Byte character/key and full flags are captured before external
validation. The sniffer list is walked by live **node token**, not a snapshot of
callback values or an index into a vector. Phase 0 requires deleted byte 0 and
before !=0; phase 2 requires before ==0 and does not test deleted. All-zero
character/key/flags exits without entering an iteration.

Eligible callbacks enter the node's callback-plugin context and run the actual
activation query. Callback/refcon are loaded after activation. After a callback
allows the event, context is released before live `node.next` is read. Register
inserts at head; new callbacks therefore wait until another event. Deferred
unregister leaves links intact, so phase 0 skips a newly deleted next node while
phase 2 may still call it. This is the recovered phase policy, not an assumed
uniform deletion rule.

If callback returns raw int32 zero, the hook closes the sniffer iteration and
possibly drains deletion **before** releasing its context. Nonzero (including
negative) permits traversal. The exhausted-list path releases every callback
context before closing/draining. Consequently, consumed-path deletion ownership
may be evaluated against the callback plugin rather than the outer frame. A
queued record of another owner can be popped/rejected and remain live/deleted
while pending clears. A dedicated native scenario and ordinary regression keep
that behavior explicit.

`xplm_sniffer::dispatch` now traverses the same registered token state used by
Register/Unregister/EndCall, invokes callbacks that can call those recovered APIs,
and uses `xplm_context` acquisition/activation/release and `xplm_deferred` drains.
It supports **sniffer phases 0 and 2 only**. The existing `xplm_key_hook` handles
other modeled SDK paths; this new API does not invoke hotkeys or focused windows,
and no live viewer/real plugin loader has been connected. Cyclic/unmapped state
returns an error rather than claiming unsafe original behavior.

## Actual original-code execution with callback-side native API calls

`tools/gen_xplm_sniffer_dispatch_vectors.py` first registers every initial record
through the real SDK API on original context constructor/destructor calls. It
then runs the complete DLL export/key-hook body with original context/enabled,
registration/unregistration, remove, EndCall, consistency validation (disabled
assertion flag), and deferred drain bodies unchanged.

Synthetic AMD64 callback thunks execute on the same emulated stack and call
actual `XPLMRegisterKeySniffer` / `XPLMUnregisterKeySniffer`. Python never starts
nested `Emulator.call` from a callback. Thunks either allow/consume, unregister
self, unregister a selected neighboring binding, register a new head callback,
or unregister self and attempt re-registration. Their logging endpoint checks
character/flags/key arguments. These are controlled synthetic callback effects;
no original menu/plugin callback body is claimed recovered.

The callback region is rewritten between scenarios, so the generator explicitly
invalidates Unicorn's translated-code cache for that **synthetic** region. An
independent nonmutating hook also verifies each root thunk's actual int32 return
at RET. An initial local comparison exposed stale thunk execution before this
fix; that discarded corpus was regenerated, not published or used as evidence.
Original DLL code bytes are not rewritten. External validation/lazy loading,
clock, tracking-map lookup and allocation/free are controlled services.

524 scenarios, each with two events, compare exact hook returns, live/deferred
heads, depth/pending, all token links, ownership/deleted/callback/plugin/before/
refcon, explicit free requests, tracking fields and raw float bits, restored TLS
stack [0], and ordered service traces. Queue head and current top context are
visible at allocation/clock/tracking/diagnostic/free endpoints. Thus the tests
observe ownership-sensitive drain timing, not only the final node count. The
native hotkey list and focused window are empty; hotkey depth/pending are checked
restored to zero rather than claiming callback integration for those paths.

Observed calls: 1062 callbacks, 1948 allocations, 111 node frees, 51 owner
rejections, 24 missing/already-deleted rejections. Cases cover both sniffer
phases, repeated/alternating events, empty input, system-ID activation bypass,
disabled/nonzero/negative plugin IDs, raw callback results, nested iteration
depths, head insertion, deferred deletion and blocked re-registration. Ordinary
CI tests retain phase-2 visitation of a deleted next callback, head insertion
waiting for the next event, and consumed drain before context release. Native
comparison is opt-in/ignored in ordinary CI and the numeric corpus remains local.

```sh
python tools/gen_xplm_sniffer_dispatch_vectors.py /path/to/XPLM_64.dll > /private/tmp/openxplane-xplm-sniffer-dispatch.txt
OPENXPLANE_XPLM_SNIFFER_DISPATCH_VECTORS=/private/tmp/openxplane-xplm-sniffer-dispatch.txt cargo test --offline -p xp-app --test xplm_sniffer_dispatch_local -- --include-ignored
```

## Remaining bounds

TLS is ready/reserved, allocation succeeds, initial live links/queues are
readable/acyclic, and plugin unload is absent. Recursive event dispatch,
concurrent mutations, loader/validation/unload effects, real tracking/allocator
bodies, enabled assertions and unsafe lifetime/error paths remain unverified.
This is an SDK CPU replay with synthetic callbacks, not Windows loading or the
running simulator. EXE/DLL release-build pairing remains unproven. Existing
paired EXE/DLL replay still uses its earlier fixed sniffer list; this mutable
registration test has not yet been combined with EXE routing.

Next connect these registered token paths with complete SDK hotkey/window
lifecycle and paired EXE input, then identify actual Alt/Esc show_menu actions,
focus/widget graph and original layout/fonts/GPU output. Paired screenshots,
GIF/video and whole-game X-Plane 12 1:1 acceptance remain outstanding.

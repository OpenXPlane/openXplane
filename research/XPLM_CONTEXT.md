# Windows XPLM initialized-TLS plugin context

Date: 2026-10-14. DLL SHA256 `706c6f1fd856c1a8c121edac2c8df12e65603e821e3f786ee8bde72543c1401f`.

## State and actual functions

`0x18001cc90` acquires the module's TLS slot (index global `0x18006a0c0`, GS:0x58 slot table), checks initialized flag TLS+0x38, and appends the full signed plugin ID to the int32 vector at TLS+0x10 (begin/end/capacity). The local replay explicitly supplies initialized TLS and sufficient reserved capacity; OS/CRT initialization and native growth/allocation failure remain unverified.

Activation `0x18001d1d0` captures the stack's last ID. ID 0 returns true unconditionally, without lazy loading or enabled lookup. Other IDs call `0x1800213f0` then `XPLMIsPluginEnabled` (`0x1800232c0`) for the **captured** ID. The enabled export validates the caller, returns zero for negative/out-of-range indices, otherwise returns the raw byte at plugin record+0x88. The record vector (`0x180069c58..0x180069c60`) has stride 0x110. The activation wrapper normalizes that byte to bool. A loader callback changing the current stack top does not change which ID gets queried.

Destructor `0x18001cf60` reads pending-unload global `0x180068290` (-1 is absent). It searches the entire current stack with original helper `0x1800312d0`; that helper finds a matching int32 and does not erase/compact the vector. If the pending ID is absent, it requests `0x18002e470(pending)` then notification `0x1800210c0`. Finally it decrements the current vector end by four, observing service changes to the stack. If a service appends a frame, the appended frame is the one removed. Actual unload/notification bodies and pending-global lifecycle remain unported.

`xp-app::xplm_context` reproduces these operations on explicitly owned per-thread state. Empty-stack use is rejected rather than inventing unsafe native memory behavior. Its `leave` returns the removed ID for Rust diagnostics; native destructor return registers are not compared. OS thread isolation, TLS initialization, recursion, allocation failures, vector lifetime and concrete module lifecycle are outside this port.

## Direct original-code evidence

`tools/gen_xplm_context_vectors.py` executes complete ctor, activation, enabled lookup, pending-ID search and destructor unchanged. Loader, caller-validation, unload and notification endpoints are controlled. The initialized TLS vector reserves 64 words, beyond all cases' sizes, so no growth helper is claimed tested.

1108 contexts compare activation, full final stack, enabled bytes, pending ID and ordered service calls. Cases cover signed/extreme IDs, absent/zero/noncanonical enabled bytes, nested contexts, pending IDs inside/outside the stack, loader changes to enabled/top, and unload appending a frame. Ordinary sequence tests retain system-ID bypass/captured-ID semantics and deferred unloading across contexts.

```sh
python tools/gen_xplm_context_vectors.py /path/to/XPLM_64.dll > /private/tmp/openxplane-xplm-context.txt
OPENXPLANE_XPLM_CONTEXT_VECTORS=/private/tmp/openxplane-xplm-context.txt cargo test --offline -p xp-app --test xplm_context_local -- --ignored
```

## Stronger hook and paired comparisons

The 1090-case DLL key-hook generator now executes these context functions and enabled lookup rather than returning a supplied activation boolean. Its previous input activation mask now initializes four actual record bytes; bit 0 cannot disable native system plugin 0. Nonzero IDs still use a logged lazy-loader service. Nonmutating hooks observe acquisition/activation/release and both implementations assert restored stack [-1] for that fixed topology with no pending unload.

The 2152-case paired EXE/DLL replay also executes SDK context functions on initialized TLS. Its explicit plugin ID 0 bypasses enabled/lazy loading natively; the Rust replay uses the recovered context state. No SDK context-result stubs remain there. EXE clock/context services, thread/caller validation, user callbacks and actual module registration/loading still remain boundaries. No live loader, OS TLS initialization, build-pairing or full game behavior is claimed.

All corpora stay local and original-code comparisons are opt-in, ignored in ordinary CI. Neither original binary is distributed.

Next: recover real module loading/unloading, validation and deferred callback-list lifecycle, then actual show_menu/Alt registrations and window effects. The viewer still exits on Esc; menus, paired screenshots and motion parity remain unfinished.

## Ownership query: XPLMGetMyID

The actual export `0x180022e50` tail-jumps to `0x18001d420`. On initialized TLS,
this function scans the int32 context vector backwards, skipping **every zero
frame**, and returns the first nonzero signed word. Empty/all-zero stacks return
0. Negative IDs are returned unchanged; there is no enabled-record query,
lazy-load request or stack mutation. This differs from activation, which reads
the final frame directly: `[7, 0]` activates system context 0 but reports owner 7.
`xplm_context::State::current_id` reproduces this ownership query.

The same local generator now also executes the complete export/body for 1657
stacks: exhaustive lengths 0..6 over `{0, -1, 7}`, all-zero lengths 0..63, and
500 seeded stacks up to 63 words including signed extremes and nested zero
frames. Both the signed result and every retained frame match exactly. No
service stub is reached by these owner cases. The existing 1108 lifecycle cases
still pass. The additional ignored Rust comparison verifies the pinned DLL
header and requires all 1657 records. The native corpus remains local.

This query is used by deferred deletion (`0x1800094f0` for sniffers and
`0x1800091a0` for hotkeys). It establishes the effective owner of a context; it
does not yet implement deletion, registration or actual menu callbacks. Native
TLS initialization (the unready branch calls `0x18002fe3c`), per-thread creation,
concurrent access and real plugin lifetime remain unverified.

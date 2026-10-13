# Complete native Begin/End command dispatch

Date: 2026-10-13. Baseline SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Ordered execution

Begin `0x140c7fac0` first checks the global block byte and recovered per-command mask query. If permitted, it reads the owner's int32 counter array (+0x80). Exact count zero records the current double at `0x142f01910` converted to float32 in timestamp array +0x98. The counter increments with 32-bit wrap, including negative counts.

End `0x140c7fc40` does not use either Begin gate. It decrements a strictly positive counter only; zero/negative counters remain unchanged, but callbacks still run. The timestamp is not cleared or refreshed by End.

Both functions then use override lookup `0x14180d3f0`, whose global ordered map head is `0x1460fddc8`, signed key +0x20 and object value +0x28. A non-null override receives virtual +0x0 for Begin or +0x10 for End and returns immediately, bypassing observers and plugin/primary processing. Counter/time changes precede this override call.

Without an override, helper `0x140c3c320` clones the owner's observer set at +0xd8. Observers are ordered by unsigned object-pointer identity. The first snapshot object is called immediately. For each later snapshot object, native code checks current membership in the live set and skips removed objects. Objects added during callbacks are absent from the snapshot and wait for the next dispatch. Original tree clone, iterator helper `0x140699650` and destruction execute unchanged in the local comparison.

After observers, plugin boundary `0x14190ff10` receives `(command, 1, 0, 1)` for Begin-before or `(command, 0, 1, 1)` for End-before. A nonzero raw int32 result permits the current primary callback in owner+0x68. The callback pointer is read after the plugin call, so changes take effect immediately. Then the plugin boundary runs again with final argument 0 regardless of the before result or primary presence. Its after result is ignored. The observer snapshot is finally destroyed.

`xp-dataref::command_dispatch` reproduces this routing with explicit command/override/observer/mask state and mutable callback/plugin services. It preserves count wrap, signed comparisons, float conversion and callback-visible mutations. Opaque object identities must match native ordering. Allocation failure, pointer replacement/lifetime, vector resizing during callbacks, concurrency, exceptions and invalid indices are outside the comparison; invalid indices return an error rather than unsafe native memory access.

## Original-code comparison

`tools/gen_command_dispatch_vectors.py` runs full Begin/End, mask query, override lookup, observer tree clone/iteration/membership checks and destruction. Only allocator/free, two virtual callback endpoints and the plugin boundary are replaced. No dispatch branch, lookup result or snapshot helper is stubbed. Controlled callbacks remove/add observers and the plugin boundary can replace the primary callback.

1768 cases compare final signed count, timestamp bits, primary identity, live observer identities and ordered callbacks/plugin calls. The deterministic matrix covers Begin/End, actual menu IDs 3/29/30 and a nonexempt command, gates, counts, override, null/present primary and plugin outcomes. Seeded cases cover catalogue endpoints, signed counter wrap, negative plugin results, arbitrary source/mask words, double-to-float time samples, observer mutation and primary replacement. These are synthetic state/effects, not the actual show_menu widget action.

```sh
python tools/gen_command_dispatch_vectors.py /path/to/X-Plane.exe > /private/tmp/openxplane-command-dispatch.txt
OPENXPLANE_COMMAND_DISPATCH_VECTORS=/private/tmp/openxplane-command-dispatch.txt cargo test --offline -p xp-dataref --test command_dispatch_local -- --ignored
```

The corpus remains local and the comparison is opt-in, ignored in ordinary CI. Ordinary sequence tests also cover repeated Begin timestamps, blocked End release, and observer changes across two snapshots.

## Esc and remaining integration

This removes a missing executor layer between keyboard Begin/End requests and actual handlers. It does not identify ID 3's registered/override/observer objects in a live engine. Their construction and concrete callback effects still need recovery. The current viewer continues using its independent command lifecycle and exits on Esc; the recovered dispatcher is not wired there yet.

Next: trace native observer/override/primary registration for show_menu, connect its actual window action and live input state, then recover Alt activation and menu layout/rendering. Paired original/openXplane screenshots and GIF/video remain required; command replays do not establish menu pixels or whole-game parity.

## Subsequent Continue extension (2026-10-14)

The current generator/test now covers 2152 combined Begin/Continue/End cases, superseding the initial 1768-case corpus above. See `COMMAND_CONTINUE.md` for elapsed/dt arguments, nested Begin and scope limits. The original Begin/End-only comparison remains historical evidence; do not interpret it as proof of held-command scheduling.

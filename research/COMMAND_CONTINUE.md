# Native held-command Continue execution

Date: 2026-10-14. Baseline SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

Function `0x140c7f8c0` completes the third native command phase. It applies the same initial global/per-command gates as Begin. Exact counter zero calls the complete Begin dispatcher; negative/nonzero counters do not. After Begin callbacks, it continues without reapplying those gates, reading the then-current timestamp and current time.

Elapsed time is `(double_now - double(float32_started))` converted once to float32. A separate float32 ramp computes `((elapsed - 1) * 3) + 1`, bounds it to 1..4 in the native comparison order, multiplies by current global dt at `0x142f018b0`, and writes the result to owner+0. This field is exposed as `rate_delta` without assigning a higher-level meaning to its users. Constants 3 and 4 come from native locations `0x14250e438` and `0x14250e4ac`.

The override/snapshot/membership/plugin/primary ordering matches Begin/End. Continue invokes virtual +0x8 with command in EDX, cached elapsed in XMM2 and the then-current global dt in XMM3. Each callback reads dt again; earlier callbacks or the before-plugin call can change later callback arguments. Plugin arguments are `(command, 0, 0, before=1/after=0)`. A non-null override bypasses other callback processing after the ramp write. A nested Begin override does not prevent the subsequent Continue override call.

`command_dispatch::continue_command` preserves nested Begin effects, counter behavior, float order, live dt reads, override priority and observer mutation. It uses `ContinueServices::continued` so elapsed/dt arguments cannot be silently lost in the Begin/End callback signature. Runtime scheduling of held commands and real callback effects remain unported; the current viewer lifecycle is independent.

## Combined native evidence

The existing `tools/gen_command_dispatch_vectors.py` now executes all three complete original functions, including nested Begin, original mask/override lookup, observer clone/iteration/membership/destruction. Allocator/free, virtual callback endpoints and the plugin boundary remain controlled services. The corpus grew from 1768 Begin/End cases to 2152 combined cases. It checks counters, timestamp bits, primary identity, observer list, owner+0 rate bits, final dt bits and ordered callback arguments. Seeded callbacks add/remove observers and mutate dt; the plugin can replace primary and dt.

Finite time/dt samples include zero/negative dt, large signed time differences, float32 timestamp rounding after nested Begin, and ramp boundaries. NaN payload/exception parity, real clock/scheduler behavior, allocation failure and callback pointer lifetime are not claimed. Ordinary sequence tests cover zero-count Continue→Begin, retained timestamps/counts and ramp progression.

```sh
python tools/gen_command_dispatch_vectors.py /path/to/X-Plane.exe > /private/tmp/openxplane-command-dispatch-v2.txt
OPENXPLANE_COMMAND_DISPATCH_VECTORS=/private/tmp/openxplane-command-dispatch-v2.txt cargo test --offline -p xp-dataref --test command_dispatch_local -- --ignored
```

The corpus stays local; original-code comparison is opt-in and ignored in ordinary CI. This verifies controlled dispatcher behavior, not the actual menu action for ID 3 or visual fidelity.

## Registration search and next work

Static startup `0x141c26694..0x141c2669c` confirms main-menu/settings registration, but does not register show_menu. The observer-removal function `0x140c6c9e0` operates on owner+0xd8; this is a useful lifecycle entry, but no ID-3 observer constructor has yet been identified. Other nearby +0xd8 references belong to unrelated structures and must not be treated as command-observer evidence.

Next: recover held-command scheduling and observer/override/primary registration for the real show_menu action, connect verified input/lifecycle to actual UI state, then recover Alt activation, layout and rendering. Paired screenshots and GIF/video remain required. No original menu is rendered in the viewer yet.

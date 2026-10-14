# Native keyboard interceptor wrapper and adapter

Date: 2026-10-13. Baseline SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Outer wrapper

`0x141913800` returns 1 without services when character byte, flags word and virtual-key byte are all zero, the registered pointer at `0x146104f88` is null, or the mode through `0x142f01eb0` is nonzero. Otherwise it reads the clock (`0x1413fa080`), optionally enters a scope through `0x1415ad510` when masks at `0x14610551c` and `0x145a16bd8` intersect, then calls the registered function with character/key bytes, full flags/phase and fifth stack argument refcon.

After the registered callback it reads the clock again, computes end minus start, and adds that double result to the then-current accumulator at `0x142f018f8`. If a scope was entered, it calls `0x1415b1920` using the saved token and the then-current context pointer at `0x146105520`. The raw signed 32-bit registered result is returned. The scope decision is cached before the callback; callback mutations of the accumulator/context are observed at their later read points.

Clock implementation, scope helper semantics and real initialization of the metadata/mode fields remain unported. These services are explicit boundaries in `xp-app::key_intercept`.

## Observed adapter and external export

A native assignment at `0x1419156a1` stores adapter `0x141917f20` into `0x146104f88`; the reset block also clears that global (`0x1419143a2`). The adapter first calls the context service through `0x146105010`, saving its signed result. It then reads export pointer `0x146106038`, forwards all five arguments when present and preserves the returned int32; an absent export returns 0. Finally it calls the restoration service through `0x146105018` only when the initial context result was >=0.

The export pointer is read after entering the hook context. An entry service can change its presence; this is covered. The outer null-registration return 1 differs from the adapter null-export return 0, so replacing both with a single default would change event consumption.

The loader's string at `0x142794b08` is `XPLMKeyPressHook`: its resolver call `0x141915684` stores the result into `0x146106038` at `0x141915689`. Adjacent wrapper assignments and export resolution are staggered; the nearby `XPLMCursorHook` string must not be used to rename the actual export read by this adapter. Full module selection, initialization ordering, replacement of the registered function and external export/context bodies are not established by this replay. No plugin DLL behavior is claimed.

## Local original-code comparison

`tools/gen_key_intercept_vectors.py` executes complete wrapper `0x141913800` and adapter `0x141917f20` unchanged. Six endpoints are logging stubs: clock, scope enter/leave, hook context enter/leave and external export. 1864 cases compare raw result, double accumulator bits, final context/export presence and callback order/arguments. A deterministic matrix covers mode/registration/export gates, signed context tokens, callback results and phases 0/2. Seeded cases include arbitrary flag/mask/refcon values, zero input, extreme signed results, time differences (including negative), export-presence changes during context entry and accumulator/context changes during the export. Inputs/times are controlled service outcomes, not original clock or external plugin execution. NaN payloads, exceptions and arbitrary registered-target replacement are outside coverage.

The initial matrix omitted an independent phase parameter and exposed an incorrect count assertion; it was expanded to both phases 0/2. All individual comparisons and the final exact 1864-case count pass.

```sh
python tools/gen_key_intercept_vectors.py /path/to/X-Plane.exe > /private/tmp/openxplane-key-intercept.txt
OPENXPLANE_KEY_INTERCEPT_VECTORS=/private/tmp/openxplane-key-intercept.txt cargo test --offline -p xp-app --test key_intercept_local -- --ignored
```

The 2152-case combined keyboard replay now executes both native interceptor layers as well as collection/query, focused UI/filter and binding loop. Its Rust replay calls `key_intercept::intercept`. Scope is disabled and clocks/context services are controlled in this combined corpus; the separate 1864-case corpus covers scope/timing branches. The external export's supplied outcomes now only apply when the actual mode gate permits invocation. Corpora remain local and comparisons are ignored in ordinary CI.

## Remaining UI and runtime fidelity

Recover external hook/context services and actual registration lifecycle when their module is available, plus action-bearing menu widgets and show_menu/Alt handlers. The viewer still uses its independent lifecycle and exits on Esc; these modules are not yet wired to live UI. Original/openXplane paired screenshots and GIF/video remain required. None of these service-boundary replays establishes whole input, plugin, menu-rendering or game parity.

## Subsequent paired module replay (2026-10-14)

The combined keyboard replay now executes the actual Windows XPLM export/body from the EXE adapter, rather than its previous export logger. See `KEYBOARD_PAIRED_REPLAY.md` for the two pinned module digests, explicit synthetic sniffer registrations and remaining validation/context/plugin callback boundaries. The separate interceptor corpus above still controls the export and is not itself a paired module run.

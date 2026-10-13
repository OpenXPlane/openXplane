# Native keyboard binding-vector transitions

Date: 2026-10-13. Baseline SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The keyboard path `0x140c64130` calls `0x140c627a0` after optional hooks/window interception. That function checks UI/plugin input, engine mode and global state before entering the binding-vector loop at `0x140c62868`. Only the loop up to exclusive `0x140c628ea` is ported as `xp_dataref::key_event::route`; entry gates and full command callback execution remain outside it.

## Observed state and order

The owner has vector begin/end pointers at +8/+16 and an int32 release-suppression field at +32. Entries have 16 bytes: int32 key, modifier mask, command ID and active flag. Key equality uses all 32 bits; modifier matching uses event flags & 7. Native active means any nonzero int32, not only 1.

For each entry whose key matches:

1. If low-three-bit modifiers match, event bit 8 is set, and the entry is inactive, clear owner suppression, store active=1, call Begin and leave the loop.
2. Otherwise, if the entry is active and event bit 16 is set, clear active regardless of the current modifiers. Call End only when owner suppression is zero; continue scanning remaining entries.

When both down/up bits are set, the Begin branch has precedence for an inactive matching entry. With duplicate key/mask entries, a second down event can activate the next inactive duplicate while the first stays active. An up event can end multiple entries in vector order. This corrects the assumption that duplicate default chords can simply be assigned a fixed winner. The separate query helper at `0x140c640d0` returns the first match, but is not the event loop.

## Local original-code comparison

`tools/gen_key_event_vectors.py` executes the exact loop in Unicorn. The original table identity is validated before construction of the supplied default-vector inputs. The two command callback functions `0x140c7fac0` (Begin) and `0x140c7fc40` (End) are replaced with logging-only stubs. Vector transitions, suppression writes and control flow execute natively. There are no other stubs in this scoped replay.

2728 cases cover all 182 default bindings under four down/up/modifier combinations plus 2000 synthetic states (empty vectors, duplicate chords, negative/noncanonical active and suppression values, extra flag bits and mismatches). Every resulting active word, owner suppression word and ordered callback trace matches the Rust port exactly.

```sh
python tools/gen_key_event_vectors.py /path/to/X-Plane.exe > /private/tmp/openxplane-key-events.txt
OPENXPLANE_KEY_EVENT_VECTORS=/private/tmp/openxplane-key-events.txt cargo test --offline -p xp-dataref --test key_events_local -- --ignored
```

The original-output corpus stays local. Ordinary CI ignores its comparison; deterministic sequence/suppression tests run normally. Callback bodies, callback effects on vector/owner state, UI/plugin gates and original OS-event delivery are not exercised. Returning an action list models inert callback bodies, not arbitrary interleaved callback mutation. This does not establish whole native input parity.

## Next connection to menus

The native Begin function at `0x140c7fac0` increments per-command counters at owner+0x80, consults overrides, then can dispatch the object pointer at owner+0x68 indexed by command ID (call at `0x140c7fc07`). The main-menu/settings callback registration is now traced in [MENU_COMMAND_ROUTING.md](MENU_COMMAND_ROUTING.md); the actual Show Menu callback still needs recovery. Native catalogue IDs are 3 for show_menu, 29 for toggle_main_menu and 30 for toggle_settings_window; these IDs are not stable across builds.

The viewer currently retains its independently implemented key lifecycle and declines ambiguous chords. The recovered stateful loop is not yet wired to it. Before connecting it, reproduce active-command continuation, interception gates and callback ordering; a first-match shortcut would discard the observed duplicate behavior. Alt/Esc menu rendering, paired captures and whole-game fidelity remain unverified.

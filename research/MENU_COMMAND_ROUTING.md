# Main-menu and settings command routing

Date: 2026-10-13. Baseline SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Registration and callback identity

The command-object setter `0x140c80080` loads owner+0x68 and stores the supplied R8 object pointer at signed EDX index × 8. Native registration routine `0x14150c860` passes its controller object to this setter for catalogue ID 29 at `0x14150c954` and ID 30 at `0x14150c97c` (toggle_main_menu and toggle_settings_window). The startup caller `0x141c26694..0x141c2669c` obtains that object through singleton getter `0x14150d020` and calls registration.

The getter constructs with `0x141505fe0`; that constructor stores primary vtable `0x1426a9cf8` at `0x14150604d`. Its first method is `0x14150bfe0`. The Begin dispatcher calls that first virtual method through the registered object at `0x140c7fc07`. This establishes the static callback path for these two commands; it does not establish every runtime override or plugin interception path.

The handler selects a jump-table entry from `0x14150c6c4` using ID minus 11. ID 29 reaches `0x14150c204`; ID 30 reaches `0x14150c06b`.

## Recovered helper requests

Main-menu command:

1. Query helper `0x141512eb0` for controller window kind 11.
2. Only when that query returns true, query `0x1417fb320` on global object `0x142f2d770`.
3. When both return true, call `0x14150ce20` with kind 11 and byte flag 1.
4. Otherwise call `0x14150e900` with kind 11 and byte flag 0, then tail-call `0x14150d370`.

Settings command tail-calls `0x14150fe90` with kind 12. It does not call the two preceding queries in this handler.

`xp-app::menu_commands::requests` preserves this short-circuiting and ordered call plan. Names CloseMainMenu/EnsureMainMenu/Refresh/ToggleSettingsWindow describe their intended roles and are anchored to the exact helpers above. The complete helper bodies and the meaning of the second engine-state predicate are not ported here. For example, the close helper queries the window, changes controller/window-system state and can clear a map entry; that is much more than changing a Rust boolean.

## Original-handler check

`tools/gen_menu_command_vectors.py` executes the full native entry `0x14150bfe0` for IDs 29 and 30 and all four boolean query combinations. Six callee bodies are replaced: the two query-return providers and four action helpers. Loggers assert the controller pointer, window kind and byte flag where those arguments are defined. The handler's branches, jump-table selection and call order execute natively.

All eight traces match the Rust request plan exactly. The corpus is local at a caller-selected path and is not distributed:

```sh
python tools/gen_menu_command_vectors.py /path/to/X-Plane.exe > /private/tmp/openxplane-menu-commands.txt
OPENXPLANE_MENU_COMMAND_VECTORS=/private/tmp/openxplane-menu-commands.txt cargo test --offline -p xp-app --test menu_commands_local -- --ignored
```

Ordinary CI ignores that local trace check. This verifies conditional routing with explicit query results, not the window-system predicates, open/close effects, widget creation, layout, rendering or visual fidelity. The request planner is not connected to the viewer; it must not replace the native services with guessed predicates or use these traces to claim working menus.

## Still unresolved

Catalogue ID 3 (show_menu, Esc) is distinct from ID 29 (toggle_main_menu). This investigation has not identified its callback. Alt-only handling is also unresolved. A large candidate at `0x140fb7af0` compares numbers 3/29/30 but emits groups of other IDs; those number matches alone do not identify it as the menu command handler.

Next: recover the six service bodies needed for these menu routes, original continuation/interception, Show Menu and Alt entry paths; connect their state and the original resource renderer, then compare matched original screenshots and motion captures. UI verified coverage remains zero.

## State-service continuation

The stronger replay in [UI_WINDOW_STATE.md](UI_WINDOW_STATE.md) now executes both queried services and their window-registry/parent-opacity dependencies without replacements. It covers 1033 state contexts and both menu commands; only the four action bodies are logged. `WindowState::menu_requests` computes the query values from those explicit states. The engine-state byte's lifecycle, real window creation and live integration remain unresolved.

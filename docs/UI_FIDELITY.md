# Original UI and graphics fidelity acceptance

The active goal requires X-Plane UI and graphics 1:1, including Alt/Esc menus. Fonts, dimensions, spacing, icons, transitions, animations, lighting and materials are part of the requirement. A redesigned prototype does not establish parity.

## Current authoritative state (2026-10-13)

The active workspace viewer exits on Escape (`crates/xp-app/src/viewer.rs`); its HUD uses a Roboto Mono bitmap atlas (`hud.rs`). The recovered default key map maps Escape to `sim/operation/show_menu` (`research/KEYMAP.md`). Original Alt behaviour has not been established by that key record.

The preserved `codex/verified-wing-legacy` source has an independent Alt/Esc menu prototype. Its own `research/UI.md` states that layout, dimensions, colors and system font were implementation choices, without original pixel comparisons. It is not a verified visual port and should not be copied into the active viewer as evidence of 1:1 fidelity.

No original font/menu resources or paired original screenshot corpus were found in the supplied reference directory during this audit. Wine/Wine64 were not found on PATH; this does not prove the absence of another installation or another machine. A usable reference UI runtime/capture source remains to be established. Existing openXplane gallery images are not reference screenshots.

## Required visual evidence

For each substantial UI/graphics change, capture the original and openXplane at the same resolution, DPI/UI scale, graphics settings, aircraft, location, camera, lighting, weather and simulation state. Preserve this metadata and reference build identity beside the captures. Mark original and port images clearly; identify every remaining difference. A pixel difference can support the comparison, but must not hide mismatched capture conditions.

Capture both Alt and Esc flows, hover/open/close transitions, keyboard/mouse selection and focus behavior in GIFs or short videos at matched timing/frame rate. Screenshots alone cannot prove animation or input behavior. Verify fonts and assets from original resources or original runtime evidence before treating a visual approximation as recovered behavior.

Next work: recover original handlers and resource paths, obtain usable original menu/font resources and matched captures, port their state/layout/rendering and compare both stationary and moving results. UI/graphics verified coverage stays zero until that evidence exists. Physics/kernel verification continues independently and does not count as UI fidelity evidence.

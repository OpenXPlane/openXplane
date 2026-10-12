# Flight input follow-up

Date: 2026-10-07. Review of the flight viewer, default keymap and pilot commands added by Claude.

The extracted key-to-command table is evidence about the original's bindings ([KEYMAP.md](KEYMAP.md)).
The command handlers and input lifecycle below are openXplane policy for its approximate flight model.
They have not been compared with the original's event processing.

## Corrections

- The held regular brake and maximum-brake latch now have separate state in `pilot::CommandInput`.
  Maximum braking takes precedence; releasing B preserves the V latch. Clearing V while B is held restores
  regular braking. Focus loss releases B but preserves the latch.
- The viewer matches key releases and repeats by physical key to the command selected on the initial press.
  Modifier/layout changes while holding a key do not change that command. Orphan repeats/releases after
  focus loss or reset are ignored until a new press.
- Focus loss clears held stick keys and immediately centres elevator, aileron and rudder input. Throttle,
  flap handle and trims remain set. Moving the cursor out of the window alone does not release keyboard input.
- Reset also clears command state, held keys and the simulation's accumulated time. The free camera returns
  to chase mode. Mouse dragging in free mode now changes the visible yaw instead of the chase-camera offset.
- Interactive flap commands use `FlightModel::flap_detent_count()` from the loaded ACF instead of assuming
  three detents. A model with no positive flap detents receives no flap step.

The stateless `pilot::apply_command` helper remains available for single-command use; interactive callers
should use `CommandInput` so held and latched brakes compose correctly.

## Validation

Regression tests cover overlapping B/V sequences, repeated toggle events, focus release of brakes,
different flap counts, modifier changes between press/repeat/release, orphan events after focus reset,
and centring axes while preserving trims, throttle and flaps.
The full test suite includes the 1200-vector profile comparison and four local Cessna flight scenarios.
The native flight-window smoke check covers startup and presentation, not interactive hardware key delivery.

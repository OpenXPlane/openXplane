# Dataref coverage of the stock Cessna 172 SP objects

Date: 2026-10-29. Tool: `tools/dataref_object_coverage.py`, fed by the reference registration table from
`tools/extract_datarefs.py` (5503 records, see [DATAREFS.md](DATAREFS.md)).

Method: collect every `/`-containing argument of `ANIM_*` commands in all OBJ files under
`Aircraft/Laminar Research/Cessna 172 SP` (index suffixes `[n]` removed), then check each name against the
reference table and against the port sources by substring search.

| Measure | Count |
|---|---:|
| Distinct datarefs referenced by animations | 201 |
| Present in the reference registration table | 144 |
| Named anywhere in the port sources | 89 at first measurement, 92 after the gear/mixture/door and flight-state datarefs |
| Not in the table (aircraft-specific `laminar/c172/...`, `CMND=` commands, dynamically registered names such as `sim/cockpit2/controls/flap_ratio`) | 57 |
| In the table but never named by the port | 55 at first measurement, 52 after those changes |

Most-used names the port does not mention: `sim/flightmodel2/misc/door_open_ratio` (22 animations),
`sim/cockpit/warnings/autopilot_test_ap_lit` (17), `sim/flightmodel2/gear/deploy_ratio` (16),
`sim/cockpit2/clock_timer/*` (timer/Hobbs/clock), `sim/graphics/view/hide_yoke`,
`sim/cockpit2/electrical/bus_volts`, `sim/cockpit2/engine/actuators/ignition_key`.

Limits: "named in the port sources" is a text match, not evidence that a value is backed by simulation
state or matches the original's accessor. The 57 names missing from the table were not classified
individually, so the table's own completeness (names built at runtime) is unproven. This is a coverage
measurement toward the Phase 2 exit check ("every dataref the stock Cessna reads has a value"), which is
not met.

## Values added after the first measurement

`FlightModel::animation_dataref` now also answers `gear/deploy_ratio[i]` (fixed gear: 1.0),
`mixture_ratio[0]` (1.0), `door_open_ratio[i]` (0.0) and, through `FlightModel::flight_dataref`, the
attitude, body-rate, speed, altitude and vertical-speed datarefs of the preview model. These are
default-state or preview-model values; none is compared with the original. Largest remaining groups:
`cockpit2/clock_timer/*`, `cockpit2/electrical/*` (bus volts, battery, generator), `cockpit2/switches/*`,
`gear/tire_rotation_angle_deg`, `cockpit2/autopilot/*`, `time/zulu_time_sec`.

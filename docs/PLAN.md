# Plan for a full implementation

The goal is an engine that loads original X-Plane 12 content and behaves like the original. This plan orders
the work by dependency. Each phase has an exit check against the original (emulator vectors, content loads, or a
recorded trajectory), not only unit tests. Shares refer to the weights in [COMPATIBILITY.md](COMPATIBILITY.md).

## Phase 1: Flight model (weight 28)

1. Wing element: finish `0x1411b6630` (delta-wing block, helpers `0x14082b800`, `0x14121bfd0`, `0x1411e1940`).
2. Force aggregation: `get_el_force` (`0x1411b9840`) with the control-surface helper `0x141221220`, supersonic blend, flaps, slats, spoilers.
3. Fuselage, stabilizers, rudder and ground effect forces; moments about the centre of gravity.
4. Mass, centre of gravity, inertia from the ACF; fuel and payload.
5. Piston engine and propeller (blade element, governor, mixture, carburetor ice), then turboprop and jet.
6. Landing gear: springs, tyre friction, brakes, steering, retraction.
7. The flight loop: integration step, atmosphere and wind, then replace the approximate model piece by piece.

Exit: recorded trajectories of the stock Cessna 172 (takeoff, cruise, stall, landing) match within a stated tolerance.

## Phase 2: Datarefs and commands (weight 8)

1. Backing values for the 5503 datarefs, grouped by owner (flight, engine, controls, view, weather).
2. Command handlers for the 3012 commands, wired to the simulation state.
3. Input: keyboard (done), mouse, joystick and yoke axes, button assignments.

Exit: every dataref the stock Cessna reads has a value; every default key binding does what the manual says.

## Phase 3: Content and rendering (weights 7 + 6 + 8)

1. OBJ8: normal and lit maps, all animation datarefs, attribute commands, instruments and panel regions.
2. Rendering: sky, sun and lighting, shadows, PBR-like materials, night lights, cockpit panel.
3. ACF: type the rest of the 1117 properties as each subsystem needs them.

Exit: the stock aircraft renders with its own textures and animations driven by datarefs.

## Phase 4: World (weight 14)

1. DSF tile reader: terrain, forests, roads, objects, polygons.
2. Global scenery and library resolution (`library.txt`, `Custom Scenery`, scenery packs).
3. apt.dat: taxi routes, start locations, lights, signs, windsocks.
4. Weather: wind, clouds, visibility, pressure, time of day.

Exit: the aircraft sits on a real runway in real terrain, with taxiways, lights and nearby objects.

## Phase 5: Cockpit and systems (weight 10)

1. Panel (PANEL_3D) and the cockpit instruments of the stock aircraft.
2. The stock Cessna's Lua scripts and system logic (electrical, fuel, avionics).
3. Autopilot, GPS and radios.

## Phase 6: Audio, multiplayer, plugins (weight 7)

1. FMOD-compatible sound banks, engine and ambient sound.
2. Networking and multiplayer.
3. The XPLM plugin interface.

## Phase 7: Platforms and distribution (weight 6)

1. A launcher and settings UI, tagged releases.
2. Packaging for macOS, Windows and Linux.

## Method for every ported function

1. Locate the function and its callers with the PE tools and the disassembler.
2. Run it in the emulator on random inputs and write the vectors.
3. Port it with the same float32/double order of operations.
4. Require identical bits (a few ulps only for libm functions) in `original_vectors.rs`.
5. Record the evidence and what is not established in `research/`.
6. Update `docs/compat.json` and the roadmap numbers.

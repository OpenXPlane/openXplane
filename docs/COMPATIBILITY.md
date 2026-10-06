# Compatibility estimate

Updated 2026-10-06. **About 31% implemented, about 16% verified identical to the original.**

A self-assessed rubric, not a measurement. Each subsystem has a weight (weights sum to 100). 'implemented' is the share of that subsystem's work that exists in any form, including approximations. 'verified' is the share whose behaviour has been confirmed identical to the original (bit for bit against its machine code, or exact by the loader/table layout in its code). The overall numbers are the weighted sums. The weights and shares are judgement calls and will change as work and understanding grow.

| Subsystem | Weight | Implemented | Verified identical | Notes |
|---|---:|---:|---:|---|
| Aircraft files (ACF) | 8 | 30% | 10% | All properties are preserved; typed: mass, CG, wings, gear, engine and controls subset. Loader schema of 1117 properties extracted; six values confirmed in code. |
| 3D models (OBJ8) | 7 | 35% | 5% | Geometry, nested transforms, rotation/translation animation, textures. No normal/lit maps, instruments, panels or most dynamic animation. |
| Airfoil evaluation (AFL) | 6 | 90% | 85% | All 34 airfoils; the whole profile function (selection, stall, buffet, blending, Mach factor) matches the original bit for bit on 1800 emulator cases. |
| Flight model | 28 | 45% | 30% | An approximate rigid-body model that takes off and flies. Verified against the original: the airfoil and per-element aerodynamics (control surfaces, wing element, supersonic regime), the whole engine update (power, propeller speed, thrust, starter) and the whole per-engine propeller force function (blade stations, airflow, ground effect, thrust and moment terms). The flight loop (update_flight), gear, wing force loop, fuel tanks, wind and the connection of these parts to the model are not ported. |
| Datarefs and commands | 8 | 28% | 15% | Registries and catalogs of 5503 datarefs and 3012 commands from the original, its default keyboard map (71 keys), values for 6 datarefs; 30 key commands act in the flight viewer. |
| Scenery and world | 14 | 10% | 0% | Airport ground from apt.dat (runways, taxiways, aprons). No DSF, terrain, objects, lights or weather. |
| Cockpit, instruments, Lua systems | 10 | 0% | 0% | Not started. |
| Rendering | 6 | 20% | 0% | wgpu viewer with lighting and glass. No PBR, shadows, sky, clouds or instrument screens. |
| Audio, weather, multiplayer, plugins | 7 | 0% | 0% | Not started. |
| Platforms and distribution | 6 | 50% | 0% | CI on Linux, macOS and Windows, nightly builds, standard folder layout. No launcher or settings UI. |

The generator is `tools/compat_score.py`; the data is `compat.json` in this folder.

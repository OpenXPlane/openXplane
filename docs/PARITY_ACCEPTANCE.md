# X-Plane 12 1:1 completion criteria

The user explicitly requires openXplane to reproduce X-Plane 12 1:1. An independent engine remains the implementation strategy; a redesigned UI, approximate flight model or partial content viewer does not fulfil the goal.

## Reference identity

Current supplied reference: X-Plane 12 Windows executable, SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`. The numeric version resource is 12.4.3.11; the user-facing release label is not confirmed. Preserve executable identity, aircraft/content identities and scenario settings with comparisons. A claim about this baseline does not automatically cover other X-Plane 12 builds.

## Required evidence

| Requirement | Evidence needed for completion | Current evidence/status |
| --- | --- | --- |
| Content loading | Original-compatible file interpretation, dependencies and resulting runtime state for the target content; explicit unsupported cases resolved | ACF/OBJ/AFL/airport subsets exist; focused loader comparisons pass; whole content loading remains incomplete |
| Original mod compatibility (mandatory) | Unmodified original aircraft/liveries, scenery/libraries, scripts and native plugins run end to end with matching dependency resolution, APIs, ABI and observable behaviour; record package/reference versions and paired scenarios as specified in GOAL.md | Partial content loaders and extension bindings exist; third-party mod compatibility is not established |
| Simulation and aircraft systems | Recovered functions wired to original caller state; complete initialization and trajectories compared under identical inputs/timesteps; controls and systems behaviour reproduced | Many kernels verified; viewer still approximate; complete trajectory and systems parity unproven |
| Reference geometry and rendering inputs | Original geometry preparation, inclusion/update state and complete live reference fields compared on real aircraft | Main point-vector/reducer assembly verified on synthetic state; body-grid loading checked on real aircraft; live integration incomplete |
| World and environment | Matching scenery, terrain, objects, lighting, weather and environment-dependent simulation in matched scenarios | Airport ground subset; DSF/world/weather parity incomplete |
| Alt/Esc and other UI | Original handler/state/layout/font/asset behaviour; paired still images and synchronized interaction/motion captures | Active viewer exits on Esc; legacy menu is an unverified prototype; original menu/font resources are available locally, but matched original captures and live UI integration remain missing |
| Graphics and animation | Paired screenshots and GIF/video with identical resolution, DPI/UI scale, scene, camera, graphics settings, lighting/weather and animation state; remaining differences resolved | Approximate renderer; no paired original visual proof |
| Audio, scripts, commands and supported extension behaviour | Original-equivalent observable behaviour, values, effects and timing through the actual runtime | Catalogs and partial bindings exist; complete audio/script/plugin and command behaviour incomplete |
| Complete engine operation | Original workflows and complete scenarios exercised end to end, across required platforms; all scoped differences and unsupported requirements resolved | Cross-platform CI is green for the current subset; it does not establish complete simulator equivalence |

A verified kernel or green CI job proves only the behaviour actually covered. Local tests skipped or explicitly ignored in CI remain separate evidence. Recorded CRT inputs, allocator substitutions, synthetic state and numerical tolerances must be disclosed. Tolerated differences are unresolved differences for the overall 1:1 goal, even when useful for diagnosing a recovered stage.

## Visual proof

Follow [UI_FIDELITY.md](UI_FIDELITY.md). Keep original and port captures clearly labelled with matched-condition metadata. Still images cover layout and appearance; GIF/video must additionally cover transitions, animation, focus, selection and timing. Existing openXplane gallery images are not original reference captures. Do not label a result 1:1 while observed differences remain.

## Completion audit

Before marking the goal complete, inspect the actual current implementation and evidence for every requirement above and every explicit user requirement. Resolve remaining missing, approximate, unverified or unsupported behaviour. Whole-game parity must be demonstrated at whole-game scope; do not infer it from a collection of isolated functions or a progress percentage. Compatibility estimates in `compat.json` are a self-assessed planning rubric and cannot serve as a completion gate.

The goal remains active while this table contains incomplete or unproven requirements. Continue recovering and integrating original behaviour, publishing verified changes and updating the roadmap and limitations after substantial confirmed progress.

# Active goal and completion requirements

Continue recovering behaviour from the supplied X-Plane 12 reference and implementing the independent openXplane engine. Compare recovered behaviour against original code, commit and push verified changes, and distinguish confirmed behaviour from approximations. The completion target is openXplane reproducing X-Plane 12 as a whole 1:1, including simulation, content behaviour, UI and graphics. Individual recovered kernels, an approximate flying viewer or weighted progress estimates do not establish completion. See [PARITY_ACCEPTANCE.md](PARITY_ACCEPTANCE.md) for the required evidence.

## Required maintenance after substantial confirmed progress

1. Review `docs/compat.json` against the new evidence. Reassess subsystem percentages; keep them unchanged when evidence does not justify broader coverage. Explain any increase or decrease.
2. Update the roadmap and remaining work in that file.
3. State reference build, executed checks, skipped checks, stubs, controlled inputs, platform tolerances and runtime integration limitations where relevant.
4. Run `python3 tools/compat_score.py --write` to regenerate README and COMPATIBILITY. Do not edit generated percentages separately.
5. Commit and push the documentation and site update. The main-branch Pages workflow publishes the shared compatibility data. Verify deployment before claiming the live site has updated.

This is a persistent addition requested by the user on 2026-10-13. It applies throughout subsequent goal work.

## UI and graphics fidelity requirement

User requirement added on 2026-10-13: reproduce the original X-Plane UI and graphics 1:1. This includes the Alt and Esc menus, fonts, dimensions, spacing, icons, transitions and animations, lighting and materials. An approximate or redesigned appearance does not satisfy this requirement.

Verify visual fidelity with paired screenshots of openXplane and the reference at the same resolution, settings and scene. Verify transitions, animation and behaviour in motion with GIFs or short videos. Provide visual evidence of substantial UI/graphics progress and identify remaining differences. Do not call the result 1:1 while differences remain. Visual evidence supplements, rather than replaces, original-code and physics comparisons.

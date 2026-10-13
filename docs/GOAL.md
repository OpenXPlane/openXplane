# Active goal and completion requirements

Continue recovering behaviour from the supplied X-Plane 12 reference and implementing the independent openXplane engine. Compare recovered behaviour against original code, commit and push verified changes, and distinguish confirmed behaviour from approximations. Whole-game compatibility remains the goal; individual recovered kernels do not establish it.

## Required maintenance after substantial confirmed progress

1. Review `docs/compat.json` against the new evidence. Reassess subsystem percentages; keep them unchanged when evidence does not justify broader coverage. Explain any increase or decrease.
2. Update the roadmap and remaining work in that file.
3. State reference build, executed checks, skipped checks, stubs, controlled inputs, platform tolerances and runtime integration limitations where relevant.
4. Run `python3 tools/compat_score.py --write` to regenerate README and COMPATIBILITY. Do not edit generated percentages separately.
5. Commit and push the documentation and site update. The main-branch Pages workflow publishes the shared compatibility data. Verify deployment before claiming the live site has updated.

This is a persistent addition requested by the user on 2026-10-13. It applies throughout subsequent goal work.

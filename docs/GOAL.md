# Completion goal and working agreement

Updated2026-10-26 at the user's request. This document supplements the active
chat goal; it does not replace or narrow it.

Deliver a complete, runnable and playable openXplane with behavior and content
compatibility1:1 against the installed X-Plane12. Isolated ports, selected green
tests and estimated percentages do not establish completion.

The local reference installation is
`E:\Games\steamapps\common\X-Plane 12`. Read original content in place for
research and compatibility. Do not distribute original code or game resources.
Use REA and comparisons with actual original instructions for recovered behavior.

Integrate content loading, menus/settings, controls, flight model, scenery,
cockpit/systems, graphics, sound and required plugin support into the running
game. Regularly run the reference and port in comparable scenarios, make new
paired screenshots and, when useful, GIF/video captures, investigate differences
and retain reproducible evidence. A single-engine render is not a paired visual
comparison. Keep large/private reference artifacts under ignored local research.

## Required Git delivery

For every completed coherent change:

1. Run the relevant checks and review the diff.
2. Commit the change with a clear message describing its final behavior.
3. Push that commit to the configured remote; do not defer a batch of finished
   changes indefinitely or leave completion only in the working directory.
4. Verify the push and report the commit and any actual delivery blocker.

Preserve unrelated work and do not force-push. When several completed changes
have accumulated, commit and push them as separate coherent changes. Temporary
unfinished edits may remain local while work continues. A push failure must be
reported explicitly and retried when the actual blocker has been resolved.

The active goal stays open until the complete game and all required evidence
are delivered. See [the implementation plan](PLAN.md) and
[the compatibility limits](COMPATIBILITY.md).

See [PARITY_ACCEPTANCE.md](PARITY_ACCEPTANCE.md) for the detailed acceptance evidence.

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

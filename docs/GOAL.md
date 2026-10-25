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

# Reading live datarefs from the reference game (attempt)

Date: 2026-10-29. Goal: get reference values for datarefs (default switch states, trajectories) from the
running X-Plane 12 to check Phase 1/2 work.

## What was tried

Frida attached by PID to a freshly started `X-Plane.exe` (Steam launch, attach at the main menu), then a
script resolved `XPLM_64.dll` and called its exports `XPLMFindDataRef` / `XPLMGetDataf` directly from the
Frida thread.

## Result

Both attempts ended with the game terminating while the script loaded (the Frida connection reported
"The connection is closed"). The game's `Log.txt` ends with:

    An unknown plugin has bypassed XPLM when calling SDK functions. This is a plugin bug.

Interpretation (not confirmed beyond the log text and the timing): XPLM identifies the calling plugin and
aborts when an SDK function is called from code that is not a registered plugin. A direct call from an
injected thread therefore cannot be used to read datarefs. (An earlier unattended start also exited with
that log line, because the installed `XP Walkaround` plugin logs "Sim is not yet started" in a loop, so the
line alone does not prove the Frida call was the trigger; the two attach attempts did coincide with it.)

## Remaining routes (not tried)

- Call the EXE's own accessor functions from the registration table (`tools/extract_datarefs.py`: getter VAs
  per record) with the refcon the record stores, bypassing the XPLM caller check. Needs the record's refcon
  slots (14/15 were not interpreted in `DATAREFS.md`).
- Use the game's own Data Output (Settings > Data Output) to write selected datarefs to a file or UDP and
  read that, which needs UI automation but no injection.
- Write a small plugin loaded from `Resources/plugins` that reads datarefs and logs them. That is the
  supported XPLM route; it has to be built for the installed plugin ABI and installed into the reference tree,
  which modifies the reference installation and was not done.

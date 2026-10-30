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

## Lead for the Data Output route (found, not exercised)

`Output/preferences/X-Plane.prf` has 173 keys `_Bdisk0` .. `_Bdisk172` (value 0 in the reference tree), one per
data-output line, in the same index order as `crates/xp-app/assets/dout/lines.tsv` (`dout_lines.cpp` builds
indices `0..0xac`, which is 173 lines). The name and the count match the "write to disk" column of the
Data Output screen; that the key actually switches the disk file on, and that the file is `Data.txt` in the
simulator root, are expected from X-Plane's behaviour and were not tested here. Lines useful for a trajectory
comparison by index: 1 (times), 3 (airspeeds), 4 (Mach, VVI, g-loads), 8 (stick), 13 (trim, flaps, speedbrakes),
16 (P, Q, R), 17 (pitch, roll, headings), 18 (alpha, beta, flight path), 20 (latitude, longitude, altitudes),
21 (position and velocity in metres), 25 (throttle handle).

A run would need: the keys above set to 1 while the game is closed (back up and restore the prefs file, since it
sits in the reference tree), a started flight of the stock Cessna 172 SP at a known airport with a scripted
control sequence, and the resulting `Data.txt` paired with the port's `FlightModel` driven by the same inputs
and time step. None of this was done; the Phase 1 exit check (recorded Cessna trajectories matching within a
stated tolerance) remains unmet.

## Data Output route confirmed (2026-10-30)
With `_Bdisk0,1,3,4,17,18,20` set to 1 in `Output/preferences/X-Plane.prf` and a flight loaded ("Resume last flight"), the reference writes `Data.txt` in the install root: a 1.6 MB table within about a minute, with a header row of column names (`f-act,/sec`, `Vind,kias`, `VVI,fpm`, `pitch,deg`, `hding,true`, `alpha,deg`, `lat/lon`, `CG,ftMSL`, ...) and one fixed-width row per sample. This settles the earlier "unverified that they write Data.txt". The resumed flight was parked at UHWW in a glass-cockpit aircraft that was not identified as the Cessna 172 SP, so no C172 trajectory exists yet; the prefs file was restored from backup afterwards (hash unchanged).

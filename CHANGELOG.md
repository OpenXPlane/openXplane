# Changelog

All notable changes. The newest is first. Versions are `MAJOR.MINOR.COMMITS` (see `scripts/version.sh`).

## 0.1

- Workspace of crates by subsystem (`xp-acf`, `xp-airfoil`, `xp-obj`, `xp-scenery`, `xp-dataref`, `xp-sim`,
  `xp-discord`, `xp-app`), with scripts, CI and nightly builds.
- Readers for ACF, OBJ8, AFL and apt.dat; the standard X-Plane folder layout and its precedence.
- The airfoil evaluation chain, the profile function and the straight-wing path of the wing element function,
  verified bit for bit against the original machine code in an emulator.
- Registries and catalogs of 5503 datarefs and 3012 commands, and the original's default keyboard map.
- An approximate flight model on the Cessna's real data, an airport ground scene from apt.dat, a chase-camera
  viewer (`fly`) and frame renderer (`fly-render`).
- Discord Rich Presence, the project website and the research notes.

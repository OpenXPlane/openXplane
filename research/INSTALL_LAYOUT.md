# Installation folder layout and which folder wins

Date: 2026-10-06. The reference and SHA256 are in [BASELINE.md](BASELINE.md). Evidence is the set of strings in
`X-Plane.exe` plus the layout of a standard installation; the original was not run, and the code that builds
these paths was not disassembled. So the folder names are confirmed by strings, but the precedence rules below
are labelled by how well they are supported.

## Standard layout

A standard X-Plane 12 installation has these folders next to `X-Plane.exe`: `Aircraft`, `Airfoils`, `Custom
Data`, `Custom Scenery`, `Global Scenery`, `Output`, `Resources`, plus `Instructions`, `Support`, `Weapons` and
the Plane Maker / Airfoil Maker tools. Strings found in the executable:

| Folder or path | Evidence in the EXE |
|---|---|
| `Aircraft/<maker>/<aircraft>/<name>.acf` | for example `Aircraft/Laminar Research/Cessna 172 SP/Cessna_172SP.acf` |
| `<aircraft>/objects/` | `Aircraft/Fighters/F-4 Phantom/objects/F4D_Fuselage.obj` |
| `Airfoils/` and an aircraft's `airfoils/` | both spellings are present as folder strings |
| `Custom Data/` | `pilot-defined waypoints to FIX file in Custom Data/ folder` |
| `Resources/default data/` | warning about non-standard files in this folder |
| `Custom Scenery/`, `scenery_packs.ini`, `SCENERY_PACK` | the strings exist; the file format is not read from the code |
| `Global Scenery/Global Airports/Earth nav data/apt.dat` | full path string, also `../Global Scenery/Global Airports/Earth nav data/` |
| `Resources/default scenery/default apt dat/` | folder string |
| `Output/` | with `diagnostic reports/`, `FDR files/`, `flight model test/`, `logbooks/`, `Log Archive/`... |

## Precedence

| Rule | Support |
|---|---|
| Navigation data (`earth_nav.dat`, `earth_fix.dat`, `earth_awy.dat`): `Custom Data/` before `Resources/default data/` | The build's own message says a navigation data update "needs to go into the Custom Data/ folder", not into `Resources/default data/`. Together with the three file names this supports the override; the lookup order itself was not read from code. |
| Airfoil files: the aircraft's `airfoils/` before the shared `Airfoils/` | Both folders exist in the strings; the order is our policy (the first existing file is used; `inspect` still audits every existing candidate). |
| Airports: enabled scenery packs in `scenery_packs.ini` order (first line wins), then Global Airports, then the default apt.dat | The folders and the `SCENERY_PACK` keyword are in the EXE; the priority order is taken from X-Plane's public documentation and is not confirmed in the build. |
| The EXE warns that Global Airports "should no longer exist under Custom Scenery" and that without them in Global Scenery "we don't know about the airports and runways of the world" | Confirms Global Airports belongs under `Global Scenery/`. |

## Implementation (`src/install.rs`)

`Install::open(root)` gives the folder paths and these lookups: `find_acf` (recursively under `Aircraft/`),
`nav_data_file`, `airfoil_candidates`, `scenery_packs`, `apt_dat_sources` and `find_airport`. Pack lines of the
form `SCENERY_PACK <path>` are read in file order and `SCENERY_PACK_DISABLED` lines are skipped; the
disabled-pack spelling comes from public documentation, not from the build. A missing `scenery_packs.ini` means
no custom packs are considered.

The minimal "flat" layout used while only a few files were provided (`Cessna 172 SP/`, `Airfoils/`,
`Earth nav data/` next to the EXE) is still accepted: aircraft are searched below the root when there is no
`Aircraft/` folder, and `Earth nav data/` at the root is consulted last.

```sh
cargo run --offline -- inspect <installation>
cargo run --offline -- airport-info <installation> KSEA
```

`inspect` prints which standard folders exist, where each navigation data file was found and the `apt.dat`
sources in priority order; `airport-info` given a directory returns the first match and says which file it
came from.

## Not done

The `scenery_packs.ini` header lines are not validated; `library.txt` and the library search order, scenery
pack `Earth nav data` DSF folders, plugin folders, `Output` writing, and the aircraft `plugins/` and `cockpit/`
folders are not modelled. Case-insensitive matching as on Windows is not emulated on other platforms.

## Checks

44 unit tests pass, Clippy without warnings. The new tests build temporary installations and check that
`Custom Data` overrides `Resources/default data`, that airports resolve through packs, Global Airports and the
flat layout in priority order and skip disabled packs, that aircraft are found in nested folders, that the
airfoil candidates prefer the aircraft folder, and that pack lines keep their order. On the provided flat
`Xplane12/` the `inspect` and `airport-info` commands behave as before.

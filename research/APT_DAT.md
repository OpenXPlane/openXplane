# apt.dat: reading airports

Date: 2026-10-06. Source: the file `Xplane12/Earth nav data/apt.dat` itself (380 MB, 12.35 million lines, format
`1200`, data cycle 2105 in `earth_nav.dat`). The original EXE was not analysed for this part: the row structure
is taken from observations of the file, so the rules below are observed, not confirmed by disassembly.

## What is implemented (`src/apt.rs`)

- The marker `I`/`A`, then the version line `1200`; any other version is rejected.
- Airport headers: codes 1, 16, 17 (`code elevation_ft - - ICAO name...`). An airport block ends at the next
  header or at code 99.
- Row 100 (land runway): 7 shared fields and two ends of 9 fields each (name, latitude, longitude, displaced
  threshold, overrun, markings, approach lights, touchdown-zone lights, REIL). Short rows are an error with the
  line number.
- Row 1302: key/value metadata pairs, order preserved.
- Pavements: row `110` with the node rows `111`-`114` that follow it (loops closed by `113`/`114`, bezier control points on `112`/`114`); see [WORLD_GEOMETRY.md](WORLD_GEOMETRY.md).
- Other codes (in KSEA: 14, 18-21, 110-116, 120, 130, 1000-1056, 1101, 1110, 1200-1206, 1300-1301, 1400-1401,
  1500) are not interpreted, only counted.

Reading is streamed: `find_airport` stops at the end of the requested block and does not keep the file in
memory. Looking up the last airport reads the whole file.

## Verification

`cargo run --offline -- airport-info "Xplane12/Earth nav data/apt.dat" KSEA` prints the three runways 16L/34R,
16C/34C, 16R/34L; the coordinates of the ends and the metadata match the source lines of the file. An unknown
ICAO gives exit code 2.

## Not done

The units and values of the surface, marking and lighting codes are not decoded; water runways (101), helipads
(102), linear features (120), parking (1300), frequencies (50-56 in the old format, 1050+ in the new one), DSF,
library objects and the link to terrain are not read. There is no index over the file: every query is a linear
pass.

# Installed-content baseline, 2026-10-26

The reference installation was found at
`E:\Games\steamapps\common\X-Plane 12`. It includes the three stock Cessna172
ACF variants, resources, custom scenery and Global Airports. Reference executable
identity remains the SHA256 recorded in [BASELINE.md](BASELINE.md).

An actual `airport-render` against this installation initially failed before
reaching KSEA: the enabled Faro pack declares apt.dat1130, but the port accepted
only1200. After admitting1130, a second enabled pack declared1000 and failed
the same way. The enabled installation also contains1100 and1200 files.

The parser now admits1000,1050,1100,1130 and1200 for its existing typed subset
(airport headers, land runway fields, metadata and pavement outlines). Unknown
rows remain counted, not implemented. Future/unknown versions still fail.
This follows the published format history, not a newly recovered native parser:
[Laminar1200 specification](https://developer.x-plane.com/article/airport-data-apt-dat-12-00-file-format-specification/)
states that1200 extends1130 and retains earlier1000/1050 support;
[Laminar1100 specification](https://developer.x-plane.com/article/airport-data-apt-dat-file-format-specification/)
documents the preceding extension. No whole-parser/native error parity is claimed.

The synthetic parser regression compares all typed fields across the accepted
headers. Installation priority coverage uses an1130 custom pack before1200
Global Airports and confirms lookup continues to a different global airport.

The repaired real lookup reaches KSEA in
`Global Scenery/Global Airports/Earth nav data/apt.dat`: runway16L/34R,
length3631m and63 pavements. The current renderer loads the Cessna's seven
exterior OBJ meshes and produces a new1280x800 PNG:

![Current openXplane KSEA installed-content render](../site/assets/scenery-legacy-20261026.png)

Reproduction:

```powershell
openxplane airport-render '<X-Plane>' KSEA '<X-Plane>/Aircraft/Laminar Research/Cessna 172 SP/Cessna_172SP.acf' '<output.png>'
```

This is an offscreen openXplane render, not an original-window screenshot or a
paired visual comparison. It shows the current flat airport geometry and static
exterior approximation; buildings, terrain textures, lighting/material fidelity,
cockpit and full runtime state remain incomplete. It must not be used as1:1
visual evidence or as proof that the reference was launched.

Both available Computer Use runtimes failed before initialization with
`failed to write kernel assets: ... (os error 3)`. No UI input was issued and no
new original screenshot was obtained. Restoring window capture and obtaining
matching original/port captures remain open work under [the goal](../docs/GOAL.md).
Original content was read in place; no original source assets were copied into
the repository. Logs and full installation inventories remain ignored/local.

Follow-up: [the reference FPS investigation](REFERENCE_FPS_CAPTURE.md) now
confirms actual reference execution and clean completion. Automatic image
capture remains unresolved; no paired original frame has been obtained.

# Reference build and first observations

Date of the investigation: 2026-10-06. All values below come from local files.

## Executable

- File: `Xplane12/X-Plane.exe`, 91,132,488 bytes.
- Format: Windows PE32+, AMD64 (machine `0x8664`).
- SHA-256: `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
- Version candidate from VS_FIXEDFILEINFO: file/product `12.4.3.11`.
  This is the numeric value of the resource; the user-facing release name has not been checked.
- The import table lists `steam_api64.dll`, `d3d11.dll`, `dxgi.dll` and system Windows DLLs. The import
  table alone does not determine the main graphics API: libraries may be loaded dynamically.
- Full local report: `research/local/pe-inventory.json` (not published).

The binary was not run. The physics algorithms are not recovered yet. The presence of a string or a library
name is treated as a hint for research, not as proof of a function's behaviour.

## Aircraft

| ACF | Format version | Properties | Unique OBJ/AFL references | Not found |
|---|---:|---:|---:|---:|
| Cessna_172SP.acf | 1200 | 30335 | 47 | 0 |
| Cessna_172SP_G1000.acf | 1200 | 30065 | 32 | 0 |
| Cessna_172SP_seaplane.acf | 1200 | 30724 | 50 | 0 |

Only the `/_afl_file_*` and `/_v10_att_file_stl` properties were checked. Textures, panels, scenery libraries
and script dependencies were not checked.

Observed layout: objects are relative to `objects/`, including `../` for the root cockpit OBJ; airfoils are in
the aircraft's `airfoils/` or the shared `Airfoils/`. The tool checks that the candidates exist. The priorities
and search rules of the original engine still need confirmation.

The stock aircraft contains XLua and seven script groups: custom_datarefs, electrical, fuel_selector, IAS, init,
sound, starter. The scripts access `sim/...` datarefs, create `laminar/c172/...` datarefs and commands, and use
the events `aircraft_load`, `flight_start`, `after_physics`. So aircraft compatibility will need a runtime, not
only an OBJ loader.

There are an `apt.dat` of format 1200 and an `earth_nav.dat`. No terrain DSF files or shared scenery libraries
were found in the provided set; they are not needed for the first aircraft view.

## Method

For every rule we record the source: a specification, a content property, static analysis or a measurement in
the original. Unknown fields are kept; unsupported behaviour is marked explicitly. Comparison results must state
the reference build, the input content, the run conditions and the tolerance.

Example of the approach: [openOMSI architecture](https://github.com/openOMSI-Project/openOMSI/blob/main/docs/ARCHITECTURE.md)
and [format research](https://github.com/openOMSI-Project/openOMSI/blob/main/docs/FORMATS.md).

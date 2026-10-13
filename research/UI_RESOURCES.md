# Original menu resource loading

Date: 2026-10-13. Baseline EXE SHA256: `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The supplied local installation now includes original fonts and interface11 bitmaps. `xp-app::ui_resources::UiResources::load` reads these local files. Fonts remain intact sfnt bytes; PNG and DXT5 DDS textures decode to RGBA8 at their original dimensions, without resizing or premultiplication. Missing or undecodable files fail with their path; no replacement font or texture is substituted. This is resource loading for the UI port, not a working original menu renderer.

## Static native bindings

LLVM objdump disassembly of this exact baseline identifies the following RIP-relative LEA references in the main-menu construction region. Each references the indicated complete string in `.rdata`:

| Instruction VA | String VA | Resource name |
|---|---|---|
| `0x140eccb26` | `0x142609148` | `main_menu/menu_resume.dds` |
| `0x140eccc9c` | `0x142609180` | `main_menu/menu_new.dds` |
| `0x140eccea4` | `0x1426091a8` | `main_menu/menu_save.dds` |
| `0x140ecd03e` | `0x1426091d0` | `main_menu/menu_saved.dds` |
| `0x140ecd1cb` | `0x142609208` | `main_menu/menu_school.dds` |
| `0x140ecd32f` | `0x142609238` | `main_menu/menu_settings.dds` |

The resume path flows via RDI into R9 at `0x140eccbe1`, followed by a call to `0x140ec4530` at `0x140eccbec`. That constructor calls `0x14129ca30` at `0x140ec4624`. Its callback, layout and styling semantics are not yet recovered. These are static references, not observed UI execution. A raw byte displacement scan also produced a false command-record candidate at `0x1412d9130`: disassembly establishes it is an unrelated conditional jump. It is not an incoming reference to the command table.

The Roboto-Regular path is passed to `0x14129a700` at several call sites in `0x1412b3223..0x1412b3433`, using varying scalar parameters. Font-role mapping and the exact rasterization implementation remain unresolved. Do not infer that the viewer's monospaced HUD matches the native menu.

## Local file evidence

| Texture | Base dimensions | Supplied density variants |
|---|---|---|
| menu_checked | 8 × 8 | 1, 1.5, 2 |
| menu_right_arrow | 8 × 12 | 1, 1.5, 2 |
| menu_popup_body | 44 × 36 | 1, 1.5, 2 |
| menu_down_arrow | 20 × 12 | 1 |
| menu_popup_item | 28 × 21 | 1 |
| menu_separator_hor | 18 × 7 | 1 |
| menu_top_level_bg | 30 × 60 | 1 |
| Six main_menu textures listed above | 512 × 512 | 1, 2 |

All twelve supplied main-menu DDS files have DXT5 headers. Density selection is explicit in the file catalog: the original automatic DPI thresholds, fallback behavior, sprite-state boundaries and nine-slice borders are not established.

The loader checks sfnt headers and preserves font bytes; it does not yet parse glyphs or render text. The twelve fonts are Roboto Regular/Light/Bold/Condensed, DejaVu Sans/Mono, Noto Sans CJK SC, Tahoma Bold, B612 Mono, Inconsolata, Menlo Regular and ProFont Windows. File presence does not identify their native UI roles.

## Checks and limitations

`OPENXPLANE_REFERENCE_ROOT=/path/to/Xplane12 cargo test --offline -p xp-app --test ui_resources_local -- --ignored` requires every resource, checks all 25 decoded dimensions and requires the resume DDS alpha channel to survive decoding. This checks our codec output against supplied file dimensions and alpha presence. It does not compare GPU output with the native decoder, pixel colors with a second decoder, original font metrics, menu behavior, or original screenshots. Ordinary CI explicitly ignores the local-resource test; original assets are not distributed.

`cargo run --offline -p xp-app -- ui-resources /path/to/Xplane12` loads all files and prints their dimensions/font lengths, making missing local input concrete. No derived preview or original asset is committed. The active viewer still exits on Esc, and Alt/Esc handler semantics, matched screenshots and motion comparisons remain outstanding. Compatibility percentages remain unchanged.

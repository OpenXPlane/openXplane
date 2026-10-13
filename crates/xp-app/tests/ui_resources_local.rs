//! Explicit local-resource check. Original assets are never bundled in this repository.
use openxplane::ui_resources::UiResources;
use std::path::PathBuf;

#[test]
#[ignore = "requires original Resources under OPENXPLANE_REFERENCE_ROOT"]
fn original_menu_resources_decode_without_resampling() {
    let root = PathBuf::from(
        std::env::var_os("OPENXPLANE_REFERENCE_ROOT")
            .expect("set OPENXPLANE_REFERENCE_ROOT to the original installation"),
    );
    let resources = UiResources::load(&root).expect("all original menu files must load");
    assert_eq!(resources.fonts.len(), 12);
    assert_eq!(resources.textures.len(), 25);
    for texture in &resources.textures {
        let name = &texture.relative_path;
        let density = if name.contains("@2x") {
            2.0
        } else if name.contains("@1.5x") {
            1.5
        } else {
            1.0
        };
        let base = if name.starts_with("main_menu/") {
            (512, 512)
        } else if name.starts_with("menu_checked") {
            (8, 8)
        } else if name.starts_with("menu_right_arrow") {
            (8, 12)
        } else if name.starts_with("menu_popup_body") {
            (44, 36)
        } else {
            match name.as_str() {
                "menu_down_arrow.png" => (20, 12),
                "menu_popup_item.png" => (28, 21),
                "menu_separator_hor.png" => (18, 7),
                "menu_top_level_bg.png" => (30, 60),
                _ => panic!("unreviewed texture {name}"),
            }
        };
        assert_eq!(
            texture.pixels.dimensions(),
            (
                (base.0 as f32 * density) as u32,
                (base.1 as f32 * density) as u32
            ),
            "{name}"
        );
    }
    let resume = &resources
        .textures
        .iter()
        .find(|t| t.relative_path == "main_menu/menu_resume.dds")
        .unwrap()
        .pixels;
    assert!(
        resume.pixels().any(|p| p.0[3] != 255),
        "DDS alpha must survive decoding"
    );
}

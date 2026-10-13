//! Local original UI resources. Decoding does not establish menu layout or rendering parity.
//! Density is explicit: the original automatic DPI-selection policy is not recovered.
use image::RgbaImage;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const FONTS: &[&str] = &[
    "Roboto-Regular.ttf",
    "Roboto-Light.ttf",
    "Roboto-Bold.ttf",
    "RobotoCondensed-Regular.ttf",
    "DejaVuSans.ttf",
    "DejaVuSansMono.ttf",
    "NotoSansCJK-SC-Regular.otf",
    "tahomabd.ttf",
    "B612Mono-Regular.ttf",
    "Inconsolata.ttf",
    "Menlo-Regular.ttf",
    "ProFontWindows.ttf",
];

#[derive(Clone, Copy, Debug)]
pub enum Density {
    One,
    OneAndHalf,
    Two,
}

impl Density {
    fn suffix(self) -> &'static str {
        match self {
            Self::One => "",
            Self::OneAndHalf => "@1.5x",
            Self::Two => "@2x",
        }
    }
}

/// Known files, including full sprites. No assumptions about state-strip or nine-slice boundaries.
pub fn menu_texture_paths() -> Vec<String> {
    let mut paths = Vec::new();
    for name in ["menu_checked", "menu_right_arrow", "menu_popup_body"] {
        for density in [Density::One, Density::OneAndHalf, Density::Two] {
            paths.push(format!("{name}{}.png", density.suffix()));
        }
    }
    for name in [
        "menu_down_arrow",
        "menu_popup_item",
        "menu_separator_hor",
        "menu_top_level_bg",
    ] {
        paths.push(format!("{name}.png"));
    }
    for name in [
        "menu_new",
        "menu_resume",
        "menu_save",
        "menu_saved",
        "menu_school",
        "menu_settings",
    ] {
        for density in [Density::One, Density::Two] {
            paths.push(format!("main_menu/{name}{}.dds", density.suffix()));
        }
    }
    paths
}

pub struct MenuTexture {
    pub relative_path: String,
    /// RGBA8 pixels at original dimensions, without resampling or premultiplication.
    /// Gamma, filtering and blend state of the original renderer remain unverified.
    pub pixels: RgbaImage,
}

pub struct FontResource {
    pub name: &'static str,
    /// Original sfnt bytes; kept intact for a future font renderer.
    pub bytes: Vec<u8>,
}

pub struct UiResources {
    pub fonts: Vec<FontResource>,
    pub textures: Vec<MenuTexture>,
}

fn failure(path: &Path, error: impl std::fmt::Display) -> String {
    format!("{}: {error}", path.display())
}

impl UiResources {
    /// All resources are required. An absent original file is an error, never a substitute asset.
    pub fn load(installation_root: &Path) -> Result<Self, String> {
        let resources = installation_root.join("Resources");
        let mut fonts = Vec::new();
        for &name in FONTS {
            let path = resources.join("fonts").join(name);
            let bytes = fs::read(&path).map_err(|e| failure(&path, e))?;
            if bytes.len() < 12 || !matches!(&bytes[..4], b"\0\x01\0\0" | b"OTTO" | b"true") {
                return Err(failure(&path, "expected an sfnt font header"));
            }
            fonts.push(FontResource { name, bytes });
        }
        let mut textures = Vec::new();
        for relative_path in menu_texture_paths() {
            let path: PathBuf = resources.join("bitmaps/interface11").join(&relative_path);
            let pixels = image::open(&path)
                .map_err(|e| failure(&path, e))?
                .into_rgba8();
            textures.push(MenuTexture {
                relative_path,
                pixels,
            });
        }
        Ok(Self { fonts, textures })
    }
}

//! Native simulator menus. Labels/commands observed in the reference; layout is our implementation.
use ab_glyph::{Font, FontArc, PxScale, ScaleFont, point};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Page {
    #[default]
    Flight,
    Main,
    Configuration,
    Settings,
    Keyboard,
    About,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Resume,
    StartFlight,
    Restart,
    Pause,
    KeyboardStick,
    ChaseCamera,
    FreeCamera,
    Quit,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Click {
    Page(Page),
    Action(Action),
    Hud,
    NextAircraft,
    AirportInput,
    Menu(usize),
}
#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}
impl Rect {
    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

pub struct Info {
    pub flying: bool,
    pub paused: bool,
    pub keyboard_stick: bool,
    pub telemetry: Vec<String>,
}

pub struct Menus {
    pub page: Page,
    pub setup: Option<crate::flight_setup::FlightSetup>,
    pub bar: bool,
    pub hud: bool,
    dropdown: Option<usize>,
    keyboard_focus: bool,
    font: FontArc,
    hits: Vec<(Rect, Click, bool)>,
    hover: Option<usize>,
}

impl Menus {
    pub fn new() -> Result<Self, String> {
        let mut paths = vec![
            PathBuf::from("/System/Library/Fonts/Supplemental/Arial.ttf"),
            PathBuf::from("/System/Library/Fonts/SFNS.ttf"),
            PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
            PathBuf::from("/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf"),
            PathBuf::from("/usr/share/fonts/TTF/DejaVuSans.ttf"),
        ];
        if let Some(root) = std::env::var_os("WINDIR") {
            paths.push(PathBuf::from(root).join("Fonts/segoeui.ttf"));
        }
        if let Some(path) = std::env::var_os("OPENXPLANE_UI_FONT") {
            paths.insert(0, PathBuf::from(path));
        }
        let font = paths
            .into_iter()
            .find_map(|p| {
                std::fs::read(p)
                    .ok()
                    .and_then(|v| FontArc::try_from_vec(v).ok())
            })
            .ok_or("No UI font found; set OPENXPLANE_UI_FONT to a .ttf font")?;
        Ok(Self {
            page: Page::Flight,
            setup: None,
            bar: false,
            hud: false,
            dropdown: None,
            keyboard_focus: false,
            font,
            hits: vec![],
            hover: None,
        })
    }
    pub fn modal(&self) -> bool {
        self.page != Page::Flight
    }
    pub fn captures_keyboard(&self) -> bool {
        self.modal() || self.dropdown.is_some() || self.keyboard_focus
    }
    pub fn escape(&mut self) {
        self.page = match self.page {
            Page::Flight => Page::Main,
            Page::Main => Page::Flight,
            _ => Page::Main,
        };
        self.dropdown = None;
        self.keyboard_focus = false;
    }
    pub fn alt(&mut self) {
        self.bar = !self.bar;
        self.keyboard_focus = self.bar;
        self.dropdown = None;
    }
    pub fn pointer(&mut self, x: f32, y: f32) -> bool {
        let next = self.hits.iter().rposition(|(r, _, _)| r.contains(x, y));
        let changed = next != self.hover;
        self.hover = next;
        changed
    }
    pub fn captures_pointer(&self, x: f32, y: f32) -> bool {
        self.captures_keyboard() || self.hits.iter().any(|(r, _, _)| r.contains(x, y))
    }
    pub fn click(&mut self, x: f32, y: f32) -> Option<Action> {
        let hit = self
            .hits
            .iter()
            .rev()
            .find(|(r, _, _)| r.contains(x, y))
            .copied();
        if let Some((_, click, true)) = hit {
            match click {
                Click::Page(page) => {
                    self.page = page;
                    self.dropdown = None;
                }
                Click::Menu(index) => {
                    self.dropdown = if self.dropdown == Some(index) {
                        None
                    } else {
                        Some(index)
                    }
                }
                Click::NextAircraft => {
                    if let Some(setup) = &mut self.setup {
                        setup.next_aircraft();
                    }
                }
                Click::AirportInput => {
                    if let Some(setup) = &mut self.setup {
                        setup.editing_airport = true;
                    }
                }
                Click::Hud => self.hud = !self.hud,
                Click::Action(action) => {
                    if matches!(action, Action::Resume | Action::Restart) {
                        self.page = Page::Flight;
                    }
                    self.dropdown = None;
                    self.keyboard_focus = false;
                    return Some(action);
                }
            }
        } else if !self.modal() {
            self.dropdown = None;
            self.keyboard_focus = false;
        }
        None
    }
    pub fn key(&mut self, name: &str) -> Option<Action> {
        if self.page == Page::Configuration
            && let Some(setup) = &mut self.setup
            && setup.editing_airport
        {
            setup.key(name);
            return None;
        }
        let choices: Vec<usize> = self
            .hits
            .iter()
            .enumerate()
            .filter(|(_, (_, _, enabled))| *enabled)
            .map(|(i, _)| i)
            .collect();
        if choices.is_empty() {
            return None;
        }
        if matches!(name, "Tab" | "Down" | "Up") {
            let current = choices.iter().position(|i| Some(*i) == self.hover);
            let next = if name == "Up" {
                current.map_or(choices.len() - 1, |i| {
                    (i + choices.len() - 1) % choices.len()
                })
            } else {
                current.map_or(0, |i| (i + 1) % choices.len())
            };
            self.hover = Some(choices[next]);
        } else if matches!(name, "Return" | "Enter" | "Space" | " ")
            && let Some(i) = self.hover
        {
            let r = self.hits.get(i)?.0;
            return self.click(r.x + r.w * 0.5, r.y + r.h * 0.5);
        }
        None
    }
    fn button(
        &mut self,
        c: &mut Canvas,
        rect: Rect,
        text: &str,
        click: Click,
        enabled: bool,
        primary: bool,
    ) {
        let hit = Rect {
            x: rect.x * c.scale,
            y: rect.y * c.scale,
            w: rect.w * c.scale,
            h: rect.h * c.scale,
        };
        let index = self.hits.len();
        let color = if !enabled {
            [28, 35, 45, 255]
        } else if primary {
            [29, 113, 190, 255]
        } else if self.hover == Some(index) {
            [66, 80, 99, 255]
        } else {
            [44, 55, 70, 255]
        };
        c.rect(rect, color);
        c.text(
            rect.x + 14.0,
            rect.y + (rect.h - 19.0) * 0.5,
            text,
            17.0,
            if enabled {
                [238, 243, 249, 255]
            } else {
                [115, 127, 141, 255]
            },
        );
        self.hits.push((hit, click, enabled));
    }
    pub fn paint(&mut self, width: u32, height: u32, dpi: f32, info: &Info) -> Vec<u8> {
        let font = self.font.clone();
        let scale = dpi * ((height as f32 / dpi) / 610.0).clamp(0.35, 1.0);
        let mut c = Canvas::new(width, height, scale, &font);
        let (w, h) = (width as f32 / scale, height as f32 / scale);
        self.hits.clear();
        if self.hud && info.flying && !self.modal() {
            c.rect(
                Rect {
                    x: w - 260.0,
                    y: 55.0,
                    w: 240.0,
                    h: 140.0,
                },
                [15, 22, 32, 225],
            );
            for (i, line) in info.telemetry.iter().take(5).enumerate() {
                c.text(
                    w - 244.0,
                    70.0 + i as f32 * 23.0,
                    line,
                    16.0,
                    [232, 242, 253, 255],
                );
            }
        }
        if self.bar || self.modal() {
            c.rect(
                Rect {
                    x: 0.0,
                    y: 0.0,
                    w,
                    h: 40.0,
                },
                [21, 28, 38, 255],
            );
            for (i, label) in ["File", "Flight", "View", "Developer", "Plugins"]
                .iter()
                .enumerate()
            {
                self.button(
                    &mut c,
                    Rect {
                        x: 12.0 + i as f32 * 116.0,
                        y: 0.0,
                        w: 112.0,
                        h: 40.0,
                    },
                    label,
                    Click::Menu(i),
                    true,
                    false,
                );
            }
            c.text(w - 155.0, 12.0, "openXplane", 16.0, [157, 178, 200, 255]);
        }
        if self.modal() {
            c.rect(
                Rect {
                    x: 0.0,
                    y: 40.0,
                    w,
                    h: h - 40.0,
                },
                [6, 12, 20, 175],
            );
            let pw = (w - 32.0).min(460.0);
            let x = (w - pw) * 0.5;
            let y = ((h - 500.0) * 0.5).max(54.0);
            c.rect(
                Rect {
                    x,
                    y,
                    w: pw,
                    h: 490.0,
                },
                [23, 32, 45, 255],
            );
            c.rect(
                Rect {
                    x,
                    y,
                    w: pw,
                    h: 4.0,
                },
                [52, 146, 221, 255],
            );
            let title = match self.page {
                Page::Main => "Main Menu",
                Page::Configuration => "Flight Configuration",
                Page::Settings => "Settings",
                Page::Keyboard => "Keyboard",
                Page::About => "About openXplane",
                Page::Flight => "",
            };
            c.text(x + 28.0, y + 27.0, title, 28.0, [243, 247, 252, 255]);
            let row =
                |ui: &mut Self, c: &mut Canvas, n: usize, label: &str, click, enabled, primary| {
                    ui.button(
                        c,
                        Rect {
                            x: x + 28.0,
                            y: y + 102.0 + n as f32 * 52.0,
                            w: pw - 56.0,
                            h: 42.0,
                        },
                        label,
                        click,
                        enabled,
                        primary,
                    );
                };
            match self.page {
                Page::Main => {
                    c.text(
                        x + 28.0,
                        y + 68.0,
                        "Your flight is held while this menu is open.",
                        15.0,
                        [149, 169, 190, 255],
                    );
                    row(
                        self,
                        &mut c,
                        0,
                        "Resume Flight",
                        Click::Action(Action::Resume),
                        true,
                        true,
                    );
                    row(
                        self,
                        &mut c,
                        1,
                        "New Flight",
                        Click::Page(Page::Configuration),
                        info.flying,
                        false,
                    );
                    row(
                        self,
                        &mut c,
                        2,
                        "Settings",
                        Click::Page(Page::Settings),
                        true,
                        false,
                    );
                    row(
                        self,
                        &mut c,
                        3,
                        "Keyboard",
                        Click::Page(Page::Keyboard),
                        true,
                        false,
                    );
                    row(
                        self,
                        &mut c,
                        4,
                        "About openXplane",
                        Click::Page(Page::About),
                        true,
                        false,
                    );
                    row(
                        self,
                        &mut c,
                        5,
                        "Quit",
                        Click::Action(Action::Quit),
                        true,
                        false,
                    );
                }
                Page::Configuration => {
                    if let Some(setup) = &self.setup {
                        let label = setup.aircraft[setup.selected]
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let airport = format!(
                            "Airport: {}{}",
                            setup.airport,
                            if setup.editing_airport { "_" } else { "" }
                        );
                        let valid = setup.valid();
                        let error = setup.error.clone();
                        c.text(
                            x + 28.0,
                            y + 72.0,
                            "Select aircraft; enter an airport identifier",
                            15.0,
                            [168, 190, 213, 255],
                        );
                        row(self, &mut c, 0, &label, Click::NextAircraft, true, false);
                        row(self, &mut c, 1, &airport, Click::AirportInput, true, false);
                        c.text(
                            x + 28.0,
                            y + 214.0,
                            "Click aircraft to cycle. Backspace edits airport.",
                            14.0,
                            [155, 174, 195, 255],
                        );
                        if let Some(error) = error {
                            c.text(
                                x + 28.0,
                                y + 248.0,
                                &error.chars().take(50).collect::<String>(),
                                14.0,
                                [255, 158, 145, 255],
                            );
                        }
                        row(
                            self,
                            &mut c,
                            3,
                            "Start New Flight",
                            Click::Action(Action::StartFlight),
                            valid,
                            true,
                        );
                    } else {
                        c.text(
                            x + 28.0,
                            y + 112.0,
                            "Restart the loaded flight at its runway.",
                            17.0,
                            [233, 240, 247, 255],
                        );
                        row(
                            self,
                            &mut c,
                            3,
                            "Start New Flight",
                            Click::Action(Action::Restart),
                            info.flying,
                            true,
                        );
                    }
                    row(
                        self,
                        &mut c,
                        5,
                        "Back",
                        Click::Page(Page::Main),
                        true,
                        false,
                    );
                }
                Page::Settings => {
                    row(
                        self,
                        &mut c,
                        0,
                        if info.keyboard_stick {
                            "Keyboard stick: ON"
                        } else {
                            "Keyboard stick: OFF"
                        },
                        Click::Action(Action::KeyboardStick),
                        info.flying,
                        false,
                    );
                    row(
                        self,
                        &mut c,
                        1,
                        if info.paused {
                            "Pause flight: ON"
                        } else {
                            "Pause flight: OFF"
                        },
                        Click::Action(Action::Pause),
                        info.flying,
                        false,
                    );
                    row(
                        self,
                        &mut c,
                        2,
                        if self.hud {
                            "Flight telemetry: ON"
                        } else {
                            "Flight telemetry: OFF"
                        },
                        Click::Hud,
                        info.flying,
                        false,
                    );
                    row(
                        self,
                        &mut c,
                        3,
                        "Keyboard bindings",
                        Click::Page(Page::Keyboard),
                        true,
                        false,
                    );
                    row(
                        self,
                        &mut c,
                        5,
                        "Back",
                        Click::Page(Page::Main),
                        true,
                        false,
                    );
                }
                Page::Keyboard => {
                    for (i, line) in [
                        "F1 / F2 / F3   Throttle down / up / full",
                        "1 / 2   Flaps up / down",
                        "B   Hold brakes       V   Maximum brakes",
                        "[ / ]   Pitch trim",
                        "Arrows, Z / X   Keyboard stick",
                        "Tab   Switch keyboard stick mode",
                        "P   Pause       Delete   Restart",
                        "Alt   Menu bar       Esc   Main menu",
                    ]
                    .iter()
                    .enumerate()
                    {
                        c.text(
                            x + 28.0,
                            y + 88.0 + i as f32 * 31.0,
                            line,
                            16.0,
                            [217, 230, 244, 255],
                        );
                    }
                    row(
                        self,
                        &mut c,
                        6,
                        "Back",
                        Click::Page(Page::Main),
                        true,
                        false,
                    );
                }
                Page::About => {
                    for (i, line) in [
                        "An independent open flight simulator.",
                        "Original aircraft content is loaded separately.",
                        "Flight dynamics are still approximate.",
                        "Menu layout is an openXplane implementation.",
                        "Original menus are not fully reproduced yet.",
                    ]
                    .iter()
                    .enumerate()
                    {
                        c.text(
                            x + 28.0,
                            y + 94.0 + i as f32 * 33.0,
                            line,
                            16.0,
                            [199, 218, 238, 255],
                        );
                    }
                    row(
                        self,
                        &mut c,
                        5,
                        "Back",
                        Click::Page(Page::Main),
                        true,
                        false,
                    );
                }
                Page::Flight => {}
            }
            c.text(
                x + 28.0,
                y + 460.0,
                "ESC to return    |    openXplane",
                13.0,
                [112, 140, 169, 255],
            );
        }
        if let Some(menu) = self.dropdown {
            let labels: Vec<(&str, Click, bool)> = match menu {
                0 => vec![
                    ("Main Menu", Click::Page(Page::Main), true),
                    (
                        "Flight Configuration",
                        Click::Page(Page::Configuration),
                        info.flying,
                    ),
                    ("Settings", Click::Page(Page::Settings), true),
                    ("Quit", Click::Action(Action::Quit), true),
                ],
                1 => vec![
                    ("Resume Flight", Click::Action(Action::Resume), true),
                    ("Pause / Resume", Click::Action(Action::Pause), info.flying),
                    (
                        "Restart Flight",
                        Click::Action(Action::Restart),
                        info.flying,
                    ),
                ],
                2 => vec![
                    (
                        "Default View",
                        Click::Action(Action::ChaseCamera),
                        info.flying,
                    ),
                    (
                        "Free Camera",
                        Click::Action(Action::FreeCamera),
                        info.flying,
                    ),
                    ("Flight Telemetry", Click::Hud, info.flying),
                ],
                3 => vec![("About openXplane", Click::Page(Page::About), true)],
                _ => vec![("No plugins loaded", Click::Page(Page::About), false)],
            };
            for (i, (label, click, enabled)) in labels.iter().enumerate() {
                self.button(
                    &mut c,
                    Rect {
                        x: 12.0 + menu as f32 * 116.0,
                        y: 42.0 + i as f32 * 39.0,
                        w: 250.0,
                        h: 38.0,
                    },
                    label,
                    *click,
                    *enabled,
                    false,
                );
            }
        }
        c.pixels
    }
}

pub struct Overlay {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    texture: wgpu::Texture,
    group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    size: (u32, u32),
}
impl Overlay {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("menu texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("menu overlay"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("menu overlay"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("menu overlay"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let texture = Self::texture(device, 1, 1);
        let group = Self::group(device, &layout, &texture, &sampler);
        Self {
            pipeline,
            layout,
            texture,
            group,
            sampler,
            size: (1, 1),
        }
    }
    fn texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("menu pixels"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }
    fn group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        texture: &wgpu::Texture,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("menu texture"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }
    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) {
        if self.size != (width, height) {
            self.texture = Self::texture(device, width, height);
            self.group = Self::group(device, &self.layout, &self.texture, &self.sampler);
            self.size = (width, height);
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
    pub fn draw(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("menus"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.draw(0..3, 0..1);
    }
}

struct Canvas<'a> {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    scale: f32,
    font: &'a FontArc,
}
impl<'a> Canvas<'a> {
    fn new(width: u32, height: u32, scale: f32, font: &'a FontArc) -> Self {
        Self {
            pixels: vec![0; width as usize * height as usize * 4],
            width,
            height,
            scale,
            font,
        }
    }
    fn rect(&mut self, r: Rect, color: [u8; 4]) {
        let (x0, y0, x1, y1) = (
            (r.x * self.scale).max(0.0) as u32,
            (r.y * self.scale).max(0.0) as u32,
            ((r.x + r.w) * self.scale).min(self.width as f32).max(0.0) as u32,
            ((r.y + r.h) * self.scale).min(self.height as f32).max(0.0) as u32,
        );
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * self.width + x) * 4) as usize;
                self.pixels[i..i + 4].copy_from_slice(&color);
            }
        }
    }
    fn text(&mut self, x: f32, y: f32, text: &str, size: f32, color: [u8; 4]) {
        let scaled = self.font.as_scaled(PxScale::from(size * self.scale));
        let mut caret = point(x * self.scale, y * self.scale + scaled.ascent());
        let mut previous = None;
        for ch in text.chars() {
            let id = self.font.glyph_id(ch);
            if let Some(prev) = previous {
                caret.x += scaled.kern(prev, id);
            }
            let glyph = id.with_scale_and_position(size * self.scale, caret);
            if let Some(outline) = self.font.outline_glyph(glyph) {
                let bounds = outline.px_bounds();
                let width = self.width;
                let height = self.height;
                let pixels = &mut self.pixels;
                outline.draw(|gx, gy, coverage| {
                    let px = bounds.min.x as i32 + gx as i32;
                    let py = bounds.min.y as i32 + gy as i32;
                    if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                        return;
                    }
                    let i = ((py as u32 * width + px as u32) * 4) as usize;
                    let alpha = coverage * color[3] as f32 / 255.0;
                    let old = pixels[i + 3] as f32 / 255.0;
                    let a = alpha + old * (1.0 - alpha);
                    if a > 0.0 {
                        for k in 0..3 {
                            pixels[i + k] = ((color[k] as f32 * alpha
                                + pixels[i + k] as f32 * old * (1.0 - alpha))
                                / a)
                                .round() as u8;
                        }
                        pixels[i + 3] = (a * 255.0).round() as u8;
                    }
                });
            }
            caret.x += scaled.h_advance(id);
            previous = Some(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn info(flying: bool) -> Info {
        Info {
            flying,
            paused: false,
            keyboard_stick: true,
            telemetry: vec![],
        }
    }
    fn activate(ui: &mut Menus, click: Click) -> Option<Action> {
        let (r, _, _) = *ui.hits.iter().rev().find(|(_, c, _)| *c == click).unwrap();
        ui.click(r.x + r.w / 2.0, r.y + r.h / 2.0)
    }
    #[test]
    fn escape_and_alt_capture_only_the_relevant_input() {
        let mut ui = Menus::new().unwrap();
        ui.alt();
        assert!(ui.bar && !ui.modal() && ui.captures_keyboard());
        assert!(ui.captures_pointer(1090.0, 710.0));
        ui.click(1090.0, 710.0);
        assert!(!ui.captures_keyboard());
        ui.escape();
        assert!(ui.modal() && ui.captures_keyboard());
        ui.page = Page::Settings;
        ui.escape();
        assert_eq!(ui.page, Page::Main);
        ui.escape();
        assert_eq!(ui.page, Page::Flight);
    }
    #[test]
    fn modal_capture_resume_and_restart_work_at_retina_scale() {
        let mut ui = Menus::new().unwrap();
        ui.escape();
        ui.paint(2200, 1440, 2.0, &info(true));
        assert!(ui.captures_pointer(2190.0, 1400.0));
        assert_eq!(
            activate(&mut ui, Click::Action(Action::Resume)),
            Some(Action::Resume)
        );
        assert!(!ui.modal());
        ui.page = Page::Configuration;
        ui.paint(1100, 720, 1.0, &info(true));
        assert_eq!(
            activate(&mut ui, Click::Action(Action::Restart)),
            Some(Action::Restart)
        );
        assert_eq!(ui.page, Page::Flight);
    }
    #[test]
    fn disabled_actions_do_not_run_without_a_flight() {
        let mut ui = Menus::new().unwrap();
        ui.page = Page::Configuration;
        ui.paint(1100, 720, 1.0, &info(false));
        assert_eq!(activate(&mut ui, Click::Action(Action::Restart)), None);
        assert_eq!(ui.page, Page::Configuration);
    }
    #[test]
    fn dropdown_captures_input_and_outside_click_dismisses_it() {
        let mut ui = Menus::new().unwrap();
        ui.alt();
        ui.paint(1100, 720, 1.0, &info(true));
        activate(&mut ui, Click::Menu(1));
        ui.paint(1100, 720, 1.0, &info(true));
        assert!(ui.captures_keyboard());
        assert_eq!(
            activate(&mut ui, Click::Action(Action::Pause)),
            Some(Action::Pause)
        );
        assert!(!ui.captures_keyboard());
        activate(&mut ui, Click::Menu(1));
        ui.click(1090.0, 710.0);
        assert!(!ui.captures_keyboard());
    }
    #[test]
    fn keyboard_navigation_skips_disabled_items_and_survives_page_changes() {
        let mut ui = Menus::new().unwrap();
        ui.page = Page::Settings;
        ui.paint(1100, 720, 1.0, &info(false));
        ui.key("Up");
        assert_eq!(ui.key("Return"), None);
        assert_eq!(ui.page, Page::Main);
        ui.paint(1100, 720, 1.0, &info(true));
        ui.hover = Some(usize::MAX);
        assert_eq!(ui.key("Return"), None);
    }
    #[test]
    fn configuration_edits_airport_and_dispatches_selection_without_closing_early() {
        let mut ui = Menus::new().unwrap();
        ui.page = Page::Configuration;
        ui.setup = Some(crate::flight_setup::FlightSetup {
            root: Default::default(),
            aircraft: vec!["a.acf".into(), "b.acf".into()],
            selected: 0,
            airport: "KSEA".into(),
            editing_airport: false,
            error: None,
        });
        ui.paint(1100, 720, 1.0, &info(true));
        activate(&mut ui, Click::NextAircraft);
        assert_eq!(ui.setup.as_ref().unwrap().selected, 1);
        activate(&mut ui, Click::AirportInput);
        for _ in 0..4 {
            ui.key("Backspace");
        }
        ui.paint(1100, 720, 1.0, &info(true));
        assert_eq!(activate(&mut ui, Click::Action(Action::StartFlight)), None);
        for key in ["K", "O", "A", "K", "Return"] {
            ui.key(key);
        }
        assert_eq!(ui.setup.as_ref().unwrap().airport, "KOAK");
        ui.paint(1100, 720, 1.0, &info(true));
        assert_eq!(
            activate(&mut ui, Click::Action(Action::StartFlight)),
            Some(Action::StartFlight)
        );
        assert_eq!(ui.page, Page::Configuration);
    }
    #[test]
    fn closed_ui_is_transparent_and_resized_pages_fit_the_canvas() {
        let mut ui = Menus::new().unwrap();
        assert!(ui.paint(640, 480, 1.0, &info(true)).iter().all(|v| *v == 0));
        ui.escape();
        let pixels = ui.paint(640, 480, 1.0, &info(true));
        assert_eq!(pixels.len(), 640 * 480 * 4);
        assert!(
            ui.hits
                .iter()
                .all(|(r, _, _)| r.y >= 0.0 && r.y + r.h <= 480.0)
        );
    }
}

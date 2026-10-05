use crate::{
    gpu::{Camera, Renderer},
    scene::Scene,
};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

struct State {
    instance: wgpu::Instance,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    targets: crate::gpu::Targets,
    camera: Camera,
}

impl State {
    async fn new(window: Arc<Window>, scene: &Scene) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = crate::gpu::instance();
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("No surface configuration")?;
        let caps = surface.get_capabilities(&adapter);
        if let Some(format) = caps.formats.iter().copied().find(|f| f.is_srgb()) {
            config.format = format;
        }
        config.present_mode = wgpu::PresentMode::Fifo;
        let renderer = Renderer::new(&adapter, config.format, scene).await?;
        surface.configure(&renderer.device, &config);
        let targets = renderer.targets(config.width, config.height);
        Ok(Self {
            instance,
            window,
            surface,
            config,
            renderer,
            targets,
            camera: Camera::new(scene),
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.renderer.device, &self.config);
        self.targets = self.renderer.targets(width, height);
    }

    fn draw(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if self.window.inner_size().width == 0 || self.window.inner_size().height == 0 {
            return Ok(false);
        }
        let (frame, reconfigure) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.resize(self.config.width, self.config.height);
                self.window.request_redraw();
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(self.window.clone())?;
                self.resize(self.config.width, self.config.height);
                self.window.request_redraw();
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("Surface validation failed".into());
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self
            .renderer
            .device
            .create_command_encoder(&Default::default());
        self.renderer
            .draw(&mut encoder, &view, &self.targets, &self.camera);
        self.renderer.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        frame.present();
        if reconfigure {
            self.resize(self.config.width, self.config.height);
        }
        Ok(true)
    }
}

struct App {
    scene: Scene,
    state: Option<State>,
    dragging: bool,
    cursor: Option<(f64, f64)>,
    error: Option<String>,
    smoke: bool,
    frames: u32,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let result = (|| -> Result<State, Box<dyn std::error::Error>> {
            let window = Arc::new(event_loop.create_window(Window::default_attributes().with_title("openXplane · Static Cessna preview · Drag to orbit / Scroll to zoom / R to reset").with_inner_size(LogicalSize::new(1100.0,720.0)))?);
            pollster::block_on(State::new(window, &self.scene))
        })();
        match result {
            Ok(state) => {
                state.window.request_redraw();
                self.state = Some(state);
            }
            Err(e) => {
                self.error = Some(e.to_string());
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = &mut self.state else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.resize(size.width, size.height);
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => match state.draw() {
                Ok(drawn) => {
                    if drawn {
                        self.frames += 1;
                    }
                    if self.smoke {
                        if self.frames >= 3 {
                            println!("Native viewer smoke test: {} frames presented", self.frames);
                            event_loop.exit();
                        } else {
                            state.window.request_redraw();
                        }
                    }
                }
                Err(e) => {
                    self.error = Some(e.to_string());
                    event_loop.exit();
                }
            },
            WindowEvent::MouseInput {
                state: button_state,
                button: MouseButton::Left,
                ..
            } => self.dragging = button_state == ElementState::Pressed,
            WindowEvent::Focused(false) | WindowEvent::CursorLeft { .. } => {
                self.dragging = false;
                self.cursor = None;
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some((x, y)) = self.cursor
                    && self.dragging
                {
                    state.camera.yaw -= (position.x - x) as f32 * 0.006;
                    state.camera.pitch =
                        (state.camera.pitch + (position.y - y) as f32 * 0.006).clamp(-1.4, 1.4);
                    state.window.request_redraw();
                }
                self.cursor = Some((position.x, position.y));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 * 0.02,
                };
                state.camera.distance = (state.camera.distance * (-amount * 0.12).exp())
                    .clamp(state.camera.radius * 0.3, state.camera.radius * 15.0);
                state.window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Character(key) if key.eq_ignore_ascii_case("r") => {
                        state.camera = Camera::new(&self.scene);
                        state.window.request_redraw();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

pub fn run(scene: Scene, smoke: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Drag left mouse: orbit. Scroll: zoom. R: reset. Esc: close.");
    let event_loop = EventLoop::new()?;
    let mut app = App {
        scene,
        state: None,
        dragging: false,
        cursor: None,
        error: None,
        smoke,
        frames: 0,
    };
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error {
        return Err(error.into());
    }
    Ok(())
}

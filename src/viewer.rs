use crate::{
    gpu::{Camera, Renderer},
    scene::{AircraftBody, FlightSession, Scene},
};
use glam::Vec3;
use openxplane::flight::{Controls, FlightModel};
use std::{sync::Arc, time::Instant};
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

/// Keys held for flying.
#[derive(Default)]
struct Keys {
    pitch_back: bool,
    pitch_forward: bool,
    roll_left: bool,
    roll_right: bool,
    yaw_left: bool,
    yaw_right: bool,
    throttle_up: bool,
    throttle_down: bool,
    brake: bool,
}

struct Flight {
    body: AircraftBody,
    model: FlightModel,
    keys: Keys,
    controls: Controls,
    last: Instant,
    accumulated: f32,
    camera_yaw: f32,
    paused: bool,
}

const STEP: f32 = 1.0 / 200.0;

fn approach(value: f32, target: f32, rate: f32, dt: f32) -> f32 {
    let step = rate * dt;
    if (target - value).abs() <= step {
        target
    } else {
        value + step * (target - value).signum()
    }
}

impl Flight {
    fn axis(positive: bool, negative: bool) -> f32 {
        f32::from(positive) - f32::from(negative)
    }

    fn update_controls(&mut self, dt: f32) {
        let k = &self.keys;
        let c = &mut self.controls;
        for (value, target) in [
            (&mut c.elevator, Self::axis(k.pitch_back, k.pitch_forward)),
            (&mut c.aileron, Self::axis(k.roll_right, k.roll_left)),
            (&mut c.rudder, Self::axis(k.yaw_right, k.yaw_left)),
        ] {
            let rate = if target == 0.0 { 4.0 } else { 2.5 };
            *value = approach(*value, target, rate, dt);
        }
        c.throttle =
            (c.throttle + Self::axis(k.throttle_up, k.throttle_down) * 0.4 * dt).clamp(0.0, 1.0);
        c.brake = approach(c.brake, f32::from(k.brake), 6.0, dt);
    }

    fn set_key(&mut self, key: &Key, pressed: bool) {
        let k = &mut self.keys;
        match key {
            Key::Named(NamedKey::ArrowDown) => k.pitch_back = pressed,
            Key::Named(NamedKey::ArrowUp) => k.pitch_forward = pressed,
            Key::Named(NamedKey::ArrowLeft) => k.roll_left = pressed,
            Key::Named(NamedKey::ArrowRight) => k.roll_right = pressed,
            Key::Named(NamedKey::PageUp) => k.throttle_up = pressed,
            Key::Named(NamedKey::PageDown) => k.throttle_down = pressed,
            Key::Named(NamedKey::Space) => k.brake = pressed,
            Key::Character(c) => match c.to_ascii_lowercase().as_str() {
                "z" => k.yaw_left = pressed,
                "x" => k.yaw_right = pressed,
                "w" => k.throttle_up = pressed,
                "s" => k.throttle_down = pressed,
                "b" => k.brake = pressed,
                _ => {}
            },
            _ => {}
        }
    }

    /// Steps the simulation by real elapsed time (at most 0.1 s per frame) and poses the aircraft meshes.
    fn advance(&mut self, scene: &mut Scene, camera: &mut Camera) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.1);
        self.last = now;
        self.update_controls(dt);
        if !self.paused {
            self.accumulated += dt;
            while self.accumulated >= STEP {
                self.model.step(STEP, &self.controls);
                self.accumulated -= STEP;
            }
        }
        self.body.apply(scene, &self.model);
        // chase camera: behind the aircraft's heading, orbiting with the mouse
        let forward = self.model.state.orientation * Vec3::NEG_Z;
        let heading = (-forward.x).atan2(-forward.z);
        camera.target = self.model.state.position;
        camera.yaw = std::f32::consts::PI - heading + self.camera_yaw;
    }

    fn title(&self) -> String {
        let t = self.model.telemetry(&self.controls);
        format!(
            "openXplane flight · IAS {:.0} kt · ALT {:.0} ft · VS {:+.0} fpm · PITCH {:+.0}° · BANK {:+.0}° · HDG {:.0}° · THR {:.0}%{}{}{}",
            t.airspeed_kt,
            t.altitude_ft,
            t.vertical_speed_fpm,
            t.pitch_deg,
            t.roll_deg,
            t.heading_deg,
            self.controls.throttle * 100.0,
            if t.on_ground { " · ON GROUND" } else { "" },
            if t.stalled_elements > 0 {
                " · STALL"
            } else {
                ""
            },
            if self.paused { " · PAUSED" } else { "" },
        )
    }
}

struct App {
    flight: Option<Flight>,
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
            WindowEvent::RedrawRequested => {
                if let Some(flight) = &mut self.flight {
                    flight.advance(&mut self.scene, &mut state.camera);
                    state
                        .renderer
                        .update_meshes(flight.body.first_mesh, &self.scene);
                    state.window.set_title(&flight.title());
                }
                match state.draw() {
                    Ok(drawn) => {
                        if drawn {
                            self.frames += 1;
                        }
                        if self.smoke {
                            if self.frames >= 3 {
                                println!(
                                    "Native viewer smoke test: {} frames presented",
                                    self.frames
                                );
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
                }
                if self.flight.is_some() {
                    state.window.request_redraw();
                }
            }
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
                    let dx = (position.x - x) as f32 * 0.006;
                    match &mut self.flight {
                        Some(flight) => flight.camera_yaw -= dx,
                        None => state.camera.yaw -= dx,
                    }
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
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                if let Some(flight) = &mut self.flight {
                    flight.set_key(&event.logical_key, pressed);
                }
                if pressed {
                    match event.logical_key {
                        Key::Named(NamedKey::Escape) => event_loop.exit(),
                        Key::Character(key) if key.eq_ignore_ascii_case("r") => {
                            match &mut self.flight {
                                Some(flight) => {
                                    flight.model.reset();
                                    flight.controls = Controls::default();
                                    flight.camera_yaw = 0.0;
                                }
                                None => state.camera = Camera::new(&self.scene),
                            }
                            state.window.request_redraw();
                        }
                        Key::Character(key) if key.eq_ignore_ascii_case("p") => {
                            if let Some(flight) = &mut self.flight {
                                flight.paused = !flight.paused;
                            }
                        }
                        Key::Character(key) if key.eq_ignore_ascii_case("f") => {
                            if let Some(flight) = &mut self.flight {
                                flight.controls.flaps =
                                    (flight.controls.flaps + 1.0 / 3.0).min(1.0);
                            }
                        }
                        Key::Character(key) if key.eq_ignore_ascii_case("v") => {
                            if let Some(flight) = &mut self.flight {
                                flight.controls.flaps =
                                    (flight.controls.flaps - 1.0 / 3.0).max(0.0);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn run(scene: Scene, smoke: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Drag left mouse: orbit. Scroll: zoom. R: reset. Esc: close.");
    run_app(scene, None, smoke)
}

/// Flies the aircraft of an airport session with the approximate flight model.
pub fn run_flight(session: FlightSession, smoke: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Fly: Down/Up pitch, Left/Right roll, Z/X rudder, PageUp/PageDown or W/S throttle, Space/B brake, \
         F/V flaps, P pause, R reset, mouse to look around, Esc quit."
    );
    let FlightSession { scene, body, model } = session;
    let flight = Flight {
        body,
        model,
        keys: Keys::default(),
        controls: Controls::default(),
        last: Instant::now(),
        accumulated: 0.0,
        camera_yaw: 0.0,
        paused: false,
    };
    run_app(scene, Some(flight), smoke)
}

fn run_app(
    scene: Scene,
    flight: Option<Flight>,
    smoke: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = App {
        flight,
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

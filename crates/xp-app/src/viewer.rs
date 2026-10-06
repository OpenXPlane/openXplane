use crate::{
    gpu::{Camera, Renderer},
    hud,
    scene::{AircraftBody, FlightSession, Scene},
};
use glam::Vec3;
use openxplane::{
    commands::Phase,
    flight::{Controls, FlightModel},
    pilot::{Effect, ViewAction},
};
use std::{sync::Arc, time::Instant};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, KeyLocation, NamedKey},
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

/// Stick keys held (openXplane's keyboard stick; the original has no default keyboard stick).
#[derive(Default)]
struct Keys {
    pitch_back: bool,
    pitch_forward: bool,
    roll_left: bool,
    roll_right: bool,
    yaw_left: bool,
    yaw_right: bool,
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
    /// Arrow keys and Z/X act as the stick (default) instead of their original meaning
    /// (view movement and smoke toggle). Toggled with Tab.
    stick_keys: bool,
    free_camera: bool,
    view_offset: Vec3,
    message: Option<(String, Instant)>,
    help: bool,
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

/// The key name used by the keymap table for a keyboard event.
fn key_name(event: &KeyEvent) -> Option<String> {
    match &event.logical_key {
        Key::Named(named) => {
            let name = format!("{named:?}");
            Some(match name.as_str() {
                "ArrowLeft" => "Left".to_string(),
                "ArrowRight" => "Right".to_string(),
                "ArrowUp" => "Up".to_string(),
                "ArrowDown" => "Down".to_string(),
                "Enter" => "Return".to_string(),
                _ => name,
            })
        }
        Key::Character(c) => {
            let ch = c.chars().next()?;
            if event.location == KeyLocation::Numpad && ch.is_ascii_digit() {
                Some(format!("Numpad{ch}"))
            } else {
                Some(ch.to_ascii_uppercase().to_string())
            }
        }
        _ => None,
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
    }

    fn say(&mut self, text: impl Into<String>) {
        self.message = Some((text.into(), Instant::now()));
    }

    /// Handles a key. Returns a view action for the caller to apply to the camera.
    fn handle_key(&mut self, name: &str, phase: Phase) -> Option<ViewAction> {
        let pressed = phase != Phase::End;
        // openXplane's own keys (not in the original's default map)
        match name {
            "Tab" if phase == Phase::Begin => {
                self.stick_keys = !self.stick_keys;
                self.keys = Keys::default();
                self.say(if self.stick_keys {
                    "arrows + Z/X fly the aircraft"
                } else {
                    "arrows and X use their original meaning"
                });
                return None;
            }
            "H" if phase == Phase::Begin => {
                self.help = !self.help;
                return None;
            }
            "Delete" if phase == Phase::Begin => {
                self.model.reset();
                self.controls = Controls::default();
                self.camera_yaw = 0.0;
                self.view_offset = Vec3::ZERO;
                self.say("reset");
                return None;
            }
            _ => {}
        }
        if self.stick_keys {
            let k = &mut self.keys;
            let consumed = match name {
                "Down" => Some(&mut k.pitch_back),
                "Up" => Some(&mut k.pitch_forward),
                "Left" => Some(&mut k.roll_left),
                "Right" => Some(&mut k.roll_right),
                "Z" => Some(&mut k.yaw_left),
                "X" => Some(&mut k.yaw_right),
                _ => None,
            };
            if let Some(flag) = consumed {
                *flag = pressed;
                return None;
            }
        }
        let binding = openxplane::keymap::command_for_key(name)?;
        let short = binding.command.trim_start_matches("sim/");
        match openxplane::pilot::apply_command(&mut self.controls, &binding.command, phase) {
            Effect::Controls => {}
            Effect::Pause => {
                self.paused = !self.paused;
            }
            Effect::View(action) => return Some(action),
            Effect::NotSimulated if phase == Phase::Begin => {
                self.say(format!("{name}: {short} (not simulated)"));
            }
            Effect::NotSimulated | Effect::Unknown => {}
        }
        None
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
        camera.target = self.model.state.position + self.view_offset;
        if !self.free_camera {
            // chase camera: behind the aircraft's heading, orbiting with the mouse
            let forward = self.model.state.orientation * Vec3::NEG_Z;
            let heading = (-forward.x).atan2(-forward.z);
            camera.yaw = std::f32::consts::PI - heading + self.camera_yaw;
        }
    }

    fn title(&self) -> String {
        let t = self.model.telemetry(&self.controls);
        let note = match &self.message {
            Some((m, at)) if at.elapsed().as_secs_f32() < 3.0 => format!(" · {m}"),
            _ => String::new(),
        };
        format!(
            "openXplane flight · IAS {:.0} kt · ALT {:.0} ft · VS {:+.0} fpm · PITCH {:+.0}° · BANK {:+.0}° · HDG {:.0}° · THR {:.0}% · FLAPS {:.0}%{}{}{}{}",
            t.airspeed_kt,
            t.altitude_ft,
            t.vertical_speed_fpm,
            t.pitch_deg,
            t.roll_deg,
            t.heading_deg,
            self.controls.throttle * 100.0,
            self.controls.flaps * 100.0,
            if t.on_ground { " · ON GROUND" } else { "" },
            if t.stalled_elements > 0 {
                " · STALL"
            } else {
                ""
            },
            if self.paused { " · PAUSED" } else { "" },
            note,
        )
    }
}

struct App {
    flight: Option<Flight>,
    scene: Scene,
    state: Option<State>,
    dragging: bool,
    cursor: Option<(f64, f64)>,
    menu: crate::menu::Menu,
    error: Option<String>,
    smoke: bool,
    frames: u32,
}

fn apply_view(action: ViewAction, flight: &mut Flight, camera: &mut Camera) {
    match action {
        ViewAction::Default => {
            flight.camera_yaw = 0.0;
            flight.free_camera = false;
            flight.view_offset = Vec3::ZERO;
            camera.pitch = 0.3;
            camera.distance = camera.radius * 1.8;
        }
        ViewAction::ToggleFree => flight.free_camera = !flight.free_camera,
        ViewAction::Shift(dx, dy) => {
            let right = Vec3::new(-camera.yaw.cos(), 0.0, -camera.yaw.sin());
            flight.view_offset += right * dx + Vec3::Y * dy;
        }
        ViewAction::Rotate { yaw, pitch } => {
            if flight.free_camera {
                camera.yaw += yaw;
            } else {
                flight.camera_yaw += yaw;
            }
            camera.pitch = (camera.pitch + pitch).clamp(-1.4, 1.4);
        }
        ViewAction::Zoom(z) => {
            camera.distance = (camera.distance * (z * 0.05_f32).exp())
                .clamp(camera.radius * 0.3, camera.radius * 15.0);
        }
    }
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
                    let telemetry = flight.model.telemetry(&flight.controls);
                    let note = match &flight.message {
                        Some((m, at)) if at.elapsed().as_secs_f32() < 3.0 => Some(m.as_str()),
                        _ => None,
                    };
                    let mut interface = hud::Hud::new(state.config.width, state.config.height);
                    hud::draw_flight(
                        &mut interface,
                        &hud::FlightHud {
                            telemetry: &telemetry,
                            controls: &flight.controls,
                            paused: flight.paused,
                            help: flight.help,
                            note,
                        },
                    );
                    self.menu.draw(&mut interface, None);
                    state.renderer.set_hud(&interface.vertices);
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
            } => {
                let pressed = button_state == ElementState::Pressed;
                if pressed && let Some((x, y)) = self.cursor {
                    let height = state.config.height as f32;
                    match self.menu.click(x as f32, y as f32, height) {
                        crate::menu::Click::Outside => {}
                        crate::menu::Click::Consumed => {
                            state.window.request_redraw();
                            return;
                        }
                        crate::menu::Click::Run("Quit") => {
                            event_loop.exit();
                            return;
                        }
                        crate::menu::Click::Run(key) => {
                            if let Some(flight) = &mut self.flight
                                && let Some(action) = flight.handle_key(key, Phase::Begin)
                            {
                                apply_view(action, flight, &mut state.camera);
                            }
                            return;
                        }
                    }
                }
                self.dragging = pressed;
            }
            WindowEvent::Focused(false) | WindowEvent::CursorLeft { .. } => {
                self.dragging = false;
                self.cursor = None;
                self.menu.pointer_left();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let height = state.config.height as f32;
                self.menu
                    .pointer_moved(position.x as f32, position.y as f32, height);
                if self
                    .menu
                    .covers(position.x as f32, position.y as f32, height)
                {
                    self.dragging = false;
                }
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
                let phase = match (pressed, event.repeat) {
                    (false, _) => Phase::End,
                    (true, false) => Phase::Begin,
                    (true, true) => Phase::Continue,
                };
                if pressed && matches!(event.logical_key, Key::Named(NamedKey::Escape)) {
                    if self.menu.open.take().is_some() {
                        return;
                    }
                    event_loop.exit();
                    return;
                }
                let name = key_name(&event);
                match (&mut self.flight, name) {
                    (Some(flight), Some(name)) => {
                        if let Some(action) = flight.handle_key(&name, phase) {
                            apply_view(action, flight, &mut state.camera);
                        }
                    }
                    (None, Some(name)) if pressed && name == "R" => {
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
    run_app(scene, None, smoke)
}

/// Flies the aircraft of an airport session with the approximate flight model.
pub fn run_flight(session: FlightSession, smoke: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Fly with the original's default keys: F1/F2/F3 throttle down/up/full, 1/2 flaps up/down, B brakes (hold), \
         V brakes max, [ ] pitch trim, 5/6/7 and 8/9/0 rudder and aileron trim, P pause, W default view. \
         openXplane's keyboard stick: arrows pitch/roll, Z/X rudder (Tab switches them to their original \
         meaning). H shows the key help, Delete resets, Esc quits."
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
        stick_keys: true,
        free_camera: false,
        view_offset: Vec3::ZERO,
        message: None,
        help: false,
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
        menu: crate::menu::Menu::default(),
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

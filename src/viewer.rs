use crate::{
    gpu::{Camera, Renderer},
    scene::{AircraftBody, FlightSession, Scene},
};
use glam::Vec3;
use openxplane::{
    commands::Phase,
    flight::{Controls, FlightModel},
    pilot::{CommandInput, Effect, ViewAction},
};
use std::{collections::HashMap, sync::Arc, time::Instant};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, KeyLocation, NamedKey, PhysicalKey},
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
    menus: crate::ui::Menus,
    overlay: crate::ui::Overlay,
    ui_info: crate::ui::Info,
    ui_dirty: bool,
    ui_updated: Instant,
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
        let overlay = crate::ui::Overlay::new(&renderer.device, config.format);
        let mut menus = crate::ui::Menus::new()?;
        if let Ok(page) = std::env::var("OPENXPLANE_UI_SMOKE") {
            match page.as_str() {
                "esc" => menus.escape(),
                "alt" => menus.alt(),
                "settings" => menus.page = crate::ui::Page::Settings,
                _ => {}
            }
        }
        Ok(Self {
            instance,
            window,
            surface,
            config,
            renderer,
            targets,
            camera: Camera::new(scene),
            menus,
            overlay,
            ui_info: crate::ui::Info {
                flying: false,
                paused: false,
                keyboard_stick: false,
                telemetry: vec![],
            },
            ui_dirty: true,
            ui_updated: Instant::now(),
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
        self.ui_dirty = true;
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
        if self.ui_dirty || (self.menus.hud && self.ui_updated.elapsed().as_millis() >= 100) {
            let pixels = self.menus.paint(
                self.config.width,
                self.config.height,
                self.window.scale_factor() as f32,
                &self.ui_info,
            );
            self.overlay.upload(
                &self.renderer.device,
                &self.renderer.queue,
                self.config.width,
                self.config.height,
                &pixels,
            );
            self.ui_dirty = false;
            self.ui_updated = Instant::now();
        }
        self.overlay.draw(&mut encoder, &view);
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

impl Keys {
    fn release(&mut self, controls: &mut Controls) {
        *self = Self::default();
        controls.elevator = 0.0;
        controls.aileron = 0.0;
        controls.rudder = 0.0;
    }
}

struct Flight {
    body: AircraftBody,
    model: FlightModel,
    keys: Keys,
    controls: Controls,
    commands: CommandInput,
    held_keys: HashMap<PhysicalKey, String>,
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

/// Match releases and repeats to the binding chosen at press, even if modifiers/layout change.
fn route_key(
    held: &mut HashMap<PhysicalKey, String>,
    physical: PhysicalKey,
    phase: Phase,
    name: Option<String>,
) -> Option<String> {
    match phase {
        Phase::Begin => {
            let name = name?;
            Some(held.entry(physical).or_insert(name).clone())
        }
        Phase::Continue => held.get(&physical).cloned(),
        Phase::End => held.remove(&physical),
    }
}

fn routed_key(held: &mut HashMap<PhysicalKey, String>, event: &KeyEvent) -> Option<String> {
    let phase = match (event.state == ElementState::Pressed, event.repeat) {
        (false, _) => Phase::End,
        (true, false) => Phase::Begin,
        (true, true) => Phase::Continue,
    };
    route_key(held, event.physical_key, phase, key_name(event))
}

impl Flight {
    fn release_held(&mut self) {
        self.keys.release(&mut self.controls);
        self.held_keys.clear();
        self.commands.release_held(&mut self.controls);
    }

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
            "Delete" if phase == Phase::Begin => {
                self.model.reset();
                self.controls = Controls::default();
                self.commands = CommandInput::new(self.model.flap_detent_count());
                self.release_held();
                self.accumulated = 0.0;
                self.last = Instant::now();
                self.camera_yaw = 0.0;
                self.free_camera = false;
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
        match self
            .commands
            .apply(&mut self.controls, &binding.command, phase)
        {
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
    fn advance(&mut self, scene: &mut Scene, camera: &mut Camera, menu_open: bool) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.1);
        self.last = now;
        self.update_controls(dt);
        if !self.paused && !menu_open {
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
    setup: Option<crate::flight_setup::FlightSetup>,
    pending_flight: bool,
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
            let window = Arc::new(event_loop.create_window(Window::default_attributes().with_title("openXplane · Static Cessna preview · Drag to orbit / Scroll to zoom / R to reset").with_inner_size(LogicalSize::new(1100.0,720.0)).with_min_inner_size(LogicalSize::new(640.0,480.0)))?);
            pollster::block_on(State::new(window, &self.scene))
        })();
        match result {
            Ok(mut state) => {
                state.menus.setup = self.setup.take();
                if self.smoke
                    && let Ok(airport) = std::env::var("OPENXPLANE_UI_RELOAD_SMOKE")
                    && let Some(setup) = &mut state.menus.setup
                {
                    setup.next_aircraft();
                    setup.airport = airport;
                    state.menus.page = crate::ui::Page::Configuration;
                    self.pending_flight = true;
                }
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
        if matches!(event, WindowEvent::RedrawRequested) && self.pending_flight {
            self.pending_flight = false;
            self.start_selected_flight();
        }
        let Some(state) = &mut self.state else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.resize(size.width, size.height);
                state.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                state.ui_dirty = true;
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let Some(flight) = &mut self.flight {
                    flight.advance(&mut self.scene, &mut state.camera, state.menus.modal());
                    let t = flight.model.telemetry(&flight.controls);
                    state.ui_info = crate::ui::Info {
                        flying: true,
                        paused: flight.paused,
                        keyboard_stick: flight.stick_keys,
                        telemetry: vec![
                            format!("IAS  {:.0} kt", t.airspeed_kt),
                            format!("ALT  {:.0} ft", t.altitude_ft),
                            format!("VS   {:+.0} fpm", t.vertical_speed_fpm),
                            format!("HDG  {:.0} deg", t.heading_deg),
                            format!("THR  {:.0}%", flight.controls.throttle * 100.0),
                        ],
                    };
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
            } => {
                if let Some((x, y)) = self.cursor
                    && state.menus.captures_pointer(x as f32, y as f32)
                {
                    self.dragging = false;
                    if button_state == ElementState::Pressed {
                        if let Some(action) = state.menus.click(x as f32, y as f32) {
                            if action == crate::ui::Action::StartFlight {
                                self.pending_flight = true;
                            }
                            apply_menu_action(
                                action,
                                &mut self.flight,
                                &mut state.camera,
                                event_loop,
                            );
                        }
                        if let Some(flight) = &mut self.flight {
                            flight.release_held();
                        }
                        state.ui_dirty = true;
                        state.window.request_redraw();
                    }
                } else {
                    self.dragging = button_state == ElementState::Pressed;
                }
            }
            WindowEvent::Focused(false) => {
                self.dragging = false;
                self.cursor = None;
                if let Some(flight) = &mut self.flight {
                    flight.release_held();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.dragging = false;
                self.cursor = None;
            }
            WindowEvent::CursorMoved { position, .. } => {
                if position.y < 8.0 * state.window.scale_factor() && !state.menus.bar {
                    state.menus.bar = true;
                    state.ui_dirty = true;
                }
                if state.menus.pointer(position.x as f32, position.y as f32) {
                    state.ui_dirty = true;
                }
                if state.ui_dirty {
                    state.window.request_redraw();
                }
                if let Some((x, y)) = self.cursor
                    && self.dragging
                    && !state
                        .menus
                        .captures_pointer(position.x as f32, position.y as f32)
                {
                    let dx = (position.x - x) as f32 * 0.006;
                    match &mut self.flight {
                        Some(flight) if !flight.free_camera => flight.camera_yaw -= dx,
                        Some(_) => state.camera.yaw -= dx,
                        None => state.camera.yaw -= dx,
                    }
                    state.camera.pitch =
                        (state.camera.pitch + (position.y - y) as f32 * 0.006).clamp(-1.4, 1.4);
                    state.window.request_redraw();
                }
                self.cursor = Some((position.x, position.y));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if state.menus.modal()
                    || self
                        .cursor
                        .is_some_and(|(x, y)| state.menus.captures_pointer(x as f32, y as f32))
                {
                    return;
                }
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
                let raw_name = key_name(&event);
                let escape = matches!(event.logical_key, Key::Named(NamedKey::Escape));
                let alt = matches!(event.logical_key, Key::Named(NamedKey::Alt));
                let config = raw_name.as_deref() == Some("`");
                if escape || alt || config {
                    if phase == Phase::Begin {
                        if escape {
                            state.menus.escape();
                        } else if alt {
                            state.menus.alt();
                        } else {
                            state.menus.page = crate::ui::Page::Configuration;
                        }
                        if let Some(flight) = &mut self.flight {
                            flight.release_held();
                        }
                        self.dragging = false;
                        state.ui_dirty = true;
                        state.window.request_redraw();
                    }
                    return;
                }
                if state.menus.captures_keyboard() {
                    if phase == Phase::Begin {
                        if let Some(action) = state.menus.key(raw_name.as_deref().unwrap_or("")) {
                            if action == crate::ui::Action::StartFlight {
                                self.pending_flight = true;
                            }
                            apply_menu_action(
                                action,
                                &mut self.flight,
                                &mut state.camera,
                                event_loop,
                            );
                        }
                        state.ui_dirty = true;
                        state.window.request_redraw();
                    }
                    return;
                }
                let name = match &mut self.flight {
                    Some(flight) => routed_key(&mut flight.held_keys, &event),
                    None => key_name(&event),
                };
                match (&mut self.flight, name) {
                    (Some(flight), Some(name)) => {
                        if let Some(action) = flight.handle_key(&name, phase) {
                            let camera = &mut state.camera;
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
                                    let right =
                                        Vec3::new(-camera.yaw.cos(), 0.0, -camera.yaw.sin());
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

fn apply_menu_action(
    action: crate::ui::Action,
    flight: &mut Option<Flight>,
    camera: &mut Camera,
    event_loop: &ActiveEventLoop,
) {
    use crate::ui::Action;
    if action == Action::Quit {
        event_loop.exit();
        return;
    }
    let Some(flight) = flight else {
        return;
    };
    match action {
        Action::Resume => flight.paused = false,
        Action::Restart => {
            flight.handle_key("Delete", Phase::Begin);
        }
        Action::Pause => flight.paused = !flight.paused,
        Action::KeyboardStick => {
            flight.handle_key("Tab", Phase::Begin);
        }
        Action::ChaseCamera => {
            flight.camera_yaw = 0.0;
            flight.free_camera = false;
            flight.view_offset = Vec3::ZERO;
            camera.pitch = 0.3;
            camera.distance = camera.radius * 1.8;
        }
        Action::FreeCamera => flight.free_camera = !flight.free_camera,
        Action::Quit | Action::StartFlight => {}
    }
}

pub fn run(scene: Scene, smoke: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Drag left mouse: orbit. Scroll: zoom. R: reset. Alt: menu bar. Esc: main menu.");
    run_app(scene, None, None, smoke)
}

/// Flies the aircraft of an airport session with the approximate flight model.
pub fn run_flight(session: FlightSession, smoke: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Fly with the original's default keys: F1/F2/F3 throttle down/up/full, 1/2 flaps up/down, B brakes (hold), \
         V brakes max, [ ] pitch trim, 5/6/7 and 8/9/0 rudder and aileron trim, P pause, W default view. \
         openXplane's keyboard stick: arrows pitch/roll, Z/X rudder (Tab switches them to their original \
         meaning). Delete resets, Alt opens the menu bar, Esc opens the main menu."
    );
    let setup = crate::flight_setup::FlightSetup::new(
        &session.root,
        &session.aircraft_path,
        &session.airport_id,
    )?;
    let (scene, flight) = flight_from_session(session);
    run_app(scene, Some(flight), Some(setup), smoke)
}

fn flight_from_session(session: FlightSession) -> (Scene, Flight) {
    let FlightSession {
        scene, body, model, ..
    } = session;
    let commands = CommandInput::new(model.flap_detent_count());
    (
        scene,
        Flight {
            body,
            model,
            commands,
            keys: Keys::default(),
            controls: Controls::default(),
            held_keys: HashMap::new(),
            last: Instant::now(),
            accumulated: 0.0,
            camera_yaw: 0.0,
            paused: false,
            stick_keys: true,
            free_camera: false,
            view_offset: Vec3::ZERO,
            message: None,
        },
    )
}

impl App {
    fn start_selected_flight(&mut self) {
        let Some(state) = &mut self.state else {
            return;
        };
        let Some(setup) = &mut state.menus.setup else {
            return;
        };
        let result = (|| -> Result<(Scene, Flight), Box<dyn std::error::Error>> {
            if !setup.valid() {
                return Err("Enter an airport identifier".into());
            }
            let session = crate::scene::airport_flight(
                &setup.root,
                &setup.airport,
                &setup.aircraft[setup.selected],
            )?;
            let (scene, flight) = flight_from_session(session);
            state.renderer.replace_scene(&scene)?;
            Ok((scene, flight))
        })();
        match result {
            Ok((scene, flight)) => {
                self.scene = scene;
                self.flight = Some(flight);
                state.camera = Camera::new(&self.scene);
                state.menus.page = crate::ui::Page::Flight;
                setup.error = None;
                setup.editing_airport = false;
                self.dragging = false;
                println!(
                    "New flight loaded: {} {}",
                    setup.airport,
                    setup.aircraft[setup.selected].display()
                );
            }
            Err(error) => {
                eprintln!("New flight rejected; current session retained: {error}");
                setup.error = Some(error.to_string());
                state.menus.page = crate::ui::Page::Configuration;
            }
        }
        state.ui_dirty = true;
        state.window.request_redraw();
    }
}

fn run_app(
    scene: Scene,
    flight: Option<Flight>,
    setup: Option<crate::flight_setup::FlightSetup>,
    smoke: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = App {
        setup,
        pending_flight: false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::KeyCode;

    #[test]
    fn releasing_stick_input_centres_axes_and_preserves_persistent_controls() {
        let mut keys = Keys {
            pitch_back: true,
            roll_right: true,
            yaw_left: true,
            ..Keys::default()
        };
        let mut controls = Controls {
            elevator: 1.0,
            aileron: 0.5,
            rudder: -0.5,
            throttle: 0.7,
            elevator_trim: 0.2,
            flaps: 0.5,
            ..Controls::default()
        };
        keys.release(&mut controls);
        assert_eq!(
            (controls.elevator, controls.aileron, controls.rudder),
            (0.0, 0.0, 0.0)
        );
        assert!(!keys.pitch_back && !keys.roll_right && !keys.yaw_left);
        assert_eq!(
            (controls.throttle, controls.elevator_trim, controls.flaps),
            (0.7, 0.2, 0.5)
        );
    }

    #[test]
    fn release_and_repeat_use_the_original_binding() {
        let mut held = HashMap::new();
        let physical = PhysicalKey::Code(KeyCode::BracketLeft);
        assert_eq!(
            route_key(&mut held, physical, Phase::Begin, Some("[".into())).as_deref(),
            Some("[")
        );
        assert_eq!(
            route_key(&mut held, physical, Phase::Continue, Some("{".into())).as_deref(),
            Some("[")
        );
        assert_eq!(
            route_key(&mut held, physical, Phase::End, Some("{".into())).as_deref(),
            Some("[")
        );
        assert!(held.is_empty());
        assert!(route_key(&mut held, physical, Phase::End, Some("[".into())).is_none());
    }

    #[test]
    fn focus_reset_prevents_orphan_repeats_and_releases() {
        let mut held = HashMap::new();
        let physical = PhysicalKey::Code(KeyCode::KeyB);
        route_key(&mut held, physical, Phase::Begin, Some("B".into()));
        held.clear();
        assert!(route_key(&mut held, physical, Phase::Continue, Some("B".into())).is_none());
        assert!(route_key(&mut held, physical, Phase::End, Some("B".into())).is_none());
        assert_eq!(
            route_key(&mut held, physical, Phase::Begin, Some("B".into())).as_deref(),
            Some("B")
        );
    }
}

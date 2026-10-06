use openxplane::{Aircraft, reference_candidates};
mod dout;
mod gpu;
mod hud;
mod scene;
mod viewer;
use std::{collections::BTreeSet, env, fs, path::Path, process::ExitCode};

/// Size of offscreen frames: `OPENXPLANE_FRAME_SIZE=WxH`, else 1280x800.
fn frame_size() -> (u32, u32) {
    std::env::var("OPENXPLANE_FRAME_SIZE")
        .ok()
        .and_then(|v| {
            let (w, h) = v.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or((1280, 800))
}

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 5 && args[0] == "airfoil-replay" {
        let airfoil = openxplane::airfoil::Airfoil::parse(&fs::read_to_string(&args[1])?)?;
        let inputs = openxplane::profile::parse_replay(&fs::read_to_string(&args[2])?)?;
        let noise = if args[3] == "-" {
            None
        } else {
            let bytes = fs::read(&args[3])?;
            if bytes.len() != openxplane::buffet::TABLE_LEN * 4 {
                return Err("expected exactly 262144 little-endian float32 table entries".into());
            }
            Some(openxplane::buffet::NoiseTable::new(
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|b| f32::from_le_bytes(*b))
                    .collect(),
            )?)
        };
        let initially_stalled = match args[4].to_str() {
            Some("0") => false,
            Some("1") => true,
            _ => return Err("initial stall must be 0 or 1".into()),
        };
        let outputs =
            openxplane::profile::replay(&airfoil, &inputs, initially_stalled, noise.as_ref())?;
        println!(
            "sample,time,lower,upper,effective_lower,effective_upper,normalized,stalled,cl,cd,cm"
        );
        for (index, (input, output)) in inputs.iter().zip(outputs.iter()).enumerate() {
            let upper = output.upper.as_ref().unwrap_or(&output.lower);
            println!(
                "{},{:.17},{},{},{:.9},{:.9},{:.9},{},{:.9},{:.9},{:.9}",
                index + 1,
                input.time.seconds(),
                output.selection.lower,
                output.selection.upper,
                output.lower.table.effective_alpha_deg,
                upper.table.effective_alpha_deg,
                output.normalized_alpha,
                u8::from(output.stalled),
                output.coefficients.cl,
                output.coefficients.cd,
                output.coefficients.cm
            );
        }
        return Ok(true);
    }
    if args.len() == 9 && args[0] == "buffet-eval" {
        let bytes = fs::read(&args[1])?;
        if bytes.len() != openxplane::buffet::TABLE_LEN * 4 {
            return Err("expected exactly 262144 little-endian float32 table entries".into());
        }
        let values = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        let table = openxplane::buffet::NoiseTable::new(values)?;
        let number = |i: usize| -> Result<f32, Box<dyn std::error::Error>> {
            Ok(args[i]
                .to_str()
                .ok_or("invalid numeric argument")?
                .parse::<f32>()?)
        };
        let running_time_seconds = args[5]
            .to_str()
            .ok_or("invalid running time argument")?
            .parse::<f64>()?;
        let time = openxplane::runtime::RunningTime::from_snapshot(running_time_seconds)?;
        let c = table.perturb_at_time(
            openxplane::airfoil::Coefficients {
                cl: number(6)?,
                cd: number(7)?,
                cm: number(8)?,
            },
            [number(2)?, number(3)?, number(4)?],
            time,
        )?;
        println!("Active-stall perturbation stage with explicitly supplied noise table.");
        println!(
            "{}={:.17} seconds (internal double snapshot)",
            openxplane::runtime::RUNNING_TIME_DATAREF,
            time.seconds()
        );
        println!("Cl={:.9}, Cd={:.9}, Cm={:.9}", c.cl, c.cd, c.cm);
        return Ok(true);
    }
    if args.len() == 6 && args[0] == "airfoil-mix" {
        let airfoil = openxplane::airfoil::Airfoil::parse(&fs::read_to_string(&args[1])?)?;
        let number = |i: usize| -> Result<f32, Box<dyn std::error::Error>> {
            Ok(args[i]
                .to_str()
                .ok_or("invalid numeric argument")?
                .parse::<f32>()?)
        };
        let (selection, c) = openxplane::regimes::evaluate_table_stage(
            &airfoil,
            number(5)?,
            number(2)?,
            number(3)?,
            number(4)?,
        )?;
        println!("Diagnostic regime blend of table stage; later evaluator corrections excluded.");
        println!(
            "Polars: {}/{}; Cl={:.9}, Cd={:.9}, Cm={:.9}",
            selection.lower, selection.upper, c.cl, c.cd, c.cm
        );
        return Ok(true);
    }
    if args.len() == 5 && args[0] == "airfoil-eval" {
        let airfoil = openxplane::airfoil::Airfoil::parse(&fs::read_to_string(&args[1])?)?;
        let number = |i: usize| -> Result<f32, Box<dyn std::error::Error>> {
            Ok(args[i]
                .to_str()
                .ok_or("invalid numeric argument")?
                .parse::<f32>()?)
        };
        let alpha = number(2)?;
        let multiplier = number(3)?;
        let divisor = number(4)?;
        println!(
            "Reference angle/table stage: induced-alpha={alpha}, multiplier={multiplier}, divisor={divisor}"
        );
        println!(
            "Each polar evaluated separately; later corrections and force integration excluded."
        );
        for (index, polar) in airfoil.polars.iter().enumerate() {
            let s = openxplane::aero::evaluate_table(polar, alpha, multiplier, divisor)?;
            let state = openxplane::stall::evaluate(polar, s.lookup_alpha_deg, false, false)?;
            println!(
                "  Polar {index}: normalized-alpha={:.9}, stalled={} (state retention disabled; perturbations excluded)",
                state.normalized_alpha, state.stalled
            );
            println!(
                "  Polar {index}: effective-alpha={:.9}, lookup-alpha={:.9}, indices={}/{}, weights={:.9}/{:.9}, Cl={:.9}, Cd={:.9}, Cm={:.9}",
                s.effective_alpha_deg,
                s.lookup_alpha_deg,
                s.lower,
                s.upper,
                s.lower_weight,
                s.upper_weight,
                s.coefficients.cl,
                s.coefficients.cd,
                s.coefficients.cm
            );
        }
        return Ok(true);
    }
    if (args.len() == 2 || args.len() == 3) && args[0] == "airfoil-info" {
        let airfoil = openxplane::airfoil::Airfoil::parse(&fs::read_to_string(&args[1])?)?;
        let alpha = if args.len() == 3 {
            args[2]
                .to_str()
                .ok_or("invalid alpha argument")?
                .parse::<f32>()?
        } else {
            0.0
        };
        println!(
            "AFL {}: {} polars, header scalars {:?}",
            airfoil.version,
            airfoil.polars.len(),
            airfoil.header_scalars
        );
        println!(
            "Diagnostic linear sampling at {alpha} degrees; no Reynolds blending or flight physics."
        );
        for (index, polar) in airfoil.polars.iter().enumerate() {
            let c = polar.sample_linear(alpha)?;
            println!(
                "  Polar {index}: parameter[0]={}; {} rows; Cl={:.6}, Cd={:.6}, Cm={:.6}",
                polar.parameters[0],
                polar.rows.len(),
                c.cl,
                c.cd,
                c.cm
            );
        }
        return Ok(true);
    }
    if args.len() == 3 && args[0] == "airport-info" {
        let id = args[2].to_string_lossy();
        let path = Path::new(&args[1]);
        // An installation directory searches the apt.dat sources in priority order; a file is read alone.
        let found = if path.is_dir() {
            openxplane::install::Install::open(path)?.find_airport(&id)?
        } else {
            let file = std::io::BufReader::new(fs::File::open(path)?);
            openxplane::apt::find_airport(file, &id)?.map(|a| (a, path.to_path_buf()))
        };
        let Some((a, source)) = found else {
            eprintln!("airport {id} not found");
            return Ok(false);
        };
        println!("Source: {}", source.display());
        println!(
            "{} {} (header row {}, line {})",
            a.id, a.name, a.kind, a.line
        );
        println!("  Elevation: {} ft", a.elevation_ft);
        for (k, v) in &a.metadata {
            println!("  {k}: {v}");
        }
        for r in &a.runways {
            let [e0, e1] = &r.ends;
            println!(
                "  Runway {}/{}: width {} m, surface {}, ends ({:.7}, {:.7}) - ({:.7}, {:.7})",
                e0.name, e1.name, r.width_m, r.surface, e0.lat, e0.lon, e1.lat, e1.lon
            );
        }
        let others: Vec<String> = a
            .other_rows
            .iter()
            .map(|(c, n)| format!("{c}x{n}"))
            .collect();
        println!("  Not interpreted (row code x count): {}", others.join(" "));
        return Ok(true);
    }
    if (args.len() == 2 || args.len() == 3) && args[0] == "commands" {
        let mut registry = openxplane::commands::CommandRegistry::new();
        let count = registry.load_catalog(&fs::read_to_string(&args[1])?)?;
        let prefix = args
            .get(2)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut shown = 0usize;
        for name in registry.names().filter(|n| n.starts_with(&prefix)) {
            println!("{name}\t{}", registry.description(name)?);
            shown += 1;
        }
        eprintln!("{shown} of {count} commands");
        return Ok(true);
    }
    if args.len() == 2 && args[0] == "wing-info" {
        let aircraft = Aircraft::parse(&fs::read_to_string(&args[1])?)?;
        let wings = openxplane::wing::Wing::all_from_acf(&aircraft)?;
        let ars = openxplane::wing::aspect_ratios(&wings);
        let mut total = 0.0;
        println!("Lifting surfaces (metres, degrees; body frame x right, y up, z aft):");
        for (w, ar) in wings.iter().zip(&ars) {
            total += w.area();
            let carries: Vec<&str> = ["aileron", "elevator", "rudder", "flap"]
                .iter()
                .enumerate()
                .filter(|(i, _)| w.controls.iter().any(|c| c[*i] > 0.0))
                .map(|(_, n)| *n)
                .collect();
            println!(
                "  wing {:>2}: root ({:6.2}, {:5.2}, {:5.2}) semilen {:5.2} chord {:4.2}->{:4.2} dihedral {:5.1} sweep {:5.1} elements {} area {:6.3} m2 AR {:5.2} {} [{}]",
                w.index,
                w.root.x,
                w.root.y,
                w.root.z,
                w.semilen,
                w.root_chord,
                w.tip_chord,
                w.dihedral_deg,
                w.sweep_deg,
                w.elements,
                w.area(),
                ar,
                w.airfoils[0],
                carries.join(",")
            );
        }
        println!("Total planform area of all surfaces: {total:.2} m2");
        return Ok(true);
    }
    if (args.len() == 3 || args.len() == 4) && args[0] == "fly-test" {
        // Scripted takeoff for checking the approximate flight model without a window.
        let install = openxplane::install::Install::open(Path::new(&args[1]))?;
        let acf = Path::new(&args[2]);
        let seconds: f32 = args
            .get(3)
            .and_then(|s| s.to_str()?.parse().ok())
            .unwrap_or(60.0);
        let scene = scene::load(acf)?;
        let wheel_bottom = scene
            .meshes
            .iter()
            .flat_map(|m| m.vertices.iter().map(|v| v.position[1]))
            .fold(f32::INFINITY, f32::min);
        let mut model = openxplane::flight::FlightModel::load(acf, &install, 80.0, 90.0)?;
        model.rest_on_ground(wheel_bottom, [0.0, 0.0], glam::Vec3::NEG_Z);
        println!(
            "mass {:.0} kg, inertia {:?}, {} gears, {} wings",
            model.mass,
            model.inertia,
            model.gears.len(),
            model.wings().len()
        );
        let mut c = openxplane::flight::Controls::default();
        let dt = 1.0 / 200.0;
        let mut liftoff = None;
        for i in 0..(seconds * 200.0) as usize {
            let t = model.telemetry(&c);
            openxplane::flight::takeoff_pilot(&model, &mut c);
            if !t.on_ground && liftoff.is_none() {
                liftoff = Some((model.state.time, t.airspeed_kt));
            }
            model.step(dt, &c);
            if i % 400 == 0 {
                let t = model.telemetry(&c);
                if std::env::var_os("FLY_DEBUG").is_some() {
                    for (w, f, m) in &model.wing_report {
                        println!(
                            "    wing {w:>2}: force ({:8.1} {:8.1} {:8.1}) N  moment ({:8.1} {:8.1} {:8.1}) Nm",
                            f.x, f.y, f.z, m.x, m.y, m.z
                        );
                    }
                }
                println!(
                    "t {:5.1}s  IAS {:5.1} kt  alt {:7.1} ft  vs {:6.0} fpm  pitch {:5.1}  roll {:5.1}  hdg {:5.1}  alpha {:5.1}  gnd {}  stalled {}",
                    model.state.time,
                    t.airspeed_kt,
                    t.altitude_ft,
                    t.vertical_speed_fpm,
                    t.pitch_deg,
                    t.roll_deg,
                    t.heading_deg,
                    t.alpha_deg,
                    t.on_ground,
                    t.stalled_elements
                );
            }
        }
        match liftoff {
            Some((t, v)) => println!("liftoff at {t:.1} s, {v:.0} kt"),
            None => println!("no liftoff"),
        }
        return Ok(true);
    }
    if args.len() >= 5 && args[0] == "fly-render" {
        // Renders chase-camera frames of the scripted takeoff at the given times (seconds). With
        // OPENXPLANE_FRAME_SIZE=WxH the frames are smaller (for animations).
        let mut session = scene::airport_flight(
            Path::new(&args[1]),
            &args[2].to_string_lossy(),
            Path::new(&args[3]),
        )?;
        let prefix = args[4].to_string_lossy().into_owned();
        let mut times: Vec<f32> = args[5..]
            .iter()
            .filter_map(|a| a.to_str()?.parse().ok())
            .collect();
        if times.is_empty() {
            times = vec![0.0, 18.0, 30.0, 50.0];
        }
        times.sort_by(f32::total_cmp);
        let (w, h) = frame_size();
        let mut offscreen = pollster::block_on(gpu::Offscreen::new(&session.scene, w, h))?;
        let mut c = openxplane::flight::Controls::default();
        for t in times {
            while session.model.state.time < f64::from(t) {
                openxplane::flight::takeoff_pilot(&session.model, &mut c);
                session.model.step(1.0 / 200.0, &c);
            }
            session.body.apply(&mut session.scene, &session.model);
            let mut camera = gpu::Camera::new(&session.scene);
            let forward = session.model.state.orientation * glam::Vec3::NEG_Z;
            camera.yaw = std::f32::consts::PI - (-forward.x).atan2(-forward.z) + 0.55;
            camera.pitch = 0.18;
            camera.distance = 24.0;
            if std::env::var_os("OPENXPLANE_FRAME_RATE").is_some() {
                // the sample values of the reference photograph, to compare the layout
                let (fw, fh) = offscreen.size();
                let mut hud = hud::Hud::new(fw, fh);
                let scale = (fh as f32 / 800.0).clamp(0.7, 1.6);
                dout::draw_line(
                    &mut hud,
                    16.0 * scale,
                    16.0 * scale,
                    scale,
                    dout::FRAME_RATE_NO_VSYNC,
                    &[
                        Some(10.462),
                        Some(19.9),
                        None,
                        Some(0.0956),
                        Some(0.0698),
                        Some(0.0956),
                        Some(1.0),
                        Some(1.0),
                    ],
                );
                offscreen.set_hud(&hud.vertices);
            }
            let out = format!("{prefix}-{t:06.2}s.png");
            offscreen.render(
                &session.scene,
                session.body.first_mesh,
                &camera,
                Path::new(&out),
            )?;
            let tel = session.model.telemetry(&c);
            println!(
                "t {t:6.2}s  IAS {:5.1} kt  alt {:6.1} ft  pitch {:5.1}  -> {out}",
                tel.airspeed_kt, tel.altitude_ft, tel.pitch_deg
            );
        }
        return Ok(true);
    }
    if args.len() == 4 && args[0] == "turntable" {
        // N frames of the aircraft preview with the camera going once around it.
        let scene = scene::load(Path::new(&args[1]))?;
        let prefix = args[2].to_string_lossy().into_owned();
        let frames: usize = args[3]
            .to_str()
            .and_then(|s| s.parse().ok())
            .ok_or("frame count")?;
        let (w, h) = frame_size();
        let mut offscreen = pollster::block_on(gpu::Offscreen::new(&scene, w, h))?;
        for i in 0..frames {
            let mut camera = gpu::Camera::new(&scene);
            camera.yaw = -0.7 + std::f32::consts::TAU * i as f32 / frames as f32;
            offscreen.render(
                &scene,
                0,
                &camera,
                Path::new(&format!("{prefix}-{i:03}.png")),
            )?;
        }
        println!("{frames} frames");
        return Ok(true);
    }
    if args.len() == 2 && args[0] == "datarefs" {
        let aircraft = Aircraft::parse(&fs::read_to_string(&args[1])?)?;
        let p = openxplane::aircraft::AircraftParameters::from_acf(&aircraft)?;
        let mut registry = openxplane::dataref::Registry::new();
        registry.register_aircraft(&p)?;
        println!(
            "Aircraft datarefs registered from the ACF (type/writability per reference build):"
        );
        for name in registry.names() {
            let writable = if registry.is_writable(name)? {
                "rw"
            } else {
                "ro"
            };
            let value = match registry.data_type(name)? {
                openxplane::dataref::DataType::Int => registry.get_int(name)?.to_string(),
                _ => format!("{:.6}", registry.get_float(name)?),
            };
            println!(
                "  {name} {:?} {writable} = {value}",
                registry.data_type(name)?
            );
        }
        return Ok(true);
    }
    if args.len() == 2 && args[0] == "aircraft-info" {
        let aircraft = Aircraft::parse(&fs::read_to_string(&args[1])?)?;
        let p = openxplane::aircraft::AircraftParameters::from_acf(&aircraft)?;
        println!("Typed ACF parameters (reference-build conversion factors):");
        println!("  Empty mass: {:.6} kg", p.empty_mass_kg);
        println!("  Maximum mass: {:.6} kg", p.maximum_mass_kg);
        println!("  Maximum fuel mass: {:.6} kg", p.maximum_fuel_mass_kg);
        println!(
            "  CG in ACF reference frame: Y={:.6} m, Z={:.6} m",
            p.cg_y_acf_m, p.cg_z_acf_m
        );
        println!("  Engines: {}", p.engine_count);
        println!("No physics, loading defaults or axis transformations are implied.");
        return Ok(true);
    }
    if args.len() == 2 && args[0] == "mesh-info" {
        return scene::audit(Path::new(&args[1]));
    }
    if (args.len() == 2 || (args.len() == 3 && args[2] == "--smoke")) && args[0] == "view" {
        let scene = scene::load(Path::new(&args[1]))?;
        // Rich Presence is optional: it needs OPENXPLANE_DISCORD_APP_ID and a running Discord.
        let _presence = match openxplane::discord::Presence::connect_from_env() {
            Ok(Some(mut presence)) => {
                let name = Path::new(&args[1])
                    .file_stem()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let started = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .ok();
                let activity = openxplane::discord::Activity {
                    details: "Viewing an aircraft".into(),
                    state: name,
                    start_timestamp: started,
                };
                if let Err(error) = presence.set_activity(&activity) {
                    eprintln!("{error}");
                }
                Some(presence)
            }
            Ok(None) => None,
            Err(error) => {
                eprintln!("{error}");
                None
            }
        };
        viewer::run(scene, args.len() == 3)?;
        return Ok(true);
    }
    if (args.len() == 4 || (args.len() == 5 && args[4] == "--smoke")) && args[0] == "fly" {
        let session = scene::airport_flight(
            Path::new(&args[1]),
            &args[2].to_string_lossy(),
            Path::new(&args[3]),
        )?;
        viewer::run_flight(session, args.len() == 5)?;
        return Ok(true);
    }
    if (args.len() == 4 || (args.len() == 5 && args[4] == "--smoke")) && args[0] == "airport-view" {
        let scene = scene::airport(
            Path::new(&args[1]),
            &args[2].to_string_lossy(),
            Path::new(&args[3]),
        )?;
        viewer::run(scene, args.len() == 5)?;
        return Ok(true);
    }
    if args.len() == 5 && args[0] == "airport-render" {
        let scene = scene::airport(
            Path::new(&args[1]),
            &args[2].to_string_lossy(),
            Path::new(&args[3]),
        )?;
        pollster::block_on(gpu::render_png(&scene, Path::new(&args[4])))?;
        return Ok(true);
    }
    if args.len() == 3 && args[0] == "render" {
        let scene = scene::load(Path::new(&args[1]))?;
        pollster::block_on(gpu::render_png(&scene, Path::new(&args[2])))?;
        return Ok(true);
    }
    if args.len() != 2 || args[0] != "inspect" {
        return Err("Usage: openxplane airfoil-replay <airfoil.afl> <inputs.csv> <table.f32le|-> <initial-stall:0|1> | inspect <content-root> | aircraft-info <aircraft.acf> | airfoil-info <airfoil.afl> [alpha-degrees] | airfoil-eval <airfoil.afl> <induced-alpha> <multiplier> <divisor> | airfoil-mix <airfoil.afl> <induced-alpha> <multiplier> <divisor> <regime-parameter> | buffet-eval <table.f32le> <x> <y> <phase> <running-time-seconds> <cl> <cd> <cm> | mesh-info <aircraft.acf> | view <aircraft.acf> [--smoke] | render <aircraft.acf> <output.png>".into());
    }
    let root = fs::canonicalize(&args[1])?;
    let install = openxplane::install::Install::open(&root)?;
    let paths = install.find_acf()?;
    if paths.is_empty() {
        return Err("No ACF aircraft found".into());
    }
    println!("Installation: {}", root.display());
    for (label, dir) in [
        ("Aircraft", install.aircraft_dir()),
        ("Airfoils", install.airfoils_dir()),
        ("Custom Data", install.custom_data_dir()),
        ("Custom Scenery", install.custom_scenery_dir()),
        ("Global Airports", install.global_airports_dir()),
        ("Resources/default data", install.default_data_dir()),
    ] {
        println!(
            "  {label}: {}",
            if dir.is_dir() { "present" } else { "absent" }
        );
    }
    for name in ["earth_nav.dat", "earth_fix.dat", "earth_awy.dat"] {
        match install.nav_data_file(name) {
            Some(p) => println!(
                "  {name}: {}",
                p.strip_prefix(&root).unwrap_or(&p).display()
            ),
            None => println!("  {name}: not found"),
        }
    }
    for source in install.apt_dat_sources()? {
        println!(
            "  apt.dat source: {}",
            source.strip_prefix(&root).unwrap_or(&source).display()
        );
    }
    let mut complete = true;
    for path in paths {
        let aircraft = Aircraft::parse(&fs::read_to_string(&path)?)?;
        println!("Aircraft: {}", path.strip_prefix(&root)?.display());
        println!(
            "  ACF version: {}; properties: {}",
            aircraft.version,
            aircraft.properties.len()
        );
        let mut references = BTreeSet::new();
        let mut missing = BTreeSet::new();
        let mut airfoil_paths = BTreeSet::new();
        for property in aircraft.references() {
            let candidates = reference_candidates(path.parent().unwrap(), &root, property);
            references.insert(property.value.clone());
            if !candidates.iter().any(|p| p.is_file()) {
                missing.insert(format!(
                    "{} (line {}, {})",
                    property.value, property.line, property.key
                ));
            }
            if property.key.contains("/_afl_file_") {
                // Audit every existing candidate without assuming the original's search precedence.
                airfoil_paths.extend(candidates.into_iter().filter(|p| p.is_file()));
            }
        }
        println!(
            "  Unique model/airfoil references: {}; missing: {}",
            references.len(),
            missing.len()
        );
        for reference in missing {
            println!("  MISSING: {reference}");
            complete = false;
        }
        for airfoil_path in airfoil_paths {
            let result = fs::read_to_string(&airfoil_path)
                .map_err(|e| e.to_string())
                .and_then(|text| openxplane::airfoil::Airfoil::parse(&text));
            match result {
                Ok(airfoil) => println!(
                    "  AFL: {} — {} polars, {} rows each",
                    airfoil_path.strip_prefix(&root)?.display(),
                    airfoil.polars.len(),
                    openxplane::airfoil::POLAR_ROWS
                ),
                Err(error) => {
                    println!("  AFL ERROR: {}: {error}", airfoil_path.display());
                    complete = false;
                }
            }
        }
    }
    println!(
        "Scope: ACF model/airfoil references and AFL tables; textures, panels, scripts and scenery are not yet checked."
    );
    Ok(complete)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

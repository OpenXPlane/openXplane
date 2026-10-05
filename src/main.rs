use openxplane::{Aircraft, reference_candidates};
mod gpu;
mod scene;
mod viewer;
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

fn collect_acf(dir: &Path, result: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect_acf(&entry.path(), result)?;
        } else if kind.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("acf"))
        {
            result.push(entry.path());
        }
    }
    Ok(())
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
                    .chunks_exact(4)
                    .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
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
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
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
        let file = std::io::BufReader::new(fs::File::open(&args[1])?);
        let id = args[2].to_string_lossy();
        let Some(a) = openxplane::apt::find_airport(file, &id)? else {
            eprintln!("airport {id} not found");
            return Ok(false);
        };
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
        viewer::run(scene, args.len() == 3)?;
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
    let mut paths = Vec::new();
    collect_acf(&root, &mut paths)?;
    paths.sort();
    if paths.is_empty() {
        return Err("No ACF aircraft found".into());
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

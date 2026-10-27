//! A transparent GNU objdump range adapter for REA's inspect-with-objdump.
//! It reads the complete original PE and adds only fixed start/stop options.
//! Backend and added options are disclosed in captured stderr. No code bytes
//! are embedded here; Evidence and original listings must remain local.
use std::process::{Command, ExitCode};
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 || args[0] != "-d" {
        eprintln!("adapter expects exactly -d PATH");
        return ExitCode::from(2);
    }
    let range = std::env::var("OPENXPLANE_REA_RANGE").unwrap_or_default();
    let (start, stop) = match range.as_str() {
        "mode" => ("0x1414cedd0", "0x1414cef70"),
        "header" => ("0x1411222e0", "0x14112240c"),
        "singleton" => ("0x1414cc7f0", "0x1414cc814"),
        "fps-pics" => ("0x140e6d3c1", "0x140e6d44b"),
        "attachment" => ("0x141aaa610", "0x141aaaac1"),
        "attachment-gate" => ("0x141a7b6a0", "0x141a7b72c"),
        "attachment-view" => ("0x1407b4780", "0x1407b47f7"),
        "attachment-matrix" => ("0x141775d10", "0x141775ece"),
        "obj-key-insert" => ("0x1408751b0", "0x1408752b0"),
        "facade-segment-loader" => ("0x141981450", "0x141981650"),
        "shader-mapping-header" => ("0x14103fafa", "0x14103fb2b"),
        "shader-mapping-reader" => ("0x14105e350", "0x14105f675"),
        "shader-mapping-names" => ("0x141060320", "0x1410604b5"),
        "shader-mapping-record" => ("0x14105dec0", "0x14105e32e"),
        "terrain-shader-registration" => ("0x14176e120", "0x14176eb6d"),
        "shader-range-record" => ("0x1409168d0", "0x140916957"),
        "shader-value-record" => ("0x1417697a0", "0x141769828"),
        "shader-registration-base" => ("0x14068e2c0", "0x14068e38f"),
        "shader-registration-empty" => ("0x14103cdb0", "0x14103cdc8"),
        "terrain-shader-factory" => ("0x14176cb30", "0x14176ce76"),
        "shader-variant-index" => ("0x14104f0a0", "0x14104f16a"),
        "terrain-material-digits" => ("0x14176b3f0", "0x14176b795"),
        "terrain-detail-mode" => ("0x1419a84b3", "0x1419a851e"),
        "terrain-composite-texture" => ("0x1419a7376", "0x1419a765f"),
        "terrain-composite-nowrap" => ("0x1419a765f", "0x1419a7937"),
        "terrain-variant-validation" => ("0x14176eb70", "0x14176ec94"),
        "terrain-projected-fields" => ("0x1419a7e05", "0x1419a7e59"),
        "terrain-composite-border-field" => ("0x1419a8365", "0x1419a838f"),
        "composite-parameter-fields" => ("0x140c2e780", "0x140c2e850"),
        "ter-composite-parameter-fields" => ("0x1419a9f64", "0x1419aa016"),
        "terrain-uniform-packing" => ("0x1417705c0", "0x1417709b2"),
        "terrain-uniform-cache" => ("0x1417702b0", "0x1417705b8"),
        "ter-composite-projection" => ("0x1419a82fc", "0x1419a8365"),
        "ter-far" => ("0x1419aa04e", "0x1419aa0fc"),
        "ter-near" => ("0x1419aaa08", "0x1419aaa9f"),
        "ter-far-bias" => ("0x1405b0240", "0x1405b027e"),
        "terrain-material-defaults" => ("0x14068a050", "0x14068a28e"),
        _ => {
            eprintln!("select OPENXPLANE_REA_RANGE mode/header/singleton/fps-pics/attachment/attachment-gate/attachment-view/attachment-matrix/obj-key-insert/facade-segment-loader/shader-mapping-header/shader-mapping-reader/shader-mapping-names/shader-mapping-record/terrain-shader-registration/shader-range-record/shader-value-record/shader-registration-base/shader-registration-empty/terrain-shader-factory/shader-variant-index/terrain-material-digits/terrain-detail-mode/terrain-composite-texture/terrain-variant-validation/terrain-projected-fields/terrain-composite-border-field/composite-parameter-fields/ter-composite-parameter-fields/terrain-uniform-packing/terrain-uniform-cache/ter-composite-projection");
            return ExitCode::from(2);
        }
    };
    let exe = std::env::var("OPENXPLANE_OBJDUMP").unwrap_or_else(|_| "objdump".into());
    eprintln!("openXplane bounded objdump adapter; backend={exe}; added --start-address={start} --stop-address={stop}; original input unchanged; this is a bounded listing, not whole-binary analysis");
    match Command::new(&exe)
        .args(&args)
        .arg(format!("--start-address={start}"))
        .arg(format!("--stop-address={stop}"))
        .status()
    {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

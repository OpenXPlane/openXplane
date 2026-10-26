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
        _ => {
            eprintln!("select OPENXPLANE_REA_RANGE mode/header/singleton/fps-pics/attachment");
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

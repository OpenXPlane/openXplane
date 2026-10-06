//! Streaming apt.dat (format 1200) reader for one airport at a time.
//! Airport headers (1/16/17), land runways (100), metadata (1302) and pavement outlines
//! (110 with nodes 111-114) are typed; every other row code is counted, not interpreted.
use std::collections::BTreeMap;
use std::io::BufRead;

#[derive(Debug, PartialEq)]
pub struct RunwayEnd {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub displaced_threshold_m: f32,
    pub overrun_m: f32,
    pub markings: u32,
    pub approach_lights: u32,
    pub touchdown_lights: bool,
    pub reil: u32,
}

#[derive(Debug, PartialEq)]
pub struct Runway {
    pub width_m: f32,
    pub surface: u32,
    pub shoulder: u32,
    pub smoothness: f32,
    pub center_lights: bool,
    pub edge_lights: u32,
    pub distance_signs: bool,
    pub ends: [RunwayEnd; 2],
    pub line: usize,
}

/// A pavement outline node. `ctrl` is the bezier control point of codes 112/114.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Node {
    pub lat: f64,
    pub lon: f64,
    pub ctrl: Option<(f64, f64)>,
}

/// Row 110 with its node rows. The first loop is the outer boundary; later loops are holes
/// (documented apt.dat convention, not confirmed in the reference build). A loop ends at a
/// closing node (113 or 114).
#[derive(Debug, Clone, PartialEq)]
pub struct Pavement {
    pub surface: u32,
    pub loops: Vec<Vec<Node>>,
    pub line: usize,
}

#[derive(Debug, PartialEq)]
pub struct Airport {
    /// Header row code: 1 land airport, 16 seaplane base, 17 heliport.
    pub kind: u32,
    pub elevation_ft: f32,
    pub id: String,
    pub name: String,
    pub metadata: Vec<(String, String)>,
    pub runways: Vec<Runway>,
    pub pavements: Vec<Pavement>,
    /// Row code -> count for rows inside this airport that are not interpreted.
    pub other_rows: BTreeMap<u32, usize>,
    pub line: usize,
}

impl Airport {
    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

fn num<T: std::str::FromStr>(token: &str, what: &str, line: usize) -> Result<T, String> {
    token
        .parse()
        .map_err(|_| format!("invalid {what} '{token}' at line {line}"))
}

fn parse_header(code: u32, fields: &[&str], line: usize) -> Result<Airport, String> {
    // code elevation deprecated deprecated id name...
    if fields.len() < 5 {
        return Err(format!("short airport header at line {line}"));
    }
    Ok(Airport {
        kind: code,
        elevation_ft: num(fields[1], "elevation", line)?,
        id: fields[4].to_string(),
        name: fields[5..].join(" "),
        metadata: Vec::new(),
        runways: Vec::new(),
        pavements: Vec::new(),
        other_rows: BTreeMap::new(),
        line,
    })
}

fn parse_end(f: &[&str], line: usize) -> Result<RunwayEnd, String> {
    Ok(RunwayEnd {
        name: f[0].to_string(),
        lat: num(f[1], "latitude", line)?,
        lon: num(f[2], "longitude", line)?,
        displaced_threshold_m: num(f[3], "displaced threshold", line)?,
        overrun_m: num(f[4], "overrun", line)?,
        markings: num(f[5], "markings", line)?,
        approach_lights: num(f[6], "approach lights", line)?,
        touchdown_lights: num::<u32>(f[7], "touchdown lights", line)? != 0,
        reil: num(f[8], "REIL", line)?,
    })
}

fn parse_runway(fields: &[&str], line: usize) -> Result<Runway, String> {
    if fields.len() < 26 {
        return Err(format!("short runway row at line {line}"));
    }
    Ok(Runway {
        width_m: num(fields[1], "width", line)?,
        surface: num(fields[2], "surface", line)?,
        shoulder: num(fields[3], "shoulder", line)?,
        smoothness: num(fields[4], "smoothness", line)?,
        center_lights: num::<u32>(fields[5], "center lights", line)? != 0,
        edge_lights: num(fields[6], "edge lights", line)?,
        distance_signs: num::<u32>(fields[7], "distance signs", line)? != 0,
        ends: [
            parse_end(&fields[8..17], line)?,
            parse_end(&fields[17..26], line)?,
        ],
        line,
    })
}

fn parse_node(code: u32, fields: &[&str], line: usize) -> Result<Node, String> {
    let bezier = matches!(code, 112 | 114 | 116);
    if fields.len() < if bezier { 5 } else { 3 } {
        return Err(format!("short pavement node row at line {line}"));
    }
    let lat = num(fields[1], "latitude", line)?;
    let lon = num(fields[2], "longitude", line)?;
    let ctrl = if bezier {
        Some((
            num(fields[3], "control latitude", line)?,
            num(fields[4], "control longitude", line)?,
        ))
    } else {
        None
    };
    Ok(Node { lat, lon, ctrl })
}

/// Reads only until the end of the requested airport block.
/// Returns `Ok(None)` if no header with this id exists.
pub fn find_airport<R: BufRead>(mut reader: R, id: &str) -> Result<Option<Airport>, String> {
    let mut buf = String::new();
    let mut line_no = 0usize;
    let mut current: Option<Airport> = None;
    let mut seen_version = false;
    // Node rows are interpreted only directly after a 110 pavement header (they also occur in
    // linear features 120 and boundaries 130, which are not interpreted).
    let mut in_pavement = false;
    let mut open_loop: Vec<Node> = Vec::new();
    loop {
        buf.clear();
        if reader.read_line(&mut buf).map_err(|e| e.to_string())? == 0 {
            return Ok(current);
        }
        line_no += 1;
        let text = buf.trim_start_matches('\u{feff}').trim();
        if line_no == 1 {
            if !matches!(text, "I" | "A") {
                return Err("unsupported apt.dat header marker".into());
            }
            continue;
        }
        if text.is_empty() {
            continue;
        }
        let fields: Vec<&str> = text.split_whitespace().collect();
        if !seen_version {
            if fields[0] != "1200" {
                return Err(format!("unsupported apt.dat version '{}'", fields[0]));
            }
            seen_version = true;
            continue;
        }
        let code: u32 = num(fields[0], "row code", line_no)?;
        if matches!(code, 1 | 16 | 17 | 99) {
            if current.is_some() {
                return Ok(current);
            }
            if code != 99 && fields.get(4) == Some(&id) {
                current = Some(parse_header(code, &fields, line_no)?);
            }
            continue;
        }
        let Some(airport) = current.as_mut() else {
            continue;
        };
        if matches!(code, 111..=116) && in_pavement {
            let node = parse_node(code, &fields, line_no)?;
            open_loop.push(node);
            if matches!(code, 113 | 114) {
                let nodes = std::mem::take(&mut open_loop);
                airport.pavements.last_mut().unwrap().loops.push(nodes);
            }
            continue;
        }
        in_pavement = code == 110;
        match code {
            110 => {
                if fields.len() < 2 {
                    return Err(format!("short pavement row at line {line_no}"));
                }
                open_loop.clear();
                airport.pavements.push(Pavement {
                    surface: num(fields[1], "pavement surface", line_no)?,
                    loops: Vec::new(),
                    line: line_no,
                });
            }
            100 => airport.runways.push(parse_runway(&fields, line_no)?),
            1302 if fields.len() >= 2 => airport
                .metadata
                .push((fields[1].to_string(), fields[2..].join(" "))),
            _ => *airport.other_rows.entry(code).or_insert(0) += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "A\n1200 test\n\n\
1 100 0 0 AAAA First Field\n\
1302 city Nowhere\n\
100 30.00 1 0 0.25 0 1 0 09 10.0 20.0 0 0 3 0 0 0 27 10.0 20.01 12.5 0 3 0 1 2\n\
111 10.0 20.0\n\
111 10.1 20.1\n\
1 200 0 0 BBBB Second Field\n\
100 30.00 1 0 0.25 0 1 0 01 1 2 0 0 3 0 0 0 19 1 2.1 0 0 3 0 0 0\n\
99\n";

    #[test]
    fn finds_airport_and_stops_at_next_header() {
        let a = find_airport(SAMPLE.as_bytes(), "AAAA").unwrap().unwrap();
        assert_eq!((a.kind, a.elevation_ft, a.line), (1, 100.0, 4));
        assert_eq!(a.name, "First Field");
        assert_eq!(a.metadata_value("city"), Some("Nowhere"));
        assert_eq!(a.runways.len(), 1);
        let r = &a.runways[0];
        assert_eq!((r.width_m, r.surface, r.smoothness), (30.0, 1, 0.25));
        assert_eq!(r.ends[0].name, "09");
        assert_eq!(r.ends[1].displaced_threshold_m, 12.5);
        assert_eq!(r.ends[1].reil, 2);
        assert!(r.ends[1].touchdown_lights);
        assert_eq!(a.other_rows.get(&111), Some(&2));
    }

    #[test]
    fn parses_pavement_loops_with_bezier_nodes_and_holes() {
        let text = "I\n1200 test\n\n1 0 0 0 PPPP Pave\n110 1 0.25 0 taxi\n111 10.0 20.0\n112 10.0 20.1 10.0 20.15\n113 10.1 20.1\n111 10.02 20.02\n111 10.02 20.05\n113 10.05 20.05\n110 2 0.25 0 apron\n111 11.0 21.0\n111 11.0 21.1\n114 11.1 21.1 11.15 21.1\n120 line\n111 12.0 22.0\n99\n";
        let a = find_airport(text.as_bytes(), "PPPP").unwrap().unwrap();
        assert_eq!(a.pavements.len(), 2);
        let p = &a.pavements[0];
        assert_eq!((p.surface, p.loops.len()), (1, 2));
        assert_eq!(p.loops[0].len(), 3);
        assert_eq!(p.loops[0][1].ctrl, Some((10.0, 20.15)));
        assert_eq!(p.loops[1].len(), 3);
        assert_eq!(a.pavements[1].loops[0][2].ctrl, Some((11.15, 21.1)));
        // nodes of the 120 linear feature are not interpreted
        assert_eq!(a.other_rows.get(&111), Some(&1));
    }

    #[test]
    fn short_pavement_node_is_an_error() {
        let text = "I\n1200 test\n1 0 0 0 PPPP Pave\n110 1 0 0 t\n112 10.0 20.0\n99\n";
        assert!(find_airport(text.as_bytes(), "PPPP").is_err());
    }

    #[test]
    fn last_airport_ends_at_terminator_and_missing_is_none() {
        let b = find_airport(SAMPLE.as_bytes(), "BBBB").unwrap().unwrap();
        assert_eq!(b.runways.len(), 1);
        assert!(find_airport(SAMPLE.as_bytes(), "ZZZZ").unwrap().is_none());
    }

    #[test]
    fn rejects_bad_version_and_short_runway() {
        assert!(find_airport("I\n1100 x\n".as_bytes(), "A").is_err());
        let short = "I\n1200 x\n1 0 0 0 AAAA N\n100 30 1 0 0 0 1 0 09 1 2\n";
        assert!(find_airport(short.as_bytes(), "AAAA").is_err());
    }
}

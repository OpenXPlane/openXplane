//! Streaming apt.dat (format 1200) reader for one airport at a time.
//! Only airport headers (1/16/17), land runways (100) and metadata (1302) are typed;
//! every other row code is counted, not interpreted.
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

#[derive(Debug, PartialEq)]
pub struct Airport {
    /// Header row code: 1 land airport, 16 seaplane base, 17 heliport.
    pub kind: u32,
    pub elevation_ft: f32,
    pub id: String,
    pub name: String,
    pub metadata: Vec<(String, String)>,
    pub runways: Vec<Runway>,
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

/// Reads only until the end of the requested airport block.
/// Returns `Ok(None)` if no header with this id exists.
pub fn find_airport<R: BufRead>(mut reader: R, id: &str) -> Result<Option<Airport>, String> {
    let mut buf = String::new();
    let mut line_no = 0usize;
    let mut current: Option<Airport> = None;
    let mut seen_version = false;
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
        match code {
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

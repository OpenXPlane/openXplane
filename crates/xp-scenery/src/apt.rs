//! Streaming apt.dat (formats 1000/1050/1100/1130/1200) reader for one airport at a time.
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

/// Row 1300: a start location (gate, ramp tie-down, hangar or runway start).
#[derive(Debug, Clone, PartialEq)]
pub struct StartLocation {
    pub lat: f64,
    pub lon: f64,
    pub heading_deg: f32,
    /// `gate`, `hangar`, `misc` or `tie-down`.
    pub location_type: String,
    /// Allowed aircraft classes (`heavy|jets|turboprops|props|helos`).
    pub aircraft_classes: Vec<String>,
    pub name: String,
    pub line: usize,
}

/// Row 1201: a taxi network node.
#[derive(Debug, Clone, PartialEq)]
pub struct TaxiNode {
    pub id: u32,
    pub lat: f64,
    pub lon: f64,
    /// `init`, `end`, `both` or `junction` (documented usage values, kept verbatim).
    pub usage: String,
    pub name: String,
    pub line: usize,
}

/// Row 1202: a taxi network edge between two node ids.
#[derive(Debug, Clone, PartialEq)]
pub struct TaxiEdge {
    pub from: u32,
    pub to: u32,
    pub two_way: bool,
    /// `taxiway` or `runway`, or `taxiway_<width code>` kept verbatim.
    pub kind: String,
    pub name: String,
    pub line: usize,
}

/// Row 20: a taxiway sign. `text` keeps the `{@...}` style codes verbatim.
#[derive(Debug, Clone, PartialEq)]
pub struct Sign {
    pub lat: f64,
    pub lon: f64,
    pub heading_deg: f32,
    pub size: u32,
    pub text: String,
    pub line: usize,
}

/// Row 21: a visual approach slope indicator (VASI/PAPI family, light type kept as the file code).
#[derive(Debug, Clone, PartialEq)]
pub struct ApproachLight {
    pub lat: f64,
    pub lon: f64,
    pub light_type: u32,
    pub heading_deg: f32,
    pub glidepath_deg: f32,
    pub runway: String,
    pub line: usize,
}

/// Rows 50-56 (1050-1056 in newer files): an airport radio frequency.
#[derive(Debug, Clone, PartialEq)]
pub struct Frequency {
    /// Row code as written (50 recorded, 51 ATIS, 52 unicom, 53 clearance, 54 tower, 55 approach, 56 departure
    /// in the documented numbering; kept as the file's code).
    pub code: u32,
    /// Frequency in units of 10 kHz as in the file (`12780` = 127.80 MHz).
    pub value: u32,
    pub name: String,
    pub line: usize,
}

/// One node of a linear feature with its trailing attribute fields (line type, light type) verbatim.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearNode {
    pub node: Node,
    pub attributes: Vec<String>,
}

/// Row 120: a linear feature (painted line or lights) and its node rows. The feature ends at the
/// first end node (115/116) or closing node (113/114).
#[derive(Debug, Clone, PartialEq)]
pub struct LinearFeature {
    pub name: String,
    pub nodes: Vec<LinearNode>,
    pub closed: bool,
    pub line: usize,
}

/// Row 1204: a taxi-network active zone (runway hold / ILS critical area) and the runway ends it covers.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveZone {
    /// `arrival`, `departure` or `ils` (kept verbatim).
    pub kind: String,
    pub runways: Vec<String>,
    pub line: usize,
}

/// Row 19: a windsock.
#[derive(Debug, Clone, PartialEq)]
pub struct Windsock {
    pub lat: f64,
    pub lon: f64,
    pub lit: bool,
    pub name: String,
    pub line: usize,
}

/// Row 14: the tower viewpoint.
#[derive(Debug, Clone, PartialEq)]
pub struct Tower {
    pub lat: f64,
    pub lon: f64,
    pub height_ft: f32,
    pub name: String,
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
    pub start_locations: Vec<StartLocation>,
    pub windsocks: Vec<Windsock>,
    pub active_zones: Vec<ActiveZone>,
    pub frequencies: Vec<Frequency>,
    pub linear_features: Vec<LinearFeature>,
    pub signs: Vec<Sign>,
    pub approach_lights: Vec<ApproachLight>,
    pub taxi_nodes: Vec<TaxiNode>,
    pub taxi_edges: Vec<TaxiEdge>,
    pub towers: Vec<Tower>,
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
        start_locations: Vec::new(),
        windsocks: Vec::new(),
        active_zones: Vec::new(),
        frequencies: Vec::new(),
        linear_features: Vec::new(),
        signs: Vec::new(),
        approach_lights: Vec::new(),
        taxi_nodes: Vec::new(),
        taxi_edges: Vec::new(),
        towers: Vec::new(),
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
    let mut in_linear = false;
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
            // These documented revisions keep the runway/pavement/header fields
            // typed here in the same positions. Other rows remain uninterpreted.
            if !matches!(fields[0], "1000" | "1050" | "1100" | "1130" | "1200") {
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
        if matches!(code, 111..=116) && in_linear {
            let node = parse_node(code, &fields, line_no)?;
            let used = if matches!(code, 112 | 114 | 116) {
                5
            } else {
                3
            };
            let feature = airport.linear_features.last_mut().unwrap();
            feature.nodes.push(LinearNode {
                node,
                attributes: fields[used..].iter().map(|f| f.to_string()).collect(),
            });
            if matches!(code, 113..=116) {
                feature.closed = matches!(code, 113 | 114);
                in_linear = false;
            }
            continue;
        }
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
        in_linear = code == 120;
        match code {
            1204 if fields.len() >= 3 => airport.active_zones.push(ActiveZone {
                kind: fields[1].to_string(),
                runways: fields[2].split(',').map(str::to_string).collect(),
                line: line_no,
            }),
            120 => airport.linear_features.push(LinearFeature {
                name: fields[1..].join(" "),
                nodes: Vec::new(),
                closed: false,
                line: line_no,
            }),
            50..=56 | 1050..=1056 if fields.len() >= 2 => airport.frequencies.push(Frequency {
                code,
                value: num(fields[1], "frequency", line_no)?,
                name: fields[2..].join(" "),
                line: line_no,
            }),
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
            1300 if fields.len() >= 6 => airport.start_locations.push(StartLocation {
                lat: num(fields[1], "start latitude", line_no)?,
                lon: num(fields[2], "start longitude", line_no)?,
                heading_deg: num(fields[3], "start heading", line_no)?,
                location_type: fields[4].to_string(),
                aircraft_classes: fields[5].split('|').map(str::to_string).collect(),
                name: fields[6..].join(" "),
                line: line_no,
            }),
            1201 if fields.len() >= 5 => airport.taxi_nodes.push(TaxiNode {
                // 1201 lat lon usage id [name]; the id is node-table position by convention.
                id: num(fields[4], "taxi node id", line_no)?,
                lat: num(fields[1], "taxi node latitude", line_no)?,
                lon: num(fields[2], "taxi node longitude", line_no)?,
                usage: fields[3].to_string(),
                name: fields[5..].join(" "),
                line: line_no,
            }),
            1202 if fields.len() >= 5 => airport.taxi_edges.push(TaxiEdge {
                from: num(fields[1], "taxi edge start", line_no)?,
                to: num(fields[2], "taxi edge end", line_no)?,
                two_way: match fields[3] {
                    "twoway" => true,
                    "oneway" => false,
                    other => {
                        return Err(format!(
                            "invalid taxi edge direction '{other}' at line {line_no}"
                        ));
                    }
                },
                kind: fields[4].to_string(),
                name: fields[5..].join(" "),
                line: line_no,
            }),
            20 if fields.len() >= 7 => airport.signs.push(Sign {
                // 20 lat lon heading reserved size text
                lat: num(fields[1], "sign latitude", line_no)?,
                lon: num(fields[2], "sign longitude", line_no)?,
                heading_deg: num(fields[3], "sign heading", line_no)?,
                size: num(fields[5], "sign size", line_no)?,
                text: fields[6..].join(" "),
                line: line_no,
            }),
            21 if fields.len() >= 7 => airport.approach_lights.push(ApproachLight {
                // 21 lat lon type heading glidepath runway [name]
                lat: num(fields[1], "approach light latitude", line_no)?,
                lon: num(fields[2], "approach light longitude", line_no)?,
                light_type: num(fields[3], "approach light type", line_no)?,
                heading_deg: num(fields[4], "approach light heading", line_no)?,
                glidepath_deg: num(fields[5], "approach light glidepath", line_no)?,
                runway: fields[6].to_string(),
                line: line_no,
            }),
            19 if fields.len() >= 4 => airport.windsocks.push(Windsock {
                lat: num(fields[1], "windsock latitude", line_no)?,
                lon: num(fields[2], "windsock longitude", line_no)?,
                lit: fields[3] != "0",
                name: fields[4..].join(" "),
                line: line_no,
            }),
            14 if fields.len() >= 4 => airport.towers.push(Tower {
                lat: num(fields[1], "tower latitude", line_no)?,
                lon: num(fields[2], "tower longitude", line_no)?,
                height_ft: num(fields[3], "tower height", line_no)?,
                name: fields.get(5..).map(|f| f.join(" ")).unwrap_or_default(),
                line: line_no,
            }),
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

    #[test]
    fn active_zones_list_runway_ends() {
        let text = "A
1200 t

1 0 0 0 TEST Test Field
1204 departure 07L,07R,25L,25R
1204 ils 25R
99
";
        let a = find_airport(text.as_bytes(), "TEST").unwrap().unwrap();
        assert_eq!(a.active_zones.len(), 2);
        assert_eq!(a.active_zones[0].kind, "departure");
        assert_eq!(a.active_zones[0].runways, ["07L", "07R", "25L", "25R"]);
        assert_eq!(a.active_zones[1].runways, ["25R"]);
    }

    #[test]
    fn frequencies_and_linear_features_are_typed() {
        let text = "A
1200 t

1 0 0 0 TEST Test Field
54 11950 TWR
50 12780 TEST ATIS
120 Default hold RWY 07L (near)
111 47.0 -122.0 1 100
112 47.1 -122.1 47.05 -122.05 1 100
115 47.2 -122.2 1
120 Loop
111 47.0 -122.0
113 47.1 -122.1 3
99
";
        let a = find_airport(text.as_bytes(), "TEST").unwrap().unwrap();
        assert_eq!(a.frequencies.len(), 2);
        assert!(a.active_zones.is_empty());
        assert_eq!((a.frequencies[0].code, a.frequencies[0].value), (54, 11950));
        assert_eq!(a.frequencies[1].name, "TEST ATIS");
        assert_eq!(a.linear_features.len(), 2);
        let hold = &a.linear_features[0];
        assert_eq!(hold.name, "Default hold RWY 07L (near)");
        assert_eq!(hold.nodes.len(), 3);
        assert_eq!(hold.nodes[0].attributes, ["1", "100"]);
        assert_eq!(hold.nodes[1].node.ctrl, Some((47.05, -122.05)));
        assert_eq!(hold.nodes[1].attributes, ["1", "100"]);
        assert_eq!(hold.nodes[2].attributes, ["1"]);
        assert!(!hold.closed && a.linear_features[1].closed);
        assert!(a.other_rows.is_empty(), "{:?}", a.other_rows);
    }

    #[test]
    fn signs_and_approach_lights_are_typed() {
        let text = "A
1200 t

1 0 0 0 TEST Test Field
20 47.0 -122.0 332.27 0 3 {@R}25L{@L}A
21 47.1 -122.1 2 56.813 3.0 25R PAPI-4L
99
";
        let a = find_airport(text.as_bytes(), "TEST").unwrap().unwrap();
        assert_eq!(a.signs.len(), 1);
        assert_eq!(
            (a.signs[0].size, a.signs[0].text.as_str()),
            (3, "{@R}25L{@L}A")
        );
        assert_eq!(a.approach_lights.len(), 1);
        let p = &a.approach_lights[0];
        assert_eq!((p.light_type, p.runway.as_str()), (2, "25R"));
        assert_eq!((p.heading_deg, p.glidepath_deg), (56.813, 3.0));
        assert!(a.other_rows.is_empty(), "{:?}", a.other_rows);
    }

    #[test]
    fn taxi_nodes_and_edges_are_typed() {
        let text = "A
1200 t

1 0 0 0 TEST Test Field
1200
1201 47.0 -122.0 both 0 C_stop
1201 47.1 -122.1 junction 1
1202 0 1 twoway taxiway C
1202 1 0 oneway runway
99
";
        let a = find_airport(text.as_bytes(), "TEST").unwrap().unwrap();
        assert_eq!(a.taxi_nodes.len(), 2);
        assert_eq!(
            (
                a.taxi_nodes[0].usage.as_str(),
                a.taxi_nodes[0].name.as_str()
            ),
            ("both", "C_stop")
        );
        assert_eq!(a.taxi_nodes[1].name, "");
        assert_eq!(a.taxi_edges.len(), 2);
        assert!(a.taxi_edges[0].two_way && !a.taxi_edges[1].two_way);
        assert_eq!((a.taxi_edges[1].from, a.taxi_edges[1].to), (1, 0));
        assert_eq!(a.taxi_edges[1].kind, "runway");
        assert!(
            find_airport(
                "A
1200 t

1 0 0 0 T x
1202 0 1 sideways taxiway
"
                .as_bytes(),
                "T"
            )
            .is_err()
        );
    }

    #[test]
    fn start_locations_windsocks_and_towers_are_typed() {
        let text = "A
1200 t

1 0 0 0 TEST Test Field
14 47.01 -122.01 60.0 0 Main Tower
19 47.02 -122.02 1 WS
1300 47.03 -122.03 148.49 gate heavy|jets|props 13
1300 47.04 -122.04 90.0 tie-down props Ramp A
99
";
        let a = find_airport(text.as_bytes(), "TEST").unwrap().unwrap();
        assert_eq!(a.towers.len(), 1);
        assert_eq!(
            (a.towers[0].height_ft, a.towers[0].name.as_str()),
            (60.0, "Main Tower")
        );
        assert_eq!(a.windsocks.len(), 1);
        assert!(a.windsocks[0].lit);
        assert_eq!(a.start_locations.len(), 2);
        let gate = &a.start_locations[0];
        assert_eq!(gate.location_type, "gate");
        assert_eq!(gate.aircraft_classes, ["heavy", "jets", "props"]);
        assert_eq!((gate.heading_deg, gate.name.as_str()), (148.49, "13"));
        assert_eq!(a.start_locations[1].name, "Ramp A");
        assert!(a.other_rows.is_empty(), "{:?}", a.other_rows);
    }

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
        // nodes of the 120 linear feature belong to that feature, not to pavements
        assert_eq!(a.other_rows.get(&111), None);
        assert_eq!(a.linear_features.len(), 1);
        assert_eq!(a.linear_features[0].nodes.len(), 1);
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
        assert!(find_airport("I\n9999 x\n".as_bytes(), "A").is_err());
        let short = "I\n1200 x\n1 0 0 0 AAAA N\n100 30 1 0 0 0 1 0 09 1 2\n";
        assert!(find_airport(short.as_bytes(), "AAAA").is_err());
    }

    #[test]
    fn prior_versions_preserve_supported_airport_fields() {
        let expected = find_airport(SAMPLE.as_bytes(), "AAAA").unwrap();
        for version in ["1000", "1050", "1100", "1130"] {
            let text = SAMPLE.replacen("1200", version, 1);
            assert_eq!(find_airport(text.as_bytes(), "AAAA").unwrap(), expected);
            assert!(find_airport(text.as_bytes(), "ZZZZ").unwrap().is_none());
        }
    }
}

#[cfg(test)]
mod installed {
    use super::*;

    #[test]
    #[ignore = "requires read-only installed UHWW scenery via OPENXPLANE_REFERENCE_ROOT"]
    fn installed_uhww_start_locations_and_windsock_parse() {
        let root = std::env::var_os("OPENXPLANE_REFERENCE_ROOT").expect("reference root required");
        let path = std::path::Path::new(&root).join("Custom Scenery/1_UHWW/Earth nav data/apt.dat");
        let file = std::io::BufReader::new(std::fs::File::open(path).unwrap());
        let a = find_airport(file, "UHWW").unwrap().unwrap();
        assert_eq!(a.start_locations.len(), 39);
        assert_eq!(a.windsocks.len(), 1);
        assert_eq!((a.taxi_nodes.len(), a.taxi_edges.len()), (26, 28));
        assert_eq!((a.signs.len(), a.approach_lights.len()), (14, 2));
        assert_eq!((a.frequencies.len(), a.linear_features.len()), (4, 11));
        assert_eq!(a.active_zones.len(), 60);
        assert!(
            a.other_rows.keys().all(|c| *c == 1200),
            "{:?}",
            a.other_rows
        );
        println!("remaining other rows: {:?}", a.other_rows);
        assert!(
            a.taxi_edges
                .iter()
                .all(|e| a.taxi_nodes.iter().any(|n| n.id == e.from)
                    && a.taxi_nodes.iter().any(|n| n.id == e.to))
        );
        assert!(!a.other_rows.contains_key(&1300) && !a.other_rows.contains_key(&19));
        assert!(
            a.start_locations
                .iter()
                .all(|s| !s.aircraft_classes.is_empty())
        );
    }
}

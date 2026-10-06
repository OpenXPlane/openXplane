//! AFL 1110 structure confirmed in the reference EXE; see research/AIRFOILS.md.
//! Sampling is a diagnostic linear interpolation, not recovered flight physics.

pub const POLAR_ROWS: usize = 721;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coefficients {
    pub cl: f32,
    pub cd: f32,
    pub cm: f32,
}

#[derive(Debug)]
pub struct PolarRow {
    pub alpha_deg: f32,
    pub coefficients: Coefficients,
}

#[derive(Debug)]
pub struct Polar {
    /// Preserve all 25 parameters. Units and evaluation rules are not inferred.
    pub parameters: [f32; 25],
    pub rows: Vec<PolarRow>,
}

#[derive(Debug)]
pub struct Airfoil {
    pub version: u32,
    /// The two header scalars and fourteen coordinate pairs, in source order.
    pub header_scalars: [f32; 2],
    pub shape_points: [[f32; 2]; 14],
    pub polars: Vec<Polar>,
}

struct Reader<'a> {
    lines: std::str::Lines<'a>,
    line: usize,
}

impl<'a> Reader<'a> {
    fn next(&mut self) -> Result<&'a str, String> {
        self.line += 1;
        self.lines
            .next()
            .map(str::trim)
            .ok_or_else(|| format!("truncated AFL at line {}", self.line))
    }

    fn expect(&mut self, expected: &str) -> Result<(), String> {
        if self.next()? != expected {
            return Err(format!("expected {expected:?} at AFL line {}", self.line));
        }
        Ok(())
    }

    fn numbers<const N: usize>(&mut self) -> Result<[f32; N], String> {
        let mut values = self.next()?.split_whitespace();
        let mut result = [0.0; N];
        for value in &mut result {
            *value = values
                .next()
                .and_then(|s| s.parse::<f32>().ok())
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("expected {N} finite numbers at AFL line {}", self.line))?;
        }
        if values.next().is_some() {
            return Err(format!("extra numbers at AFL line {}", self.line));
        }
        Ok(result)
    }
}

impl Airfoil {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut reader = Reader {
            lines: text.trim_start_matches('\u{feff}').lines(),
            line: 0,
        };
        if !matches!(reader.next()?, "I" | "A") {
            return Err("unsupported AFL marker".into());
        }
        reader.expect("1110 Version")?;
        reader.expect("1234 device type code")?;
        let header_scalars = [reader.numbers::<1>()?[0], reader.numbers::<1>()?[0]];
        let mut shape_points = [[0.0; 2]; 14];
        for point in &mut shape_points {
            *point = reader.numbers()?;
        }
        let count = reader
            .next()?
            .parse::<usize>()
            .map_err(|_| "invalid AFL polar count")?;
        // Bound allocation by actual remaining input, rather than a guessed format limit.
        if count == 0 || count > reader.lines.clone().count() / (POLAR_ROWS + 2) {
            return Err("invalid or truncated AFL polar count".into());
        }
        let mut polars = Vec::with_capacity(count);
        for _ in 0..count {
            let parameters = reader.numbers()?;
            reader.expect("alpha cl cd cm:")?;
            let mut rows: Vec<PolarRow> = Vec::with_capacity(POLAR_ROWS);
            for _ in 0..POLAR_ROWS {
                let [alpha_deg, cl, cd, cm] = reader.numbers()?;
                if rows.last().is_some_and(|r| alpha_deg <= r.alpha_deg) {
                    return Err(format!("unordered alpha at AFL line {}", reader.line));
                }
                rows.push(PolarRow {
                    alpha_deg,
                    coefficients: Coefficients { cl, cd, cm },
                });
            }
            polars.push(Polar { parameters, rows });
        }
        if reader.lines.any(|line| !line.trim().is_empty()) {
            return Err("unexpected data after AFL polars".into());
        }
        Ok(Self {
            version: 1110,
            header_scalars,
            shape_points,
            polars,
        })
    }
}

impl Polar {
    /// Inspect one table without blending Reynolds regimes or extrapolating.
    /// Uses the file's alpha column, including its nonuniform spacing.
    pub fn sample_linear(&self, alpha_deg: f32) -> Result<Coefficients, String> {
        let first = self.rows.first().ok_or("empty polar")?;
        let last = self.rows.last().ok_or("empty polar")?;
        if !alpha_deg.is_finite() || alpha_deg < first.alpha_deg || alpha_deg > last.alpha_deg {
            return Err("alpha outside polar range".into());
        }
        let right = self.rows.partition_point(|r| r.alpha_deg < alpha_deg);
        let b = &self.rows[right];
        if b.alpha_deg == alpha_deg {
            return Ok(b.coefficients);
        }
        let a = &self.rows[right - 1];
        let t = (alpha_deg - a.alpha_deg) / (b.alpha_deg - a.alpha_deg);
        let mix = |x: f32, y: f32| x + (y - x) * t;
        Ok(Coefficients {
            cl: mix(a.coefficients.cl, b.coefficients.cl),
            cd: mix(a.coefficients.cd, b.coefficients.cd),
            cm: mix(a.coefficients.cm, b.coefficients.cm),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        let mut s = String::from("I\n1110 Version\n1234 device type code\n0.12\n0.73\n");
        for _ in 0..14 {
            s.push_str("0 0\n");
        }
        s.push_str("2\n");
        for parameter in [1, 2] {
            s.push_str(&format!(
                "{parameter} {}\nalpha cl cd cm:\n",
                vec!["0"; 24].join(" ")
            ));
            for i in 0..POLAR_ROWS {
                let alpha = match i {
                    0..=159 => i as f32 - 180.0,
                    160..=560 => (i as f32 - 360.0) / 10.0,
                    _ => i as f32 - 540.0,
                };
                // Independent synthetic coefficients; no original content embedded.
                s.push_str(&format!(
                    "{alpha} {} 0.02 -0.05\n",
                    alpha * parameter as f32
                ));
            }
        }
        s
    }

    #[test]
    fn retains_multiple_polars_and_samples_nonuniform_grid() {
        let a = Airfoil::parse(&fixture().replace('\n', "\r\n")).unwrap();
        assert_eq!(a.polars.len(), 2);
        assert_eq!(a.polars[1].parameters[0], 2.0);
        for alpha in [-180.0, -20.5, -0.05, 0.0, 20.5, 180.0] {
            let c = a.polars[1].sample_linear(alpha).unwrap();
            assert!((c.cl - alpha * 2.0).abs() < 0.00001);
            assert_eq!(c.cd, 0.02);
        }
        for alpha in [f32::NAN, f32::INFINITY, -181.0, 181.0] {
            assert!(a.polars[0].sample_linear(alpha).is_err());
        }
    }

    #[test]
    fn rejects_truncation_bad_numbers_and_unordered_angles() {
        let s = fixture();
        for broken in [
            s[..s.len() / 2].to_string(),
            s.replace("-180 -180", "NaN -180"),
            s.replace("-179 -179", "-180 -179"),
            s.replace("1110 Version", "900 Version"),
            format!("{s}extra\n"),
            s.replacen("\n2\n", "\n999999999\n", 1),
        ] {
            assert!(Airfoil::parse(&broken).is_err());
        }
    }
}

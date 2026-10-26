//! TER base-material subset. Projection phase and advanced shader behaviour are
//! preview policies, not native-renderer parity.
#[derive(Debug)]
pub struct Material {
    pub base: String,
    pub repeat: bool,
    pub no_alpha: bool,
    pub projected: Option<[f32; 2]>,
    pub offset: [f32; 2],
    pub angle: [f32; 4],
    pub unsupported: std::collections::BTreeSet<String>,
}

impl Material {
    pub fn parse(text: &str) -> Result<Self, String> {
        let lines = text
            .lines()
            .map(|l| l.split('#').next().unwrap_or("").trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>();
        if lines.len() < 3
            || !["A", "I"].contains(&lines[0])
            || lines[1] != "800"
            || lines[2] != "TERRAIN"
        {
            return Err("expected TER version800 header".into());
        }
        let mut out = Self {
            base: String::new(),
            repeat: true,
            no_alpha: false,
            projected: None,
            offset: [0.0; 2],
            angle: [0.0, 1.0, 0.0, 0.0],
            unsupported: Default::default(),
        };
        for line in &lines[3..] {
            let mut fields = line.split_whitespace();
            let command = fields.next().unwrap();
            let values = fields.collect::<Vec<_>>();
            let number = |i: usize| -> Result<f32, String> {
                let n = values
                    .get(i)
                    .ok_or("missing TER value")?
                    .parse::<f32>()
                    .map_err(|_| "invalid TER number")?;
                if !n.is_finite() {
                    return Err("nonfinite TER value".into());
                }
                Ok(n)
            };
            match command {
                "BASE_TEX" | "BASE_TEX_NOWRAP" => {
                    out.base = values.join(" ").replace(['\\', ':'], "/");
                    out.repeat = command == "BASE_TEX";
                }
                "NO_ALPHA" => out.no_alpha = true,
                "PROJECTED" => {
                    let s = [number(0)?, number(1)?];
                    if s.iter().any(|v| *v <= 0.0) {
                        return Err("invalid TER projection scale".into());
                    }
                    out.projected = Some(s);
                }
                "PROJECT_OFFSET" => out.offset = [number(0)?, number(1)?],
                "PROJECT_ANGLE" => out.angle = [number(0)?, number(1)?, number(2)?, number(3)?],
                _ => {
                    out.unsupported.insert(command.into());
                }
            }
        }
        if out.base.is_empty() {
            return Err("TER missing base texture".into());
        }
        Ok(out)
    }

    /// Explicit DSF ST or vertical local-meter projection. General tilted
    /// projection and the simulator's geographic anchoring remain unsupported.
    pub fn uv(&self, local_xz: [f32; 2], point: &[f64]) -> Result<[f32; 2], String> {
        if let Some(scale) = self.projected {
            if self.angle != [0.0, 1.0, 0.0, 0.0] {
                return Err("nonvertical TER projection unsupported".into());
            }
            Ok([
                (local_xz[0] - self.offset[0]) / scale[0],
                (-local_xz[1] - self.offset[1]) / scale[1],
            ])
        } else {
            let s = *point.get(5).ok_or("TER requires DSF base ST planes")?;
            let t = *point.get(6).ok_or("TER requires DSF base ST planes")?;
            if !(s as f32).is_finite() || !(t as f32).is_finite() {
                return Err("invalid TER ST".into());
            }
            Ok([s as f32, t as f32])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn base_mapping_preserves_explicit_st_and_projected_scale() {
        let m=Material::parse("I\n800\nTERRAIN\nBASE_TEX_NOWRAP textures:base.png\nNO_ALPHA\nBORDER_TEX ignored.png\n").unwrap();
        assert_eq!(m.base, "textures/base.png");
        assert!(!m.repeat);
        assert!(m.no_alpha);
        assert!(m.unsupported.contains("BORDER_TEX"));
        assert_eq!(
            m.uv([9.0, 9.0], &[0.0, 0.0, 0.0, 0.0, 0.0, 0.25, 0.75])
                .unwrap(),
            [0.25, 0.75]
        );
        assert!(m.uv([0.0; 2], &[0.0; 5]).is_err());
        let text = "A\n800\nTERRAIN\nBASE_TEX base.png\nPROJECTED 100 200\nPROJECT_OFFSET 10 20\n";
        let m = Material::parse(text).unwrap();
        assert_eq!(m.uv([110.0, -220.0], &[]).unwrap(), [1.0, 1.0]);
        assert!(Material::parse(&text.replace("100 200", "0 200")).is_err());
        assert!(Material::parse(&text.replace("10 20", "NaN 20")).is_err());
        let m = Material::parse(&(text.to_owned() + "PROJECT_ANGLE 1 0 0 0\n")).unwrap();
        assert!(m.uv([0.0; 2], &[]).is_err());
    }
}

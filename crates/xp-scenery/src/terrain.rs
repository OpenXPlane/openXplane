//! TER base-material subset. Projection phase and advanced shader behaviour are
//! preview policies, not native-renderer parity.
#[derive(Debug)]
pub struct Material {
    pub base: String,
    pub repeat: bool,
    pub no_alpha: bool,
    pub border: Option<String>,
    pub border_repeat: bool,
    pub composite_borders: bool,
    pub projected: Option<[f32; 2]>,
    pub offset: [f32; 2],
    pub angle: [f32; 4],
    pub composite: Composite,
    pub unsupported: std::collections::BTreeSet<String>,
}

/// Serialized inputs only. The renderer does not yet evaluate compositing.
#[derive(Debug, Default)]
pub struct Composite {
    pub texture: Option<String>,
    pub nowrap: bool,
    pub noise: Option<String>,
    pub projected: Option<[f32; 2]>,
    /// Six required values and the optional seventh (zero when absent).
    pub params: Option<[f32; 7]>,
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
            border: None,
            border_repeat: false,
            composite_borders: false,
            projected: None,
            offset: [0.0; 2],
            angle: [0.0, 1.0, 0.0, 0.0],
            composite: Composite::default(),
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
                "BORDER_TEX" | "BORDER_TEX_WRAP" => {
                    let name = values.join(" ").replace(['\\', ':'], "/");
                    if name.is_empty() {
                        return Err("empty TER border path".into());
                    }
                    out.border = Some(name);
                    out.border_repeat = command == "BORDER_TEX_WRAP";
                }
                "COMPOSITE_BORDERS" => out.composite_borders = true,
                "PROJECTED" => {
                    let s = [number(0)?, number(1)?];
                    if s.iter().any(|v| *v <= 0.0) {
                        return Err("invalid TER projection scale".into());
                    }
                    out.projected = Some(s);
                }
                "PROJECT_OFFSET" => out.offset = [number(0)?, number(1)?],
                "PROJECT_ANGLE" => out.angle = [number(0)?, number(1)?, number(2)?, number(3)?],
                "COMPOSITE_TEX" | "COMPOSITE_TEX_NOWRAP" | "COMPOSITE_NOISE" => {
                    let path = values.join(" ").replace(['\\', ':'], "/");
                    if path.is_empty() {
                        return Err("empty TER composite path".into());
                    }
                    if command == "COMPOSITE_NOISE" {
                        out.composite.noise = Some(path);
                    } else {
                        out.composite.texture = Some(path);
                        out.composite.nowrap = command == "COMPOSITE_TEX_NOWRAP";
                    }
                    // Preserve the runtime warning until these inputs reach the shader.
                    out.unsupported.insert(command.into());
                }
                "COMPOSITE_PROJECTED" => {
                    if values.len() != 2 {
                        return Err("TER composite projection requires two values".into());
                    }
                    out.composite.projected = Some([number(0)?, number(1)?]);
                    out.unsupported.insert(command.into());
                }
                "COMPOSITE_PARAMS" => {
                    if !(6..=7).contains(&values.len()) {
                        return Err("TER composite parameters require six or seven values".into());
                    }
                    let mut params = [0.0; 7];
                    for (i, value) in params.iter_mut().enumerate().take(values.len()) {
                        *value = number(i)?;
                    }
                    out.composite.params = Some(params);
                    out.unsupported.insert(command.into());
                }
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

    pub fn border_uv(&self, point: &[f64]) -> Result<[f32; 2], String> {
        let index = if self.projected.is_some() { 5 } else { 7 };
        let s = *point.get(index).ok_or("missing TER border S")? as f32;
        let t = *point.get(index + 1).ok_or("missing TER border T")? as f32;
        if !s.is_finite() || !t.is_finite() {
            return Err("invalid TER border ST".into());
        }
        Ok([s, t])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a read-only installed reference via OPENXPLANE_REFERENCE_ROOT"]
    fn installed_composite_metadata_and_resources() {
        let root = std::env::var_os("OPENXPLANE_REFERENCE_ROOT").expect("reference root required");
        let directory = std::path::Path::new(&root)
            .join("Resources/default scenery/1000 world terrain/terrain10");
        let mut parsed = 0;
        let mut composites = 0;
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|s| s.to_str()) != Some("ter") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let m = Material::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            parsed += 1;
            if let Some(texture) = &m.composite.texture {
                assert!(
                    path.parent().unwrap().join(texture).is_file(),
                    "{}",
                    path.display()
                );
                composites += 1;
            }
            if let Some(noise) = &m.composite.noise {
                assert!(
                    path.parent().unwrap().join(noise).is_file(),
                    "{}",
                    path.display()
                );
            }
        }
        assert!(composites > 0);
        eprintln!("Read {parsed} installed TER files; {composites} declare composite textures");
    }
    #[test]
    fn composite_inputs_preserve_order_optional_value_and_rendering_warning() {
        let header = "A\n800\nTERRAIN\nBASE_TEX base.png\n";
        let m = Material::parse(
            &(header.to_owned()
                + "COMPOSITE_TEX_NOWRAP a:first.png\nCOMPOSITE_NOISE noise.png\n"
                + "COMPOSITE_PROJECTED 1024 512.5\nCOMPOSITE_PARAMS 1 2 3 4 5 6\n"),
        )
        .unwrap();
        assert_eq!(m.composite.texture.as_deref(), Some("a/first.png"));
        assert!(m.composite.nowrap);
        assert_eq!(m.composite.noise.as_deref(), Some("noise.png"));
        assert_eq!(m.composite.projected, Some([1024.0, 512.5]));
        assert_eq!(
            m.composite.params,
            Some([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 0.0])
        );
        assert!(m.unsupported.contains("COMPOSITE_PARAMS"));
        let m = Material::parse(
            &(header.to_owned()
                + "COMPOSITE_PARAMS 1 2 3 4 5 6 7\nCOMPOSITE_PARAMS 7 6 5 4 3 2\n"
                + "COMPOSITE_TEX_NOWRAP first.png\nCOMPOSITE_TEX last.png\n"),
        )
        .unwrap();
        assert_eq!(
            m.composite.params,
            Some([7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 0.0])
        );
        assert!(!m.composite.nowrap);
        assert_eq!(m.composite.texture.as_deref(), Some("last.png"));
        for directive in [
            "COMPOSITE_PARAMS 1 2 3 4 5",
            "COMPOSITE_PARAMS 1 2 3 4 5 NaN",
            "COMPOSITE_PARAMS 1 2 3 4 5 6 7 8",
            "COMPOSITE_PROJECTED 1",
            "COMPOSITE_PROJECTED inf 2",
            "COMPOSITE_NOISE",
            "COMPOSITE_TEX",
        ] {
            assert!(
                Material::parse(&(header.to_owned() + directive)).is_err(),
                "{directive}"
            );
        }
    }
    #[test]
    fn base_mapping_preserves_explicit_st_and_projected_scale() {
        let m=Material::parse("I\n800\nTERRAIN\nBASE_TEX_NOWRAP textures:base.png\nNO_ALPHA\nBORDER_TEX ignored.png\n").unwrap();
        assert_eq!(m.base, "textures/base.png");
        assert!(!m.repeat);
        assert!(m.no_alpha);
        assert_eq!(m.border.as_deref(), Some("ignored.png"));
        assert_eq!(
            m.border_uv(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.1, 0.2, 0.3, 0.4])
                .unwrap(),
            [0.3, 0.4]
        );
        assert_eq!(
            m.uv([9.0, 9.0], &[0.0, 0.0, 0.0, 0.0, 0.0, 0.25, 0.75])
                .unwrap(),
            [0.25, 0.75]
        );
        assert!(m.uv([0.0; 2], &[0.0; 5]).is_err());
        let text = "A\n800\nTERRAIN\nBASE_TEX base.png\nPROJECTED 100 200\nPROJECT_OFFSET 10 20\n";
        let m = Material::parse(text).unwrap();
        assert_eq!(
            m.border_uv(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.25, 0.75]).unwrap(),
            [0.25, 0.75]
        );
        assert!(m.border_uv(&[0.0; 5]).is_err());
        let border =
            Material::parse(&(text.to_owned() + "BORDER_TEX_WRAP border.png\nCOMPOSITE_BORDERS\n"))
                .unwrap();
        assert!(border.border_repeat && border.composite_borders);
        assert_eq!(border.border.as_deref(), Some("border.png"));
        assert!(Material::parse(&(text.to_owned() + "BORDER_TEX\n")).is_err());
        assert_eq!(m.uv([110.0, -220.0], &[]).unwrap(), [1.0, 1.0]);
        assert!(Material::parse(&text.replace("100 200", "0 200")).is_err());
        assert!(Material::parse(&text.replace("10 20", "NaN 20")).is_err());
        let m = Material::parse(&(text.to_owned() + "PROJECT_ANGLE 1 0 0 0\n")).unwrap();
        assert!(m.uv([0.0; 2], &[]).is_err());
    }
}

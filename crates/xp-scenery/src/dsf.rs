//! DSF raster-height subset from the public DSF specification. Not native parity.
//! Reads original files in place; 7z members are decoded only into memory.
use std::{collections::BTreeMap, fs, path::Path};

const LIMIT: usize = 256 * 1024 * 1024;

type Atom<'a> = ([u8; 4], &'a [u8]);

fn atoms(mut data: &[u8]) -> Result<Vec<Atom<'_>>, String> {
    let mut out = Vec::new();
    while !data.is_empty() {
        if data.len() < 8 {
            return Err("truncated DSF atom header".into());
        }
        let tag = data[..4].try_into().unwrap();
        let len = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
        if len < 8 || len > data.len() {
            return Err("invalid DSF atom length".into());
        }
        out.push((tag, &data[8..len]));
        data = &data[len..];
    }
    Ok(out)
}

fn strings(data: &[u8]) -> Result<Vec<String>, String> {
    if data.is_empty() {
        return Ok(Vec::new());
    }
    if data.last() != Some(&0) {
        return Err("unterminated DSF string table".into());
    }
    data[..data.len() - 1]
        .split(|b| *b == 0)
        .map(|s| String::from_utf8(s.to_vec()).map_err(|e| e.to_string()))
        .collect()
}

pub struct HeightRaster {
    pub west: f64,
    pub east: f64,
    pub south: f64,
    pub north: f64,
    pub width: usize,
    pub height: usize,
    pub post: bool,
    values: Vec<f32>,
}

impl HeightRaster {
    pub fn read(path: &Path) -> Result<Self, String> {
        let raw = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let data = if raw.starts_with(b"7z\xbc\xaf\x27\x1c") {
            let mut archive = sevenz_rust2::ArchiveReader::new(
                std::io::Cursor::new(raw),
                sevenz_rust2::Password::empty(),
            )
            .map_err(|e| e.to_string())?;
            let mut payload = None;
            archive
                .for_each_entries(|entry, reader| {
                    if entry.is_directory {
                        return Ok(true);
                    }
                    if payload.is_some() || entry.size > LIMIT as u64 {
                        return Err(sevenz_rust2::Error::Other(
                            "DSF archive must contain one bounded member".into(),
                        ));
                    }
                    let mut bytes = Vec::new();
                    reader.take(LIMIT as u64 + 1).read_to_end(&mut bytes)?;
                    if bytes.len() > LIMIT {
                        return Err(sevenz_rust2::Error::Other("DSF member too large".into()));
                    }
                    payload = Some(bytes);
                    Ok(true)
                })
                .map_err(|e| e.to_string())?;
            payload.ok_or("empty DSF archive")?
        } else {
            raw
        };
        Self::parse(&data)
    }

    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 28
            || data.len() > LIMIT
            || &data[..8] != b"XPLNEDSF"
            || data[8..12] != 1u32.to_le_bytes()
        {
            return Err("unsupported DSF header or size".into());
        }
        // Footer MD5 is retained but not yet verified. 7z CRC is checked by the decoder.
        let mut properties = BTreeMap::new();
        let mut names = Vec::new();
        let mut infos = Vec::new();
        let mut rasters = Vec::new();
        for (tag, body) in atoms(&data[12..data.len() - 16])? {
            if tag == *b"DAEH" {
                for (t, p) in atoms(body)? {
                    if t == *b"PORP" {
                        let values = strings(p)?;
                        if values.len() % 2 != 0 {
                            return Err("odd DSF properties".into());
                        }
                        for pair in values.as_chunks::<2>().0 {
                            properties.insert(pair[0].clone(), pair[1].clone());
                        }
                    }
                }
            } else if tag == *b"NFED" {
                for (t, p) in atoms(body)? {
                    if t == *b"NMED" {
                        names = strings(p)?;
                    }
                }
            } else if tag == *b"SMED" {
                for (t, p) in atoms(body)? {
                    if t == *b"IMED" {
                        infos.push(p);
                    }
                    if t == *b"DMED" {
                        rasters.push(p);
                    }
                }
            }
        }
        let index = names
            .iter()
            .position(|n| n == "elevation")
            .ok_or("DSF has no elevation raster")?;
        let info = *infos.get(index).ok_or("missing raster metadata")?;
        let raw = *rasters.get(index).ok_or("missing raster data")?;
        if info.len() != 20 || info[0] != 1 {
            return Err("unsupported raster metadata".into());
        }
        let size = info[1] as usize;
        let flags = u16::from_le_bytes(info[2..4].try_into().unwrap());
        let width = u32::from_le_bytes(info[4..8].try_into().unwrap()) as usize;
        let height = u32::from_le_bytes(info[8..12].try_into().unwrap()) as usize;
        let scale = f32::from_le_bytes(info[12..16].try_into().unwrap());
        let offset = f32::from_le_bytes(info[16..20].try_into().unwrap());
        let count = width
            .checked_mul(height)
            .filter(|n| *n <= LIMIT / 4)
            .ok_or("raster size overflow")?;
        if width < 2
            || height < 2
            || ![1, 2, 4].contains(&size)
            || count.checked_mul(size) != Some(raw.len())
        {
            return Err("raster dimensions disagree".into());
        }
        let mut values = Vec::with_capacity(count);
        for pixel in raw.chunks_exact(size) {
            let value = match (flags & 3, size) {
                (0, 4) => f32::from_le_bytes(pixel.try_into().unwrap()),
                (1, 1) => pixel[0] as i8 as f32,
                (1, 2) => i16::from_le_bytes(pixel.try_into().unwrap()) as f32,
                (1, 4) => i32::from_le_bytes(pixel.try_into().unwrap()) as f32,
                (2, 1) => pixel[0] as f32,
                (2, 2) => u16::from_le_bytes(pixel.try_into().unwrap()) as f32,
                (2, 4) => u32::from_le_bytes(pixel.try_into().unwrap()) as f32,
                _ => return Err("unsupported raster encoding".into()),
            } * scale
                + offset;
            if !value.is_finite() {
                return Err("non-finite raster height".into());
            }
            values.push(value);
        }
        let bound = |key: &str| -> Result<f64, String> {
            properties
                .get(key)
                .ok_or_else(|| format!("missing {key}"))?
                .parse()
                .map_err(|_| format!("invalid {key}"))
        };
        let result = Self {
            west: bound("sim/west")?,
            east: bound("sim/east")?,
            south: bound("sim/south")?,
            north: bound("sim/north")?,
            width,
            height,
            post: flags & 4 != 0,
            values,
        };
        if ![result.west, result.east, result.south, result.north]
            .iter()
            .all(|v| v.is_finite())
            || result.east <= result.west
            || result.north <= result.south
        {
            return Err("invalid DSF bounds".into());
        }
        Ok(result)
    }

    pub fn sample(&self, lat: f64, lon: f64) -> f32 {
        let coord = |v: f64, lo: f64, hi: f64, n: usize| {
            let unit = ((v - lo) / (hi - lo)).clamp(0.0, 1.0);
            if self.post {
                unit * (n - 1) as f64
            } else {
                (unit * n as f64 - 0.5).clamp(0.0, (n - 1) as f64)
            }
        };
        let x = coord(lon, self.west, self.east, self.width);
        let y = coord(lat, self.south, self.north, self.height);
        let ix = x.floor() as usize;
        let iy = y.floor() as usize;
        let at = |xx, yy| self.values[yy * self.width + xx];
        let tx = (x - ix as f64) as f32;
        let ty = (y - iy as f64) as f32;
        let a = at(ix, iy) * (1.0 - tx) + at((ix + 1).min(self.width - 1), iy) * tx;
        let b = at(ix, (iy + 1).min(self.height - 1)) * (1.0 - tx)
            + at((ix + 1).min(self.width - 1), (iy + 1).min(self.height - 1)) * tx;
        a * (1.0 - ty) + b * ty
    }
}
use std::io::Read;

#[cfg(test)]
mod tests {
    use super::*;
    fn atom(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
        [
            tag.as_slice(),
            &((body.len() + 8) as u32).to_le_bytes(),
            body,
        ]
        .concat()
    }
    fn fixture() -> Vec<u8> {
        let props = b"sim/west\0-123\0sim/east\0-122\0sim/south\x0047\0sim/north\x0048\0";
        let mut info = vec![1, 2];
        info.extend(5u16.to_le_bytes());
        info.extend(2u32.to_le_bytes());
        info.extend(2u32.to_le_bytes());
        info.extend(2f32.to_le_bytes());
        info.extend(10f32.to_le_bytes());
        let pixels = [-10i16, 0, 10, 20]
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect::<Vec<_>>();
        let mut data = b"XPLNEDSF".to_vec();
        data.extend(1u32.to_le_bytes());
        data.extend(atom(b"DAEH", &atom(b"PORP", props)));
        data.extend(atom(b"NFED", &atom(b"NMED", b"elevation\0")));
        data.extend(atom(
            b"SMED",
            &[atom(b"IMED", &info), atom(b"DMED", &pixels)].concat(),
        ));
        data.extend([0; 16]);
        data
    }
    #[test]
    fn signed_scaled_raster_samples_corners_and_center() {
        let r = HeightRaster::parse(&fixture()).unwrap();
        assert_eq!(r.sample(47.0, -123.0), -10.0);
        assert_eq!(r.sample(48.0, -122.0), 50.0);
        assert_eq!(r.sample(47.5, -122.5), 20.0);
    }
    #[test]
    fn rejects_truncated_and_invalid_atoms() {
        let mut d = fixture();
        d.truncate(d.len() - 17);
        assert!(HeightRaster::parse(&d).is_err());
        let mut d = fixture();
        d[16..20].copy_from_slice(&0u32.to_le_bytes());
        assert!(HeightRaster::parse(&d).is_err());
    }
}

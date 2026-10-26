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

/// Placed OBJ instances from a documented DSF command subset. Other primitive
/// commands are decoded for stream alignment but are not instantiated here.
pub struct ObjectPlacement {
    pub resource: String,
    pub lon: f64,
    pub lat: f64,
    pub heading: f32,
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|e| *e <= self.bytes.len())
            .ok_or("truncated DSF payload")?;
        let out = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(out)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
}

/// Polygon contours retain every coordinate plane (including wall/Bezier data).
#[derive(Debug)]
pub struct PolygonPlacement {
    pub resource: String,
    pub parameter: u16,
    pub windings: Vec<Vec<Vec<f64>>>,
}

#[derive(Default)]
pub struct Features {
    pub objects: Vec<ObjectPlacement>,
    pub polygons: Vec<PolygonPlacement>,
}

pub fn object_placements(data: &[u8]) -> Result<Vec<ObjectPlacement>, String> {
    Ok(features(data)?.objects)
}

pub fn features(data: &[u8]) -> Result<Features, String> {
    if data.len() < 28 || &data[..8] != b"XPLNEDSF" || data[8..12] != 1u32.to_le_bytes() {
        return Err("unsupported object DSF header".into());
    }
    let mut definitions = Vec::new();
    let mut polygon_definitions = Vec::new();
    let mut pools = Vec::new();
    let mut scales = Vec::new();
    let mut commands = None;
    for (tag, body) in atoms(&data[12..data.len() - 16])? {
        match &tag {
            b"NFED" => {
                for (t, b) in atoms(body)? {
                    if t == *b"TJBO" {
                        definitions = strings(b)?;
                    }
                    if t == *b"YLOP" {
                        polygon_definitions = strings(b)?;
                    }
                }
            }
            b"DOEG" => {
                for (t, b) in atoms(body)? {
                    if t == *b"LOOP" {
                        pools.push(decode_pool(b)?);
                    }
                    if t == *b"LACS" {
                        scales.push(b);
                    }
                }
            }
            b"SDMC" => commands = Some(body),
            _ => {}
        }
    }
    if pools.len() != scales.len() {
        return Err("DSF pool/scale mismatch".into());
    }
    let point = |pool: usize, index: usize| -> Result<Vec<f64>, String> {
        let planes = pools.get(pool).ok_or("pool index out of range")?;
        let scale = scales[pool];
        if scale.len() != planes.len() * 8 {
            return Err("invalid DSF scaling atom".into());
        }
        planes
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let raw = *p.get(index).ok_or("point index out of range")? as f64;
                let mult = f32::from_le_bytes(scale[i * 8..i * 8 + 4].try_into().unwrap()) as f64;
                let offset =
                    f32::from_le_bytes(scale[i * 8 + 4..i * 8 + 8].try_into().unwrap()) as f64;
                Ok(if mult == 0.0 {
                    raw
                } else {
                    raw * mult / 65535.0 + offset
                })
            })
            .collect()
    };
    let mut c = Cursor {
        bytes: commands.ok_or("missing DSF commands")?,
        pos: 0,
    };
    let mut pool = None;
    let mut definition = None;
    let mut out = Features::default();
    while c.pos < c.bytes.len() {
        let at = c.pos;
        let op = c.u8()?;
        match op {
            1 => pool = Some(c.u16()? as usize),
            2 | 5 => {
                let v = c.u32()?;
                if op == 5 {
                    definition = Some(v as usize);
                }
            }
            3 => definition = Some(c.u8()? as usize),
            4 => definition = Some(c.u16()? as usize),
            6 => {
                c.take(1)?;
            }
            7 | 8 => {
                let first = c.u16()? as usize;
                let end = if op == 7 {
                    first + 1
                } else {
                    c.u16()? as usize
                };
                if end < first {
                    return Err("reversed object range".into());
                }
                let name = definitions
                    .get(definition.ok_or("missing object definition")?)
                    .ok_or("object definition out of range")?;
                for i in first..end {
                    let p = point(pool.ok_or("missing object pool")?, i)?;
                    if p.len() < 3 {
                        return Err("object pool needs three planes".into());
                    }
                    out.objects.push(ObjectPlacement {
                        resource: name.clone(),
                        lon: p[0],
                        lat: p[1],
                        heading: p[2] as f32,
                    });
                }
            }
            9 | 11 => {
                let n = c.u8()? as usize;
                c.take(n * if op == 9 { 2 } else { 4 })?;
            }
            10 => {
                c.take(4)?;
            }
            12..=15 => {
                let parameter = c.u16()?;
                let mut windings = Vec::new();
                let mut read_points = |indices: Vec<usize>| -> Result<(), String> {
                    let points = indices
                        .into_iter()
                        .map(|i| point(pool.ok_or("missing polygon pool")?, i))
                        .collect::<Result<Vec<_>, _>>()?;
                    if points
                        .iter()
                        .any(|p| p.len() < 2 || p.iter().any(|v| !v.is_finite()))
                    {
                        return Err("invalid polygon coordinates".into());
                    }
                    windings.push(points);
                    Ok(())
                };
                match op {
                    12 => {
                        let n = c.u8()?;
                        let indices = (0..n)
                            .map(|_| c.u16().map(usize::from))
                            .collect::<Result<Vec<_>, _>>()?;
                        read_points(indices)?;
                    }
                    13 => {
                        let first = c.u16()? as usize;
                        let end = c.u16()? as usize;
                        if end < first {
                            return Err("reversed polygon range".into());
                        }
                        read_points((first..end).collect())?;
                    }
                    14 => {
                        let n = c.u8()?;
                        for _ in 0..n {
                            let count = c.u8()?;
                            let indices = (0..count)
                                .map(|_| c.u16().map(usize::from))
                                .collect::<Result<Vec<_>, _>>()?;
                            read_points(indices)?;
                        }
                    }
                    _ => {
                        let n = c.u8()?;
                        let mut first = c.u16()? as usize;
                        for _ in 0..n {
                            let end = c.u16()? as usize;
                            if end < first {
                                return Err("reversed nested polygon range".into());
                            }
                            read_points((first..end).collect())?;
                            first = end;
                        }
                    }
                }
                let resource = polygon_definitions
                    .get(definition.ok_or("missing polygon definition")?)
                    .ok_or("polygon definition out of range")?
                    .clone();
                out.polygons.push(PolygonPlacement {
                    resource,
                    parameter,
                    windings,
                });
            }
            16 => {}
            17 => {
                c.take(1)?;
            }
            18 => {
                c.take(9)?;
            }
            23 | 26 | 29 => {
                let n = c.u8()? as usize;
                c.take(n * 2)?;
            }
            24 | 27 | 30 => {
                let n = c.u8()? as usize;
                c.take(n * 4)?;
            }
            25 | 28 | 31 => {
                c.take(4)?;
            }
            32..=34 => {
                let n = match op {
                    32 => c.u8()? as usize,
                    33 => c.u16()? as usize,
                    _ => c.u32()? as usize,
                };
                c.take(n)?;
            }
            _ => return Err(format!("unsupported DSF command {op} at {at}")),
        }
    }
    Ok(out)
}

fn decode_pool(data: &[u8]) -> Result<Vec<Vec<u16>>, String> {
    let mut c = Cursor {
        bytes: data,
        pos: 0,
    };
    let count = c.u32()? as usize;
    let dimensions = c.u8()? as usize;
    if count > 2_000_000 || dimensions > 32 {
        return Err("DSF pool exceeds bounds".into());
    }
    let mut planes = Vec::new();
    for _ in 0..dimensions {
        let encoding = c.u8()?;
        if encoding > 3 {
            return Err("unsupported pool encoding".into());
        }
        let mut values = Vec::with_capacity(count);
        while values.len() < count {
            let run = if encoding >= 2 { c.u8()? } else { 1 };
            let n = (run & 127) as usize;
            if n == 0 || values.len() + n > count {
                return Err("invalid DSF pool run".into());
            }
            if run & 128 != 0 {
                let v = c.u16()?;
                values.extend(std::iter::repeat_n(v, n));
            } else {
                for _ in 0..n {
                    values.push(c.u16()?);
                }
            }
        }
        if encoding & 1 != 0 {
            let mut prior = 0u16;
            for v in &mut values {
                *v = v.wrapping_add(prior);
                prior = *v;
            }
        }
        planes.push(values);
    }
    if c.pos != data.len() {
        return Err("trailing DSF pool bytes".into());
    }
    Ok(planes)
}

#[cfg(test)]
mod object_tests {
    use super::*;
    fn atom(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
        [
            tag.as_slice(),
            &((body.len() + 8) as u32).to_le_bytes(),
            body,
        ]
        .concat()
    }
    #[test]
    fn decodes_raw_delta_and_rle_pool_planes() {
        for encoding in 0..4u8 {
            let mut raw = 3u32.to_le_bytes().to_vec();
            raw.push(1);
            raw.push(encoding);
            if encoding >= 2 {
                raw.push(3);
            }
            let values = if encoding & 1 != 0 {
                [4u16, 1, 2]
            } else {
                [4u16, 5, 7]
            };
            for v in values {
                raw.extend(v.to_le_bytes());
            }
            assert_eq!(decode_pool(&raw).unwrap(), vec![vec![4, 5, 7]]);
        }
        let mut raw = 3u32.to_le_bytes().to_vec();
        raw.extend([1, 2, 131]);
        raw.extend(9u16.to_le_bytes());
        assert_eq!(decode_pool(&raw).unwrap(), vec![vec![9, 9, 9]]);
        raw[6] = 0;
        assert!(decode_pool(&raw).is_err());
    }
    #[test]
    fn object_range_uses_pool_scale_and_definition() {
        let mut pool = 2u32.to_le_bytes().to_vec();
        pool.push(3);
        for values in [[0u16, 65535], [0, 65535], [0, 32767]] {
            pool.push(0);
            for v in values {
                pool.extend(v.to_le_bytes());
            }
        }
        let scale = [1f32, -123.0, 1.0, 47.0, 360.0, 0.0]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        let mut data = b"XPLNEDSF".to_vec();
        data.extend(1u32.to_le_bytes());
        data.extend(atom(
            b"NFED",
            &[atom(b"TJBO", b"test.obj\0"), atom(b"YLOP", b"test.fac\0")].concat(),
        ));
        data.extend(atom(
            b"DOEG",
            &[atom(b"LOOP", &pool), atom(b"LACS", &scale)].concat(),
        ));
        for command in [
            vec![12, 7, 0, 2, 0, 0, 1, 0],
            vec![13, 7, 0, 0, 0, 2, 0],
            vec![14, 7, 0, 1, 2, 0, 0, 1, 0],
            vec![15, 7, 0, 1, 0, 0, 2, 0],
        ] {
            let mut data = data.clone();
            let mut commands = vec![1, 0, 0, 3, 0];
            commands.extend(command);
            commands.extend([8, 0, 0, 2, 0]);
            data.extend(atom(b"SDMC", &commands));
            data.extend([0; 16]);
            let features = features(&data).unwrap();
            assert_eq!(features.polygons.len(), 1);
            assert_eq!(features.polygons[0].resource, "test.fac");
            assert_eq!(features.polygons[0].windings[0].len(), 2);
            assert_eq!(
                features.polygons[0].windings[0][1],
                vec![-122.0, 48.0, 360.0 * 32767.0 / 65535.0]
            );
            assert_eq!(features.polygons[0].parameter, 7);
            let objects = features.objects;
            assert_eq!(objects.len(), 2);
            assert_eq!(objects[0].resource, "test.obj");
            assert_eq!(objects[0].lon, -123.0);
            assert_eq!(objects[1].lat, 48.0);
        }
        data.extend(atom(b"SDMC", &[1, 0, 0, 3, 0, 13, 0, 0, 2, 0, 0, 0]));
        data.extend([0; 16]);
        assert!(features(&data).is_err());
        data.truncate(data.len() - 17);
        assert!(object_placements(&data).is_err());
    }
}

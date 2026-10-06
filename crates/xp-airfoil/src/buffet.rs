//! Recovered noise and coefficient perturbation stages. Runtime table supplied explicitly.
//! Clock storage is identified; table initialization and clock advancement remain unknown.
use crate::airfoil::Coefficients;
use crate::runtime::RunningTime;

pub const TABLE_LEN: usize = 262144;
pub struct NoiseTable {
    values: Vec<f32>,
}

impl NoiseTable {
    pub fn new(values: Vec<f32>) -> Result<Self, String> {
        if values.len() != TABLE_LEN || values.iter().any(|v| !v.is_finite()) {
            return Err("noise table requires 262144 finite float32 values".into());
        }
        Ok(Self { values })
    }

    /// 0x1411e17a0..0x1411e1935, including mirror coordinates and wrapped indices.
    pub fn basis(&self, xyz: [f32; 3], seed: u32) -> Result<f32, String> {
        let mut integer = [0u32; 3];
        let mut fraction = [0.0; 3];
        for i in 0..3 {
            let a = xyz[i].abs();
            // Supported diagnostic domain; avoids the original's large-conversion edge cases.
            if !a.is_finite() || a >= 4294967296.0 {
                return Err("noise coordinate outside supported domain".into());
            }
            integer[i] = a as u32;
            fraction[i] = a - integer[i] as f32;
        }
        let [x, y, z] = integer;
        let [fx, fy, fz] = fraction;
        let base = z
            .wrapping_shl(6)
            .wrapping_add(y)
            .wrapping_shl(6)
            .wrapping_add(seed.wrapping_mul(100))
            .wrapping_add(x);
        let v = |offset: u32| self.values[(base.wrapping_add(offset) & 0x3ffff) as usize];
        let ix = 1.0 - fx;
        let iy = 1.0 - fy;
        let iz = 1.0 - fz;
        let upper0 = (ix * v(4096) + fx * v(4097)) * iy;
        let upper1 = (ix * v(4160) + fx * v(4161)) * fy;
        let upper = (upper0 + upper1) * fz;
        let lower1 = (ix * v(64) + fx * v(65)) * fy;
        let lower0 = (fx * v(1) + ix * v(0)) * iy;
        let lower = (lower1 + lower0) * iz;
        let combined = upper + lower;
        Ok((f64::from(combined) * 2.0 - 1.0) as f32)
    }

    /// `0x140984e50`: the two-dimensional lookup the propeller turbulence uses: the cell is `(|y| << 9) + |x| +
    /// 100 * seed` (32-bit), the four table entries at `+0`, `+1`, `+0x200`, `+0x201` (indices wrap at 0x40000)
    /// are blended bilinearly and mapped from `0..1` to `-1..1` in double precision.
    pub fn basis2(&self, x: f32, y: f32, seed: i32) -> f32 {
        let (ax, ay) = (x.abs(), y.abs());
        let (ix, iy) = (ax as i64 as u32, ay as i64 as u32);
        let base = (iy << 9)
            .wrapping_add(ix)
            .wrapping_add((seed as u32).wrapping_mul(100));
        let v = |offset: u32| self.values[(base.wrapping_add(offset) & 0x3ffff) as usize];
        let fx = ax - ix as f32;
        let fy = ay - iy as f32;
        let inv_x = 1.0 - fx;
        let inv_y = 1.0 - fy;
        let low = (fx * v(1) + inv_x * v(0)) * inv_y;
        let high = (inv_x * v(0x200) + fx * v(0x201)) * fy;
        (f64::from(low + high) * 2.0 - 1.0) as f32
    }

    /// 0x141a42a10..0x141a42c0e: six calls, double accumulation, final float32.
    pub fn fractal(&self, xyz: [f32; 3], seed: u32) -> Result<f32, String> {
        let sample = |scale: f64| self.basis(xyz.map(|v| (f64::from(v) * scale) as f32), seed);
        let mut result = f64::from(sample(2.0)?) * 0.5;
        result += f64::from(self.basis(xyz, seed)?);
        for (scale, weight) in [(4.0, 0.25), (8.0, 0.125), (16.0, 0.0625), (32.0, 0.03125)] {
            result += f64::from(sample(scale)?) * weight;
        }
        let result = result as f32;
        if !result.is_finite() {
            return Err("non-finite noise result".into());
        }
        Ok(result)
    }

    /// Evaluate with the internal double snapshot, bypassing the float dataref getter.
    pub fn perturb_at_time(
        &self,
        coefficients: Coefficients,
        xyz: [f32; 3],
        time: RunningTime,
    ) -> Result<Coefficients, String> {
        self.perturb(coefficients, xyz, time.seconds())
    }

    /// Active-stall branch only. Time is sim/time/total_running_time_sec in seconds.
    pub fn perturb(
        &self,
        coefficients: Coefficients,
        xyz: [f32; 3],
        running_time_seconds: f64,
    ) -> Result<Coefficients, String> {
        if !running_time_seconds.is_finite()
            || ![coefficients.cl, coefficients.cd, coefficients.cm]
                .iter()
                .all(|v| v.is_finite())
        {
            return Err("invalid perturbation operand".into());
        }
        let lift_phase = (running_time_seconds * 4.0 + f64::from(xyz[2])) as f32;
        let lift_noise = self.fractal([xyz[0], xyz[1], lift_phase], 0)?;
        let lift_factor = f64::from(lift_noise) * 0.25 + 1.0;
        let cl = (lift_factor * f64::from(coefficients.cl)) as f32;
        let drag_phase = (running_time_seconds * 5.0 + f64::from(xyz[2])) as f32;
        let drag_noise = self.fractal([xyz[0], xyz[1], drag_phase], 1)?;
        let drag_factor = f64::from(drag_noise) * 0.25 + 1.5;
        let cd = (drag_factor * f64::from(coefficients.cd)) as f32;
        if !cl.is_finite() || !cd.is_finite() {
            return Err("non-finite perturbed coefficient".into());
        }
        Ok(Coefficients {
            cl,
            cd,
            cm: coefficients.cm,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uses_corner_weights_seed_offsets_and_mirrored_coordinates() {
        let mut values = vec![0.5; TABLE_LEN];
        for (i, v) in [
            (0, 0.0),
            (1, 1.0),
            (64, 0.25),
            (65, 0.75),
            (4096, 0.125),
            (4097, 0.625),
            (4160, 0.375),
            (4161, 0.875),
        ] {
            values[i] = v;
        }
        values[100] = 0.75;
        let n = NoiseTable::new(values).unwrap();
        assert_eq!(n.basis([0.0; 3], 0).unwrap(), -1.0);
        assert_eq!(n.basis([0.5, 0.0, 0.0], 0).unwrap(), 0.0);
        assert_eq!(n.basis([0.5, 0.5, 0.5], 0).unwrap(), 0.0);
        assert_eq!(n.basis([0.0; 3], 1).unwrap(), 0.5);
        assert_eq!(
            n.basis([-0.25, -0.5, -0.75], 0).unwrap(),
            n.basis([0.25, 0.5, 0.75], 0).unwrap()
        );
        assert_eq!(n.basis([262144.0, 0.0, 0.0], 0).unwrap(), -1.0);
    }
    #[test]
    fn six_scales_and_coefficient_factors_preserve_cm() {
        let n = NoiseTable::new(vec![1.0; TABLE_LEN]).unwrap();
        assert_eq!(n.fractal([0.0; 3], 0).unwrap(), 1.96875);
        let c = n
            .perturb(
                Coefficients {
                    cl: 2.0,
                    cd: 0.25,
                    cm: -0.1,
                },
                [0.0; 3],
                0.0,
            )
            .unwrap();
        assert_eq!(c.cl, 2.984375);
        assert_eq!(c.cd, 255.0 / 512.0);
        assert_eq!(c.cm, -0.1);
        assert!(n.basis([f32::NAN, 0.0, 0.0], 0).is_err());
        assert!(NoiseTable::new(vec![0.0; 3]).is_err());
    }
}

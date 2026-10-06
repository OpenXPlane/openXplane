//! The aerodynamic functions of the bodies (fuselage-like volumes, the `B+0x6040` records) of the flight step.
use crate::engine::signed_pow;
use crate::vm::Vm;
use crate::wing_element::interpolate_clamped;

fn sse_min(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

fn sse_max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

const DEG: f32 = f32::from_bits(0x42652ee0);

/// The three results of [`body_aero`], the forces along the body's axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyForces {
    pub axial: f32,
    pub side: f32,
    pub normal: f32,
    /// The value the function returns: the magnitude of the two cross-flow terms.
    pub magnitude: f32,
}

/// `0x141a51600(R, q, &out1, &out3, &out2, a6, a7, a8, a9, diag)`: the cross-flow forces of a body record `R`
/// (lengths `+0x10`, `+0x14`, `+0x18`, end points `+0x58..0x60` and `+0x64..0x6c`, shape `+0`) at the dynamic
/// pressure term `q`: the angle `a8` gives the sine and the fourth power of the cosine, the slenderness from the
/// end points (through the two clamped interpolations with the 0.03 and 0.0031 terms) scales the cross-flow terms
/// `g1` (from `a6`, wrapped into -90..90 degrees) and `g2` (from `a7`), and `a9` the pressure `a9^2 q / 2`; the
/// third result interpolates the shape factor by the fourth power of `|sin a8|`. Bodies with a length under 0.0001
/// give nothing (`None`). The diagnostic output (`diag` nonzero) is not ported.
pub fn body_aero(
    vm: &Vm,
    r: u64,
    q: f32,
    a6: f32,
    a7: f32,
    a8: f32,
    a9: f32,
) -> Option<BodyForces> {
    let (x10, x14, x18) = (vm.f32(r + 0x10), vm.f32(r + 0x14), vm.f32(r + 0x18));
    if 0.0001 > f64::from(x18) || 0.0001 > f64::from(x14) || 0.0001 > f64::from(x10) {
        return None;
    }
    let s = a8.sin().abs();
    let c = a8.cos().abs();
    let p4 = signed_pow(c, 4.0);
    let scale = f64::from(x10) * f64::from_bits(0x3f90e56040000000);
    let min_length = f32::from_bits(0x38d1b717);
    let x14m = sse_max(x14, min_length);
    let x18m = sse_max(x18, min_length);
    let dx = vm.f32(r + 0x64) - vm.f32(r + 0x58);
    let dy = vm.f32(r + 0x68) - vm.f32(r + 0x5c);
    let dz = vm.f32(r + 0x6c) - vm.f32(r + 0x60);
    let t1 = (scale / f64::from(x18m)) as f32;
    let t2 = (scale / f64::from(x14m)) as f32;
    let k4 = f32::from_bits(0x3b4b295f);
    let ua = (f64::from(t1 + k4) * 0.5) as f32;
    let ub = (f64::from(t2 + k4) * 0.5) as f32;
    let ratio = |v: f32| {
        let v = v - 1.0 + 0.0;
        if 0.0 > v { 0.0 } else { sse_min(1.0, v) }
    };
    let (r1, r2) = (ratio(dx / dy), ratio(dy / dx));
    let k5 = f32::from_bits(0x3cf5c28f);
    let root = |v: f32| if 0.0 > v { f32::NAN } else { v.sqrt() };
    let fa = interpolate_clamped(0.0, ua, 1.0, root(dx / dz) * k5, r1);
    let fb = interpolate_clamped(0.0, ub, 1.0, root(dy / dz) * k5, r2);
    let mut w = a6 * DEG;
    while -90.0 > w {
        w += 180.0;
    }
    while w > 90.0 {
        w += -180.0;
    }
    let g1 = w * fa * p4;
    let g2 = fb * a7 * DEG * p4;
    let x12 = (f64::from(vm.f32(r + 0x18) + vm.f32(r + 0x14)) * 0.5 / f64::from(x10)) as f32;
    let fc = interpolate_clamped(0.0, vm.f32(r), 1.0, x12, signed_pow(s, 4.0));
    let qq = (f64::from(a9 * a9 * q) * 0.5) as f32;
    Some(BodyForces {
        axial: g1 * x18 * qq,
        side: g2 * x14 * qq,
        normal: fc * x10 * qq,
        magnitude: root(g1 * g1 + g2 * g2),
    })
}

/// `0x141a522d0(R, a, b, c)`: the supersonic wave term of a body whose surface is a grid of points (`R+0x654`
/// rows, `R+0x658` columns at most 18 per row, three floats each from `R+0x65c`, a row `0xd8` apart): the grid cells
/// (two triangles each) give an area-weighted mean normal; where its elevation `atan2(nz, |n_xy|)` is above -15 degrees
/// the Ackeret factor `2 / sqrt(M^2 - 1)` (with the Mach number `c / b` held to at least 2) turns the elevation
/// into a pressure term; the result sums `2 (sin(elevation) s + 0.0023) A h` over the cells, `h = c^2 a / 2`.
pub fn body_wave_drag(vm: &Vm, r: u64, a: f32, b: f32, c: f32) -> f32 {
    let mach = sse_max(c / b, 2.0);
    let h = (f64::from(c * c * a) * 0.5) as f32;
    let m2 = mach * mach;
    let ackeret = (2.0 / (f64::from(m2) - 1.0).sqrt()) as f32;
    let rows = vm.i32(r + 0x654) - 1;
    let mut acc = 0.0f32;
    if rows <= 0 {
        return acc;
    }
    let columns = vm.i32(r + 0x658);
    let cells = columns / 2 - 1;
    let point = |row: i64, col: i64| -> [f32; 3] {
        let base = r + 0x65c + (row * 0xd8 + col * 12) as u64;
        [vm.f32(base), vm.f32(base + 4), vm.f32(base + 8)]
    };
    let root = |v: f32| if 0.0 > v { f32::NAN } else { v.sqrt() };
    for row in 0..i64::from(rows) {
        for col in 0..i64::from(cells.max(0)) {
            let (mut area_sum, mut sx, mut sy, mut sz) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for triangle in 0..2 {
                let p0 = point(row, col);
                let (pb, pc) = if triangle == 0 {
                    (point(row, col + 1), point(row + 1, col + 1))
                } else {
                    (point(row + 1, col + 1), point(row + 1, col))
                };
                let u = [pb[0] - p0[0], pb[1] - p0[1], pb[2] - p0[2]];
                let v = [pc[0] - p0[0], pc[1] - p0[1], pc[2] - p0[2]];
                let ny = u[2] * v[0] - u[0] * v[2];
                let nx_first = u[2] * v[1] - u[1] * v[2];
                let nz = u[0] * v[1] - u[1] * v[0];
                let sum = nz * nz + nx_first * nx_first + ny * ny;
                let area = (f64::from(root(sum)) * 0.5) as f32;
                let nx = u[1] * v[2] - u[2] * v[1];
                if area > 0.0 {
                    area_sum += area;
                    sx += nx * area;
                    sy += ny * area;
                    sz += nz * area;
                }
            }
            if area_sum > 0.0 {
                let (mx, my, mz) = (sx / area_sum, sy / area_sum, sz / area_sum);
                let horizontal = root(my * my + mx * mx);
                let elevation = mz.atan2(horizontal);
                if elevation > f32::from_bits(0xbe860a93) {
                    let t = elevation * ackeret;
                    let s = if -0.5 > t { -0.5 } else { sse_min(0.5, t) };
                    let pressure = elevation.sin() * s;
                    let term = ((f64::from(pressure) + f64::from_bits(0x3f62d77318fc5048))
                        * f64::from(area_sum)
                        * f64::from(h)) as f32;
                    acc = (f64::from(term) * 2.0 + f64::from(acc)) as f32;
                }
            }
        }
    }
    acc
}

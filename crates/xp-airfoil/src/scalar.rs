//! Small scalar helpers of the original, all float32 with the original's comparison and `minss`/`maxss`
//! semantics (an unordered comparison takes the branch of "not greater"). Each is verified against the machine
//! code by `tools/gen_scalar_vectors.py`.

/// `0x14081dfa0`: `x` held to `lo..=hi` (`lo` wins over `hi` is not possible: the lower bound is tested first).
pub fn clamp(x: f32, lo: f32, hi: f32) -> f32 {
    if lo > x {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}

/// `0x140910be0`: `-1` for a negative argument, otherwise `1` (also for NaN).
pub fn sign(x: f32) -> f32 {
    if 0.0 > x { -1.0 } else { 1.0 }
}

/// `0x1411b4a20`: `maxss`-style maximum of three: `max(c, max(a, b))` with the second operand winning ties and NaN.
pub fn max3(a: f32, b: f32, c: f32) -> f32 {
    let m = if a > b { a } else { b };
    if c > m { c } else { m }
}

/// `0x1411b03f0`: a value inside `lo..=hi` snaps to the nearer bound (the midpoint goes to `hi`); a value outside
/// (or NaN, or when the bounds are crossed) is returned unchanged.
pub fn snap(x: f32, lo: f32, hi: f32) -> f32 {
    if lo > x || x > hi {
        return x;
    }
    let mid = (lo + hi) * 0.5;
    if mid > x { lo } else { hi }
}

/// `0x140819240`: `(1 - t) * a + b * t` with `t` held to `0..=1`.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    let t = clamp(t, 0.0, 1.0);
    (1.0 - t) * a + b * t
}

/// `0x1411e26e0`: whether a part record's kind word (`+0`) is 3 or 7.
pub fn kind_is_3_or_7(kind: i32) -> bool {
    (kind.wrapping_sub(3) as u32) & 0xffff_fffb == 0
}

/// `0x1408fd570`: the angle (degrees, wrapped to `0..=360`) between `v0` and `v1` at the fraction of `x` between
/// `a0` and `a1` (0.5 when they are equal, otherwise `(x - a0) / (a1 - a0)` held to 0..1), taking the shorter way
/// round (the difference is first wrapped to `-180..=180`).
pub fn angle_lerp(a0: f32, v0: f32, a1: f32, v1: f32, x: f32) -> f32 {
    let t = if a0 == a1 {
        0.5
    } else {
        let t = (1.0 / (a1 - a0)) * (x - a0) + 0.0;
        if 0.0 > t {
            0.0
        } else if 1.0 < t {
            1.0
        } else {
            t
        }
    };
    let mut d = v1 - v0;
    if -180.0 > d {
        while -180.0 > d {
            d += 360.0;
        }
    }
    if d > 180.0 {
        while d > 180.0 {
            d += -360.0;
        }
    }
    let mut r = d * t + v0;
    if 0.0 > r {
        while 0.0 > r {
            r += 360.0;
        }
    }
    if r > 360.0 {
        while r > 360.0 {
            r += -360.0;
        }
    }
    r
}

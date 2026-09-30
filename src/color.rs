//! Colour helpers.
//!
//! Colours are packed as **little endian RGBA** (`0xAA_BB_GG_RR`) so that the
//! `u32` can be dropped straight into the shared `Vec<u8>` pixel buffer with
//! `to_le_bytes()`, producing the exact `[R, G, B, A]` order that
//! `ImageData` expects on the JavaScript side - no per-channel work at all.

/// Pack four channels into a little endian RGBA value.
#[inline]
pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
}

/// Pack an opaque colour.
#[inline]
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    rgba(r, g, b, 255)
}

#[inline]
pub const fn red(color: u32) -> u8 {
    (color & 0xFF) as u8
}

#[inline]
pub const fn green(color: u32) -> u8 {
    ((color >> 8) & 0xFF) as u8
}

#[inline]
pub const fn blue(color: u32) -> u8 {
    ((color >> 16) & 0xFF) as u8
}

#[inline]
pub const fn alpha(color: u32) -> u8 {
    ((color >> 24) & 0xFF) as u8
}

/// `#rrggbb` string, used for the HTML palette in the UI.
pub fn to_hex(color: u32) -> String {
    format!("#{:02x}{:02x}{:02x}", red(color), green(color), blue(color))
}

/// Brighten (`delta > 0`) or darken (`delta < 0`) an RGB colour, keeping alpha.
#[inline]
pub fn shade(color: u32, delta: i16) -> u32 {
    if delta == 0 {
        return color;
    }
    #[inline]
    fn apply(c: u8, delta: i16) -> u8 {
        (i16::from(c) + delta).clamp(0, 255) as u8
    }
    rgba(
        apply(red(color), delta),
        apply(green(color), delta),
        apply(blue(color), delta),
        alpha(color),
    )
}

/// Linear interpolation between two colours, `t == 0` -> `a`, `t == 255` -> `b`.
#[inline]
pub fn mix(a: u32, b: u32, t: u8) -> u32 {
    #[inline]
    fn lerp(x: u8, y: u8, t: u8) -> u8 {
        let t = u32::from(t);
        (((u32::from(x) * (255 - t)) + (u32::from(y) * t)) / 255) as u8
    }
    rgba(
        lerp(red(a), red(b), t),
        lerp(green(a), green(b), t),
        lerp(blue(a), blue(b), t),
        lerp(alpha(a), alpha(b), t),
    )
}

/// Source-over blend of `src` (which may be translucent) on top of `dst`.
#[inline]
pub fn blend(dst: u32, src: u32) -> u32 {
    let sa = alpha(src);
    if sa == 255 {
        return src;
    }
    if sa == 0 {
        return dst;
    }
    #[inline]
    fn over(d: u8, s: u8, a: u8) -> u8 {
        // dst_out = src * a + dst * (1 - a)
        let a = u32::from(a);
        let blended = (u32::from(s) * a + u32::from(d) * (255 - a)) / 255;
        blended as u8
    }
    rgba(
        over(red(dst), red(src), sa),
        over(green(dst), green(src), sa),
        over(blue(dst), blue(src), sa),
        255,
    )
}

/// Cheap deterministic positional noise in `0..=255`, used to add a little
/// speckle to elements and a faint grain to the empty "air" background so that
/// large flat areas do not look like dead pixels.
#[inline]
pub fn noise2d(x: i32, y: i32, salt: u32) -> u8 {
    let mut h = (x as u32).wrapping_mul(0x27D4_EB2D) ^ (y as u32).wrapping_mul(0x1656_67B1) ^ salt;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    (h & 0xFF) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_round_trip() {
        let c = rgba(1, 2, 3, 4);
        assert_eq!((red(c), green(c), blue(c), alpha(c)), (1, 2, 3, 4));
        assert_eq!(c.to_le_bytes(), [1, 2, 3, 4]);
    }

    #[test]
    fn to_hex_formats_rgb() {
        assert_eq!(to_hex(rgb(0, 0, 0)), "#000000");
        assert_eq!(to_hex(rgb(255, 128, 16)), "#ff8010");
    }

    #[test]
    fn shade_clamps_and_keeps_alpha() {
        let c = rgba(250, 5, 128, 33);
        assert_eq!(red(shade(c, 20)), 255);
        assert_eq!(green(shade(c, -20)), 0);
        assert_eq!(blue(shade(c, 0)), 128);
        assert_eq!(alpha(shade(c, 20)), 33);
    }

    #[test]
    fn mix_endpoints_and_middle() {
        let a = rgb(0, 0, 0);
        let b = rgb(255, 255, 255);
        assert_eq!(mix(a, b, 0), a);
        assert_eq!(mix(a, b, 255), b);
        assert_eq!(red(mix(a, b, 128)), 128);
    }

    #[test]
    fn blend_respects_alpha() {
        let bg = rgb(100, 100, 100);
        assert_eq!(blend(bg, rgb(0, 0, 0)), rgb(0, 0, 0));
        assert_eq!(blend(bg, rgba(0, 0, 0, 0)), bg);
        let half = blend(bg, rgba(0, 0, 0, 128));
        assert!(red(half) < 100 && red(half) > 20, "got {half:#x}");
        assert_eq!(alpha(half), 255);
    }

    #[test]
    fn noise_is_deterministic_and_spread_out() {
        assert_eq!(noise2d(3, 7, 0), noise2d(3, 7, 0));
        assert_ne!(noise2d(3, 7, 0), noise2d(7, 3, 0));
        // Roughly uniform over a small block.
        let mut min = 255;
        let mut max = 0;
        for y in 0..16 {
            for x in 0..16 {
                let n = noise2d(x, y, 1);
                min = min.min(n);
                max = max.max(n);
            }
        }
        assert!(max - min > 128, "noise range was {min}..{max}");
    }
}

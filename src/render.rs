//! Rendering: state buffer -> shared RGBA pixel buffer.
//!
//! This is the half of the "zero copy" trick that lives in Rust. `render` writes
//! `[R, G, B, A]` bytes into a `Vec<u8>` that JavaScript reads through a
//! `Uint8ClampedArray` view over the wasm linear memory, so a frame costs one
//! pass over the grid and no serialisation at all.
//!
//! Air is pre-rendered once into a background buffer. Every frame starts by
//! copying that background, which lets translucent elements (steam, smoke,
//! hydrogen, fire) blend over it - without that reset they would smear the
//! previous frame into the current one and leave ghost trails.

use crate::cell::VARIANT_HOT;
use crate::color::{alpha, blend, mix, noise2d, rgba, shade};
use crate::element::Element;
use crate::grid::Grid;
use crate::world::World;

/// Strongest flicker applied to flames.
const FLAME_FLICKER: i16 = 14;

/// Paints a little grain into the empty background so big flat areas are not
/// dead pixels.
const AIR_GRAIN: i16 = 5;

/// Reads an RGBA pixel from the byte buffer.
#[inline]
fn read_pixel(buffer: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        buffer[offset],
        buffer[offset + 1],
        buffer[offset + 2],
        buffer[offset + 3],
    ])
}

/// Writes a colour (packed little endian RGBA) into the byte buffer.
#[inline]
fn write_pixel(buffer: &mut [u8], offset: usize, color: u32) {
    buffer[offset..offset + 4].copy_from_slice(&color.to_le_bytes());
}

/// Builds the pre-rendered empty world.
pub fn build_background(grid: &Grid) -> Vec<u8> {
    let mut buffer = vec![0u8; grid.len() * 4];
    let air = Element::Empty.color();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let grain = i16::from(noise2d(x, y, 0x51ED_2701)) / 40 - AIR_GRAIN;
            let offset = ((y * grid.width() + x) * 4) as usize;
            write_pixel(&mut buffer, offset, shade(air, grain));
        }
    }
    buffer
}

/// Colour of a flame, ramping from deep red (almost out) to white hot.
fn flame_color(life: u16, variant: u8, x: i32, y: i32) -> u32 {
    const RED: u32 = rgba(178, 38, 14, 235);
    const ORANGE: u32 = rgba(246, 130, 25, 235);
    const YELLOW: u32 = rgba(255, 198, 72, 235);
    const WHITE: u32 = rgba(255, 246, 214, 240);

    let t = (u32::from(life.min(140)) * 255 / 140) as u8;
    let base = if t < 85 {
        mix(RED, ORANGE, t.saturating_mul(3))
    } else if t < 170 {
        mix(ORANGE, YELLOW, (t - 85).saturating_mul(3))
    } else {
        mix(YELLOW, WHITE, (t - 170).saturating_mul(3))
    };
    // Burning magnesium is blindingly white.
    let base = if variant == VARIANT_HOT {
        mix(base, WHITE, 210)
    } else {
        base
    };
    let flicker = i16::from(noise2d(x, y, 0x00F1_A11E)) / 16 - 8;
    shade(base, flicker.clamp(-FLAME_FLICKER, FLAME_FLICKER))
}

/// Colour of one cell, including per-pixel speckle and flame gradients.
pub fn element_color(element: Element, variant: u8, life: u16, x: i32, y: i32) -> u32 {
    if element == Element::Fire {
        return flame_color(life, variant, x, y);
    }
    let props = element.props();
    if props.speckle == 0 {
        return props.color;
    }
    // Position based noise keeps the speckle stable frame to frame, so the world
    // does not sparkle while it is at rest.
    let noise = i16::from(noise2d(x, y, u32::from(element.id()).wrapping_mul(7919)));
    let delta = (noise - 128) * i16::from(props.speckle) / 160;
    shade(props.color, delta)
}

impl World {
    /// Redraws the shared pixel buffer from the state buffer.
    pub fn render(&mut self) {
        self.pixels.copy_from_slice(&self.background);
        let (width, height) = (self.grid.width(), self.grid.height());
        for y in 0..height {
            for x in 0..width {
                let cell = self.grid.cell(x, y);
                if cell.element == Element::Empty {
                    continue;
                }
                let color = element_color(cell.element, cell.variant, cell.life, x, y);
                let offset = ((y * width + x) * 4) as usize;
                if alpha(color) == 255 {
                    write_pixel(&mut self.pixels, offset, color);
                } else {
                    let under = read_pixel(&self.pixels, offset);
                    write_pixel(&mut self.pixels, offset, blend(under, color));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{alpha, blue, green, red};
    use crate::element::Element;

    fn pixel(world: &World, x: i32, y: i32) -> u32 {
        let offset = ((y * world.width() + x) * 4) as usize;
        let bytes = &world.pixels()[offset..offset + 4];
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    fn brightness(color: u32) -> u16 {
        u16::from(red(color)) + u16::from(green(color)) + u16::from(blue(color))
    }

    #[test]
    fn buffer_covers_every_pixel() {
        let world = World::new(32, 16, 1);
        assert_eq!(world.pixel_len(), 32 * 16 * 4);
        assert_eq!(world.pixel_len(), world.len() * 4);
        assert_eq!(world.pixels().len(), world.pixel_len());
    }

    #[test]
    fn empty_cells_render_the_background() {
        let world = World::new(32, 16, 1);
        let air = Element::Empty.color();
        for (x, y) in [(0, 0), (5, 5), (31, 15)] {
            let painted = pixel(&world, x, y);
            // Same hue as air, with a tiny positional grain.
            assert_eq!(alpha(painted), 255);
            assert!(
                (i16::from(red(painted)) - i16::from(red(air))).abs() <= 12,
                "grain too strong at ({x}, {y})"
            );
        }
    }

    #[test]
    fn opaque_elements_are_written_verbatim() {
        let mut world = World::new(16, 16, 1);
        // Wall has zero speckle, so its colour must survive exactly.
        world.paint_circle(4, 4, 1, Element::Wall);
        assert_eq!(pixel(&world, 4, 4), Element::Wall.color());
    }

    #[test]
    fn speckled_elements_vary_per_pixel() {
        let mut world = World::new(32, 32, 1);
        world.paint_circle(16, 16, 10, Element::Sand);
        let base = Element::Sand.color();
        let mut seen = std::collections::HashSet::new();
        for y in 8..24 {
            for x in 8..24 {
                if world.element(x, y) != Element::Sand {
                    continue;
                }
                let shade = pixel(&world, x, y);
                // Still recognisably sand, never noise.
                assert!((i16::from(red(shade)) - i16::from(red(base))).abs() < 30);
                seen.insert(shade);
            }
        }
        assert!(
            seen.len() > 5,
            "sand should look grainy, got {} shades",
            seen.len()
        );
    }

    #[test]
    fn translucent_elements_blend_into_an_opaque_pixel() {
        let mut world = World::new(16, 16, 1);
        let before = pixel(&world, 8, 8);
        world.paint_circle(8, 8, 2, Element::Steam);
        let after = pixel(&world, 8, 8);
        assert_ne!(before, after, "steam must tint what is behind it");
        assert_eq!(alpha(after), 255, "the shared buffer is always opaque");
        assert_eq!(alpha(Element::Steam.color()), 120);
    }

    #[test]
    fn flames_ramp_from_red_to_white() {
        let cool = element_color(Element::Fire, 0, 2, 0, 0);
        let mid = element_color(Element::Fire, 0, 60, 0, 0);
        let hot = element_color(Element::Fire, 0, 140, 0, 0);
        assert!(
            brightness(cool) < brightness(mid) && brightness(mid) < brightness(hot),
            "flames should get brighter as they are fed"
        );
        // Magnesium reports itself as HOT, so the same life is painted whiter.
        let magnified = element_color(Element::Fire, VARIANT_HOT, 60, 0, 0);
        assert!(brightness(magnified) > brightness(mid));
    }

    #[test]
    fn render_is_deterministic() {
        let mut a = World::new(24, 24, 7);
        let mut b = World::new(24, 24, 7);
        for world in [&mut a, &mut b] {
            world.paint_circle(12, 6, 5, Element::Sand);
            world.paint_circle(12, 18, 4, Element::Water);
            world.step_n(30);
        }
        assert_eq!(a.pixels(), b.pixels());
        assert_eq!(a.hash(), b.hash());
    }

    #[test]
    fn render_refreshes_the_whole_buffer() {
        let mut world = World::new(16, 16, 3);
        world.paint_circle(8, 8, 3, Element::Lava);
        assert_eq!(pixel(&world, 8, 8), {
            // Lava is opaque, so the centre is lava (with speckle applied).
            let expected = element_color(Element::Lava, 0, 0, 8, 8);
            assert_eq!(alpha(expected), 255);
            expected
        });
    }
}

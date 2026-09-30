//! The simulation world: state buffer, chemistry, the per-tick update loop and
//! the shared pixel buffer that JavaScript reads directly.

use std::collections::VecDeque;
use std::fmt::Write as _;

use crate::cell::Cell;
use crate::color::to_hex;
use crate::element::{Category, Element};
use crate::grid::Grid;
use crate::rng::Rng;
use crate::{physics, reaction, render};

/// Default world size: chunky enough for a falling sand game, small enough that
/// one tick costs microseconds.
pub const DEFAULT_WIDTH: i32 = 320;
/// Default world height.
pub const DEFAULT_HEIGHT: i32 = 200;

/// How many snapshots the undo stack keeps.
pub const UNDO_LIMIT: usize = 16;

/// Highest fall speed in cells per tick, reached after a short acceleration.
pub const TERMINAL_FALL: i32 = 3;
/// Highest climb speed in cells per tick for gases.
pub const TERMINAL_RISE: i32 = 4;
/// Momentum is capped here so the acceleration maths stays inside `i8`.
pub const MAX_MOMENTUM: i8 = 12;

/// One line of the reaction log.
#[derive(Clone, Copy, Debug)]
pub struct LogEntry {
    /// The balanced equation (or a descriptive line).
    pub equation: &'static str,
    /// How many times it fired in the frame it was last seen.
    pub count: u32,
    /// Frame in which it last fired.
    pub frame: u32,
}

/// Rolling "what is happening in my beaker" log, surfaced in the UI.
#[derive(Default)]
struct ReactionLog {
    this_frame: Vec<(&'static str, u32)>,
    recent: VecDeque<LogEntry>,
}

impl ReactionLog {
    const CAPACITY: usize = 8;

    fn begin_frame(&mut self) {
        self.this_frame.clear();
    }

    fn record(&mut self, equation: &'static str) {
        if let Some(entry) = self.this_frame.iter_mut().find(|(e, _)| *e == equation) {
            entry.1 += 1;
        } else {
            self.this_frame.push((equation, 1));
        }
    }

    fn end_frame(&mut self, frame: u32) {
        for (equation, count) in &self.this_frame {
            if let Some(front) = self.recent.front_mut() {
                if front.equation == *equation {
                    front.count = front.count.saturating_add(*count);
                    front.frame = frame;
                    continue;
                }
            }
            self.recent.push_front(LogEntry {
                equation,
                count: *count,
                frame,
            });
            while self.recent.len() > Self::CAPACITY {
                self.recent.pop_back();
            }
        }
    }

    fn entries(&self) -> impl Iterator<Item = &LogEntry> {
        self.recent.iter()
    }
}

/// The whole simulation. Everything here is plain `std` Rust so the crate builds
/// and tests natively; only the `lib.rs` wrapper knows about wasm.
pub struct World {
    pub(crate) grid: Grid,
    pub(crate) rng: Rng,
    /// Monotonic tick counter, doubling as the "already updated" stamp value.
    pub(crate) frame: u32,
    /// Shared pixel buffer: `width * height * 4` bytes of `[R, G, B, A]`.
    pub(crate) pixels: Vec<u8>,
    /// Pre-rendered empty world, copied into `pixels` before every frame.
    pub(crate) background: Vec<u8>,
    log: ReactionLog,
    undo: Vec<Vec<Cell>>,
}

impl World {
    /// Creates an empty world with a deterministic random stream.
    pub fn new(width: i32, height: i32, seed: u32) -> Self {
        let grid = Grid::new(width, height);
        let mut world = Self {
            pixels: vec![0; grid.len() * 4],
            background: render::build_background(&grid),
            grid,
            rng: Rng::new(seed),
            frame: 0,
            log: ReactionLog::default(),
            undo: Vec::new(),
        };
        world.render();
        world
    }

    #[inline]
    pub fn width(&self) -> i32 {
        self.grid.width()
    }

    #[inline]
    pub fn height(&self) -> i32 {
        self.grid.height()
    }

    /// Number of interior cells.
    #[inline]
    pub fn len(&self) -> usize {
        self.grid.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.grid.is_empty()
    }

    /// Ticks simulated so far.
    #[inline]
    pub fn frame(&self) -> u32 {
        self.frame
    }

    /// Read-only access to the state buffer.
    #[inline]
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Mutable access to the state buffer (tests and advanced tooling).
    #[inline]
    pub fn grid_mut(&mut self) -> &mut Grid {
        &mut self.grid
    }

    /// Element at an interior coordinate.
    #[inline]
    pub fn element(&self, x: i32, y: i32) -> Element {
        self.grid.element(x, y)
    }

    /// The shared pixel buffer, `width * height * 4` bytes of RGBA.
    #[inline]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Pointer to the shared pixel buffer, handed to JavaScript so it can wrap
    /// that memory in a `Uint8ClampedArray` without copying anything.
    #[inline]
    pub fn pixel_ptr(&self) -> *const u8 {
        self.pixels.as_ptr()
    }

    /// Length of the pixel buffer in bytes.
    #[inline]
    pub fn pixel_len(&self) -> usize {
        self.pixels.len()
    }

    /// Order independent fingerprint of the state buffer.
    pub fn hash(&self) -> u32 {
        self.grid.hash()
    }

    /// How many cells are not air?
    pub fn particle_count(&self) -> u32 {
        self.len() as u32 - self.grid.count(Element::Empty)
    }

    /// How many cells are made of `element`?
    pub fn count(&self, element: Element) -> u32 {
        self.grid.count(element)
    }

    /// Recent chemical reactions, newest first, as JSON.
    pub fn reactions_json(&self) -> String {
        let mut out = String::from("[");
        for (i, entry) in self.log.entries().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "{{\"equation\":\"{}\",\"count\":{},\"frame\":{}}}",
                entry.equation, entry.count, entry.frame
            );
        }
        out.push(']');
        out
    }

    /// The element catalogue as JSON, so the UI builds its palette from the
    /// single source of truth instead of duplicating it in JavaScript.
    pub fn element_catalog_json() -> String {
        let mut out = String::from("[");
        for (i, element) in Element::ALL.into_iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let props = element.props();
            let _ = write!(
                out,
                "{{\"id\":{},\"name\":\"{}\",\"formula\":\"{}\",\"category\":\"{}\",\"color\":\"{}\",\"density\":{},\"desc\":\"{}\"}}",
                element.id(),
                props.name,
                props.formula,
                props.category.as_str(),
                to_hex(props.color),
                props.density,
                props.desc
            );
        }
        out.push(']');
        out
    }

    /// Advances the simulation by exactly one tick and redraws the pixels.
    pub fn step(&mut self) {
        self.advance();
        self.render();
    }

    /// Runs `ticks` ticks (used by the speed slider and by the tests).
    ///
    /// Rendering happens once at the end: at 4x speed the intermediate frames
    /// would only ever be overwritten, so this is four times less pixel work per
    /// displayed frame with an identical result.
    pub fn step_n(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.advance();
        }
        self.render();
    }

    /// One tick of simulation, without redrawing the pixel buffer.
    ///
    /// Canonical powder-game ordering, and the reason it looks right:
    ///
    /// * **Bottom-up rows.** Falling cells move *down*, into rows that were
    ///   already processed this tick, so a grain can never be updated twice and
    ///   teleport to the floor.
    /// * **Alternating columns.** Even frames scan left to right, odd frames
    ///   right to left. Without this, water would always prefer to flow left
    ///   (or right) and the world would show a constant directional bias.
    /// * **Stamps.** Anything that moves on a different axis (gases rising,
    ///   liquids sliding sideways) is stamped with the frame number so it is
    ///   skipped for the rest of the tick.
    fn advance(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        if self.frame == 0 {
            // The counter wrapped: old stamps must not alias a fresh frame.
            self.frame = 1;
            self.grid.clear_stamps();
        }
        let frame = self.frame;
        self.log.begin_frame();

        let left_to_right = frame % 2 == 0;
        let (width, height) = (self.width(), self.height());
        for y in (0..height).rev() {
            if left_to_right {
                for x in 0..width {
                    self.tick_cell(x, y);
                }
            } else {
                for x in (0..width).rev() {
                    self.tick_cell(x, y);
                }
            }
        }

        self.log.end_frame(frame);
    }

    /// One cell: chemistry, then lifetime, then heat, then movement.
    fn tick_cell(&mut self, x: i32, y: i32) {
        let frame = self.frame;
        if self.grid.is_marked(x, y, frame) {
            return;
        }
        let cell = self.grid.cell(x, y);
        if cell.is_empty() || cell.element == Element::Wall {
            return;
        }

        // 1. Chemistry. A reaction may replace this cell entirely, in which case
        //    it gets its own movement phase on the next tick.
        if reaction::react(self, x, y, cell.element) {
            return;
        }

        // 2. Lifetime. Done before movement so the decremented life travels
        //    along with the cell; an expired cell becomes its residue and waits
        //    for the next tick.
        let mut cell = self.grid.cell(x, y);
        if cell.life > 0 {
            if cell.tick_lifetime() {
                self.grid.set(x, y, cell);
                return;
            }
            self.grid.set(x, y, cell);
        }

        // 3. Heat sources (fire, lava) ignite whatever fuel touches them.
        if cell.element.props().heat > 0 {
            physics::ignite_neighbours(self, x, y, cell.element);
        }

        // 4. Movement.
        match cell.element.category() {
            Category::Powder => physics::step_powder(self, x, y),
            Category::Liquid => physics::step_liquid(self, x, y),
            Category::Gas => physics::step_gas(self, x, y),
            Category::Fire => physics::step_fire(self, x, y),
            Category::Solid => physics::step_solid(self, x, y),
            Category::Empty => {}
        }
    }

    /// Remembers the current state so a whole brush stroke can be undone.
    pub fn snapshot(&mut self) {
        if self.undo.len() >= UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.undo.push(self.grid.snapshot());
    }

    /// Is there anything to undo?
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Steps back to the previous snapshot.
    pub fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some(snapshot) => {
                self.grid.restore(&snapshot);
                self.grid.clear_stamps();
                self.render();
                true
            }
            None => false,
        }
    }
}

/// Movement plumbing and the tools that JavaScript drives.
impl World {
    /// Moves a cell and stamps **both** ends with the current frame, which is
    /// what guarantees that nothing is simulated twice in one tick.
    ///
    /// Marking the source matters too: whatever gets displaced into the old
    /// position (water pushed up by sinking sand) must not move again either.
    pub(crate) fn move_cell(&mut self, from: (i32, i32), to: (i32, i32)) {
        self.grid.swap(from.0, from.1, to.0, to.1);
        self.grid.mark(from.0, from.1, self.frame);
        self.grid.mark(to.0, to.1, self.frame);
    }

    /// Same as `move_cell` but with a little momentum added to the mover, so
    /// falling material speeds up until it reaches terminal velocity.
    pub(crate) fn move_with_momentum(&mut self, from: (i32, i32), to: (i32, i32)) {
        let mut cell = self.grid.cell(from.0, from.1);
        cell.vel = cell.vel.saturating_add(1).min(MAX_MOMENTUM);
        self.grid.set(from.0, from.1, cell);
        self.move_cell(from, to);
    }

    /// Bleeds off momentum after a blocked move.
    pub(crate) fn stop_momentum(&mut self, x: i32, y: i32) {
        let mut cell = self.grid.cell(x, y);
        if cell.vel != 0 {
            cell.vel = 0;
            self.grid.set(x, y, cell);
        }
    }

    /// Records a chemical reaction for the on-screen log.
    pub(crate) fn log_reaction(&mut self, equation: &'static str) {
        self.log.record(equation);
    }

    /// Blows a hole in the world.
    ///
    /// `Wall` is immune, everything else is destroyed or replaced by fire, and a
    /// puff of `product` is released around the rim (hydrogen for alkali metals,
    /// smoke for the explosives).
    pub fn explode(&mut self, cx: i32, cy: i32, radius: i32, product: Element) {
        let radius = radius.clamp(1, 32);
        let r2 = radius * radius;
        let inner = radius / 2;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = dx * dx + dy * dy;
                if d2 > r2 {
                    continue;
                }
                let (x, y) = (cx + dx, cy + dy);
                if !self.grid.in_bounds(x, y) || self.grid.element(x, y) == Element::Wall {
                    continue;
                }
                // Full force at the centre, tapering to a shove at the rim.
                let falloff = (d2 as f32).sqrt() / radius as f32;
                let destroy = (255.0 - falloff * 195.0) as u8;
                if !self.rng.chance(destroy) {
                    continue;
                }
                if d2 <= inner * inner {
                    self.grid
                        .set_transient(x, y, Element::Fire, 36, Element::Smoke);
                } else if self.rng.chance(150) {
                    self.grid
                        .set_transient(x, y, Element::Fire, 20, Element::Smoke);
                } else if product != Element::Empty && self.rng.chance(110) {
                    let life = product.props().life;
                    let residue = product.props().residue;
                    self.grid.set_transient(x, y, product, life, residue);
                } else {
                    self.grid.set(x, y, Cell::empty());
                }
                // Blast material sits out the rest of the tick.
                self.grid.mark(x, y, self.frame);
            }
        }
    }

    /// Paints a filled circle of `element`, returning the number of cells
    /// covered. `radius` is in cells: `0` paints a single pixel.
    pub fn paint_circle(&mut self, cx: i32, cy: i32, radius: i32, element: Element) -> u32 {
        let painted = self.paint_circle_raw(cx, cy, radius, element);
        self.render();
        painted
    }

    fn paint_circle_raw(&mut self, cx: i32, cy: i32, radius: i32, element: Element) -> u32 {
        let radius = radius.clamp(0, 64);
        let r2 = radius * radius;
        let cell = Cell::new(element);
        let mut painted = 0;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let (x, y) = (cx + dx, cy + dy);
                if !self.grid.in_bounds(x, y) {
                    continue;
                }
                self.grid.set(x, y, cell);
                painted += 1;
            }
        }
        painted
    }

    /// Paints an interpolation-free straight-ish line of circles, so that fast
    /// mouse drags do not leave gaps. Uses integer maths to stay deterministic.
    pub fn paint_line(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        radius: i32,
        element: Element,
    ) -> u32 {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let steps = dx.abs().max(dy.abs()).max(1);
        let mut painted = 0;
        for i in 0..=steps {
            let x = x0 + dx * i / steps;
            let y = y0 + dy * i / steps;
            painted += self.paint_circle_raw(x, y, radius, element);
        }
        self.render();
        painted
    }

    /// Paints a rectangle (inclusive corners). Handy for the square brush and
    /// for building demo scenes.
    pub fn paint_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, element: Element) -> u32 {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (y0, y1) = (y0.min(y1), y0.max(y1));
        let cell = Cell::new(element);
        let mut painted = 0;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if !self.grid.in_bounds(x, y) {
                    continue;
                }
                self.grid.set(x, y, cell);
                painted += 1;
            }
        }
        self.render();
        painted
    }

    /// Empties the world (undoable).
    pub fn clear(&mut self) {
        self.snapshot();
        self.grid.clear(Element::Empty);
        self.grid.clear_stamps();
        self.render();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lab() -> World {
        World::new(32, 24, 42)
    }

    #[test]
    fn painting_a_circle_covers_the_disc() {
        let mut world = lab();
        let painted = world.paint_circle(16, 12, 3, Element::Sand);
        // Cells within r = 3 of the centre: 3^2 + ... <= 9 -> 29 cells.
        assert_eq!(painted, 29);
        assert_eq!(world.count(Element::Sand), painted);
        assert_eq!(world.element(16, 12), Element::Sand);
        assert_eq!(world.element(16, 12 + 3), Element::Sand);
        assert_eq!(world.element(16, 12 + 4), Element::Empty);
    }

    #[test]
    fn painting_outside_the_world_is_safe_and_clipped() {
        let mut world = lab();
        let painted = world.paint_circle(-20, -20, 4, Element::Water);
        assert_eq!(painted, 0, "nothing is painted off-world");
        let painted = world.paint_circle(0, 0, 4, Element::Water);
        assert!(
            painted > 0 && painted < 29,
            "the brush is clipped at the edge"
        );
        let mut world = lab();
        assert_eq!(world.paint_rect(-5, -5, 2, 2, Element::Stone), 9);
    }

    #[test]
    fn painting_overwrites_anything() {
        let mut world = lab();
        world.paint_circle(10, 10, 3, Element::Water);
        world.paint_circle(10, 10, 1, Element::Wall);
        assert_eq!(world.element(10, 10), Element::Wall);
        assert!(world.count(Element::Water) > 0, "the ring is still water");
    }

    #[test]
    fn a_stroke_has_no_gaps() {
        let mut world = lab();
        world.paint_line(2, 2, 28, 20, 0, Element::Stone);
        for x in 2..=28 {
            let y = 2 + (20 - 2) * (x - 2) / 26;
            assert_eq!(
                world.element(x, y),
                Element::Stone,
                "the stroke skipped ({x}, {y})"
            );
        }
    }

    #[test]
    fn a_rectangle_fills_its_area() {
        let mut world = lab();
        let painted = world.paint_rect(2, 3, 5, 7, Element::Ice);
        assert_eq!(painted, 4 * 5);
        assert_eq!(world.count(Element::Ice), painted);
        assert_eq!(world.element(5, 7), Element::Ice);
        assert_eq!(world.element(6, 7), Element::Empty);
    }

    #[test]
    fn clear_empties_the_interior_and_is_undoable() {
        let mut world = lab();
        world.paint_circle(16, 12, 6, Element::Sand);
        world.paint_circle(8, 6, 3, Element::Water);
        world.snapshot();
        let before = world.hash();

        world.clear();
        assert_eq!(world.particle_count(), 0);
        assert_eq!(world.element(-1, 0), Element::Wall, "the border stays");

        assert!(world.undo());
        assert_eq!(world.hash(), before, "undo brings the lab back");
    }

    #[test]
    fn particle_count_matches_the_individual_counts() {
        let mut world = lab();
        world.paint_circle(10, 10, 4, Element::Sand);
        world.paint_circle(20, 10, 4, Element::Water);
        world.paint_rect(1, 20, 30, 22, Element::Stone);
        let sum = [Element::Sand, Element::Water, Element::Stone]
            .into_iter()
            .map(|element| world.count(element))
            .sum::<u32>();
        assert_eq!(world.particle_count(), sum);
    }

    #[test]
    fn stepping_changes_the_state_and_the_pixels() {
        let mut world = lab();
        world.paint_circle(16, 4, 4, Element::Sand);
        let hash = world.hash();
        let pixel = world.pixels()[0];
        world.step();
        assert_ne!(world.hash(), hash, "the sand should have moved");
        assert_eq!(world.pixels().len(), world.len() * 4);
        assert_eq!(world.pixels()[0], pixel, "the render pass is stable");
    }

    #[test]
    fn the_catalogue_lists_every_element_with_a_colour() {
        let json = World::element_catalog_json();
        assert!(json.starts_with('[') && json.ends_with(']'));
        for element in Element::ALL {
            assert!(
                json.contains(&format!("\"name\":\"{}\"", element.name())),
                "{element:?} is missing"
            );
        }
        assert_eq!(json.matches("\"color\":\"#").count(), Element::ALL.len());
    }

    #[test]
    fn the_reaction_log_starts_empty_and_stays_valid_json() {
        let mut world = lab();
        assert_eq!(world.reactions_json(), "[]");
        world.paint_circle(16, 12, 3, Element::Fire);
        world.paint_circle(16, 12, 1, Element::Oil);
        world.step_n(30);
        let json = world.reactions_json();
        assert_ne!(json, "[]");
        assert!(json.starts_with('[') && json.ends_with(']'));
    }

    #[test]
    fn the_undo_stack_is_bounded() {
        let mut world = lab();
        for _ in 0..(UNDO_LIMIT * 2) {
            world.snapshot();
        }
        for _ in 0..UNDO_LIMIT {
            assert!(world.undo(), "the stack should hold exactly UNDO_LIMIT");
        }
        assert!(!world.can_undo(), "and no more");
    }

    #[test]
    fn the_frame_counter_advances() {
        let mut world = lab();
        assert_eq!(world.frame(), 0);
        world.step_n(5);
        assert_eq!(world.frame(), 5);
    }
}

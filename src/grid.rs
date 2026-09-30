//! The state buffer: one flat `Vec<Cell>` plus a per-cell "already updated this
//! frame" stamp, wrapped in a one cell thick inert border.
//!
//! Why flat? `grid.index(x, y) == (y + 1) * stride + (x + 1)` keeps rows next to
//! each other in memory, so scanning a row is a linear walk through the cache and
//! the whole world is a single allocation instead of `height` boxed slices.
//!
//! Why the border? Every interior cell then has all eight neighbours inside the
//! allocation, so neighbour lookups need **no bounds checks and no branching on
//! edges**, and nothing can wrap around from one side of the world to the other.
//! The border is filled with `Wall`, so it also acts as the floor, ceiling and
//! side walls of the lab.

use crate::cell::Cell;
use crate::element::Element;

/// Immovable border material.
pub const BORDER_ELEMENT: Element = Element::Wall;

/// Smallest world side.
pub const MIN_SIZE: i32 = 8;
/// Largest world side, mostly to keep the browser happy.
pub const MAX_SIZE: i32 = 2048;

/// A flat, bordered cellular automaton grid.
#[derive(Clone)]
pub struct Grid {
    width: i32,
    height: i32,
    stride: i32,
    cells: Vec<Cell>,
    stamp: Vec<u32>,
}

impl Grid {
    /// Allocates a `width x height` world surrounded by `Wall`.
    pub fn new(width: i32, height: i32) -> Self {
        let width = width.clamp(MIN_SIZE, MAX_SIZE);
        let height = height.clamp(MIN_SIZE, MAX_SIZE);
        let stride = width + 2;
        let total = (stride * (height + 2)) as usize;
        let mut cells = vec![Cell::new(BORDER_ELEMENT); total];
        for y in 0..height {
            for x in 0..width {
                cells[((y + 1) * stride + (x + 1)) as usize] = Cell::empty();
            }
        }
        Self {
            width,
            height,
            stride,
            cells,
            stamp: vec![0; total],
        }
    }

    #[inline]
    pub fn width(&self) -> i32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Number of interior cells.
    #[inline]
    pub fn len(&self) -> usize {
        (self.width * self.height) as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Is this an interior coordinate?
    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }

    /// Flat index for an interior coordinate. `-1` and `width` address the
    /// border on purpose, and anything further away is clamped onto the border,
    /// which makes every neighbour lookup branch-free *and* panic-free.
    #[inline]
    pub fn index(&self, x: i32, y: i32) -> usize {
        let x = x.clamp(-1, self.width);
        let y = y.clamp(-1, self.height);
        ((y + 1) * self.stride + (x + 1)) as usize
    }

    /// Cell at an interior coordinate. Out of range coordinates resolve to the
    /// inert border instead of panicking.
    #[inline]
    pub fn cell(&self, x: i32, y: i32) -> Cell {
        self.cells[self.index(x, y)]
    }

    /// Element at an interior coordinate.
    #[inline]
    pub fn element(&self, x: i32, y: i32) -> Element {
        self.cell(x, y).element
    }

    /// Overwrite a cell.
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, cell: Cell) {
        let index = self.index(x, y);
        self.cells[index] = cell;
    }

    /// Overwrite a cell with a fresh copy of `element`.
    #[inline]
    pub fn set_element(&mut self, x: i32, y: i32, element: Element) {
        self.set(x, y, Cell::new(element));
    }

    /// Overwrite a cell with a transient element (fire, smoke, steam).
    #[inline]
    pub fn set_transient(&mut self, x: i32, y: i32, element: Element, life: u16, residue: Element) {
        self.set(x, y, Cell::transient(element, life, residue));
    }

    /// Compare and swap, so two reactions in the same tick cannot both consume
    /// the same cell.
    #[inline]
    pub fn replace(&mut self, x: i32, y: i32, expected: Element, cell: Cell) -> bool {
        if self.element(x, y) == expected {
            self.set(x, y, cell);
            true
        } else {
            false
        }
    }

    /// Exchange two cells.
    #[inline]
    pub fn swap(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let a = self.index(x0, y0);
        let b = self.index(x1, y1);
        self.cells.swap(a, b);
    }
}

impl Grid {
    /// Stamp a cell as "already updated in `frame`".
    #[inline]
    pub fn mark(&mut self, x: i32, y: i32, frame: u32) {
        let index = self.index(x, y);
        self.stamp[index] = frame;
    }

    /// Was this cell already updated in `frame`?
    #[inline]
    pub fn is_marked(&self, x: i32, y: i32, frame: u32) -> bool {
        self.stamp[self.index(x, y)] == frame
    }

    /// Forget all stamps. Called when the frame counter wraps around.
    pub fn clear_stamps(&mut self) {
        self.stamp.fill(0);
    }

    /// Copy of the interior cells, row major (`y * width + x`).
    pub fn snapshot(&self) -> Vec<Cell> {
        let mut out = Vec::with_capacity(self.len());
        for y in 0..self.height {
            for x in 0..self.width {
                out.push(self.cell(x, y));
            }
        }
        out
    }

    /// Restores a snapshot produced by `snapshot`. Extra or missing cells are
    /// ignored, so a snapshot from a differently sized world cannot corrupt this
    /// one.
    pub fn restore(&mut self, snapshot: &[Cell]) {
        for y in 0..self.height {
            for x in 0..self.width {
                let index = (y * self.width + x) as usize;
                if let Some(cell) = snapshot.get(index) {
                    let flat = self.index(x, y);
                    self.cells[flat] = *cell;
                }
            }
        }
    }

    /// Turn every interior cell into `fill`.
    pub fn clear(&mut self, fill: Element) {
        let cell = Cell::new(fill);
        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.index(x, y);
                self.cells[index] = cell;
            }
        }
    }

    /// How many cells of `element` exist?
    pub fn count(&self, element: Element) -> u32 {
        let mut total = 0;
        for y in 0..self.height {
            for x in 0..self.width {
                if self.cell(x, y).element == element {
                    total += 1;
                }
            }
        }
        total
    }

    /// Every interior coordinate, row by row from the top.
    pub fn coords(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        (0..self.height).flat_map(move |y| (0..self.width).map(move |x| (x, y)))
    }

    /// Order independent fingerprint of the state buffer, used to prove that the
    /// simulation is deterministic.
    pub fn hash(&self) -> u32 {
        let mut hash: u32 = 0x811C_9DC5;
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = self.cell(x, y);
                for byte in [
                    cell.element.id(),
                    cell.variant,
                    cell.residue.id(),
                    cell.vel as u8,
                    (cell.life & 0xFF) as u8,
                    (cell.life >> 8) as u8,
                ] {
                    hash ^= u32::from(byte);
                    hash = hash.wrapping_mul(0x0100_0193);
                }
            }
        }
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::Cell;

    fn grid() -> Grid {
        Grid::new(8, 4)
    }

    #[test]
    fn sizes_are_clamped_to_sane_limits() {
        assert_eq!(Grid::new(0, 0).width(), MIN_SIZE);
        assert_eq!(Grid::new(-5, 3).height(), MIN_SIZE);
        assert_eq!(Grid::new(10_000, 10_000).width(), MAX_SIZE);
    }

    #[test]
    fn index_is_row_major() {
        let g = grid();
        let stride = g.width() + 2;
        assert_eq!(g.index(0, 0), (stride + 1) as usize);
        assert_eq!(g.index(1, 0), (stride + 2) as usize);
        assert_eq!(g.index(0, 1), (2 * stride + 1) as usize);
        assert_eq!(g.index(-1, -1), 0);
        assert_eq!(g.len(), (g.width() * g.height()) as usize);
    }

    #[test]
    fn border_is_wall_and_blocks_every_direction() {
        let g = grid();
        let (w, h) = (g.width(), g.height());
        for (x, y) in [
            (-1, 0),
            (0, -1),
            (w, 0),
            (0, h),
            (-1, h),
            (w, -1),
            (w, h),
            (-1, -1),
        ] {
            assert_eq!(g.element(x, y), BORDER_ELEMENT, "({x}, {y})");
        }
    }

    #[test]
    fn far_out_of_range_coordinates_are_safe() {
        let g = grid();
        assert_eq!(g.element(-10_000, -10_000), BORDER_ELEMENT);
        assert_eq!(g.element(10_000, 10_000), BORDER_ELEMENT);
        assert_eq!(g.element(3, 10_000), BORDER_ELEMENT);
    }

    #[test]
    fn first_and_last_interior_cells_are_empty() {
        let g = grid();
        let (w, h) = (g.width(), g.height());
        for (x, y) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
            assert_eq!(g.element(x, y), Element::Empty);
            assert!(g.in_bounds(x, y));
        }
        assert!(!g.in_bounds(-1, 0) && !g.in_bounds(w, 0) && !g.in_bounds(0, h));
    }

    #[test]
    fn set_and_swap() {
        let mut g = grid();
        g.set_element(2, 2, Element::Sand);
        g.set_element(3, 2, Element::Water);
        assert_eq!(g.element(2, 2), Element::Sand);
        g.swap(2, 2, 3, 2);
        assert_eq!(g.element(2, 2), Element::Water);
        assert_eq!(g.element(3, 2), Element::Sand);
    }

    #[test]
    fn replace_only_matches_the_expected_element() {
        let mut g = grid();
        g.set_element(1, 1, Element::Sodium);
        assert!(!g.replace(1, 1, Element::Water, Cell::empty()));
        assert_eq!(g.element(1, 1), Element::Sodium);
        assert!(g.replace(1, 1, Element::Sodium, Cell::empty()));
        assert_eq!(g.element(1, 1), Element::Empty);
    }

    #[test]
    fn clear_fills_interior_only() {
        let mut g = grid();
        g.clear(Element::Water);
        assert_eq!(g.count(Element::Water), g.len() as u32);
        assert_eq!(g.element(-1, 0), BORDER_ELEMENT);
        assert_eq!(g.element(g.width(), g.height()), BORDER_ELEMENT);
    }

    #[test]
    fn count_counts_only_the_interior() {
        let mut g = grid();
        assert_eq!(g.count(Element::Wall), 0);
        g.set_element(0, 0, Element::Sand);
        g.set_element(7, 3, Element::Sand);
        assert_eq!(g.count(Element::Sand), 2);
        assert_eq!(g.count(Element::Empty), g.len() as u32 - 2);
    }

    #[test]
    fn stamps_are_per_frame_and_cleared() {
        let mut g = grid();
        assert!(!g.is_marked(1, 1, 7));
        g.mark(1, 1, 7);
        assert!(g.is_marked(1, 1, 7));
        assert!(!g.is_marked(1, 1, 8));
        g.clear_stamps();
        assert!(!g.is_marked(1, 1, 7));
    }

    #[test]
    fn hash_tracks_the_state_buffer() {
        let mut g = grid();
        let before = g.hash();
        assert_eq!(before, g.hash());
        g.set_element(4, 2, Element::Sand);
        assert_ne!(before, g.hash());
        let mut other = grid();
        other.set_element(4, 2, Element::Sand);
        assert_eq!(g.hash(), other.hash(), "hash must be order independent");
        other.set(4, 2, Cell::raw(Element::Sand, 1, 0, 0, Element::Empty));
        assert_ne!(g.hash(), other.hash(), "variant must affect the hash");
    }

    #[test]
    fn coords_visits_every_cell_once() {
        let g = grid();
        let seen: Vec<(i32, i32)> = g.coords().collect();
        assert_eq!(seen.len(), g.len());
        let unique: std::collections::HashSet<(i32, i32)> = seen.iter().copied().collect();
        assert_eq!(unique.len(), seen.len());
    }
}

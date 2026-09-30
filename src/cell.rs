//! The single unit of state: one grid cell.
//!
//! The layout is deliberately small (6 bytes) and `Copy` so that the flat state
//! buffer stays cache friendly even at a few hundred thousand cells.

use crate::element::Element;

/// Reserved `variant` values. Everything else is free for future element
/// specific state.
pub const VARIANT_DEFAULT: u8 = 0;
/// Set on wettable elements (gunpowder, TNT) that have been touched by water.
pub const VARIANT_WET: u8 = 1;
/// Set on flames hot enough to be white (burning magnesium).
pub const VARIANT_HOT: u8 = 2;

/// One cell of the world.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    /// What the cell is made of.
    pub element: Element,
    /// Element specific bit of state (`VARIANT_WET`, `VARIANT_HOT`, ...).
    pub variant: u8,
    /// Remaining lifetime in ticks. `0` means "does not expire".
    pub life: u16,
    /// Vertical momentum, used for gravity acceleration and gas buoyancy.
    pub vel: i8,
    /// What this cell turns into when `life` runs out.
    pub residue: Element,
}

impl Default for Cell {
    fn default() -> Self {
        Self::empty()
    }
}

impl Cell {
    /// An empty (air) cell.
    #[inline]
    pub const fn empty() -> Self {
        Self {
            element: Element::Empty,
            variant: VARIANT_DEFAULT,
            life: 0,
            vel: 0,
            residue: Element::Empty,
        }
    }

    /// Fully explicit constructor.
    #[inline]
    pub const fn raw(element: Element, variant: u8, life: u16, vel: i8, residue: Element) -> Self {
        Self {
            element,
            variant,
            life,
            vel,
            residue,
        }
    }

    /// Cell with the element's default lifetime and residue.
    #[inline]
    pub fn new(element: Element) -> Self {
        let props = element.props();
        Self {
            element,
            variant: VARIANT_DEFAULT,
            life: props.life,
            vel: 0,
            residue: props.residue,
        }
    }

    /// A transient cell (flame, smoke, steam, spark) that turns into `residue`
    /// after `life` ticks.
    #[inline]
    pub fn transient(element: Element, life: u16, residue: Element) -> Self {
        Self {
            element,
            variant: VARIANT_DEFAULT,
            life,
            vel: 0,
            residue,
        }
    }

    /// True when this cell is made of `element`.
    #[inline]
    pub fn is(&self, element: Element) -> bool {
        self.element == element
    }

    /// True when the cell is air.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.element == Element::Empty
    }

    /// True when the cell has been wetted (gunpowder, TNT).
    #[inline]
    pub fn is_wet(&self) -> bool {
        self.variant == VARIANT_WET
    }

    /// Moves the cell into its `residue` and returns `true` when it expired.
    ///
    /// Transient elements (fire, smoke, steam) use this to turn into their
    /// residue once their lifetime runs out. Immortal elements (`life == 0`)
    /// are never touched.
    #[inline]
    pub fn tick_lifetime(&mut self) -> bool {
        if self.life == 0 {
            return false;
        }
        self.life -= 1;
        if self.life == 0 {
            let residue = self.residue;
            *self = Self::new(residue);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_is_compact() {
        assert!(std::mem::size_of::<Cell>() <= 8, "Cell grew too large");
    }

    #[test]
    fn new_uses_element_defaults() {
        let fire = Cell::new(Element::Fire);
        assert_eq!(fire.life, Element::Fire.props().life);
        assert_eq!(fire.residue, Element::Fire.props().residue);
        assert_eq!(fire.variant, VARIANT_DEFAULT);
    }

    #[test]
    fn lifetime_expires_into_residue() {
        let mut steam = Cell::transient(Element::Steam, 3, Element::Water);
        assert!(!steam.tick_lifetime());
        assert!(!steam.tick_lifetime());
        assert!(steam.tick_lifetime());
        assert_eq!(steam.element, Element::Water);
    }

    #[test]
    fn immortal_cells_never_expire() {
        let mut sand = Cell::new(Element::Sand);
        for _ in 0..1000 {
            assert!(!sand.tick_lifetime());
        }
        assert_eq!(sand.element, Element::Sand);
    }

    #[test]
    fn empty_cell_is_default() {
        assert_eq!(Cell::default(), Cell::empty());
        assert!(Cell::empty().is_empty());
    }

    #[test]
    fn wet_flag_round_trips() {
        let mut powder = Cell::new(Element::Gunpowder);
        assert!(!powder.is_wet());
        powder.variant = VARIANT_WET;
        assert!(powder.is_wet());
    }
}

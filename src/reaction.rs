//! Chemistry: a table of reaction rules, applied to neighbouring cells.
//!
//! Rules are declarative. Add a row to `REACTIONS` and the pair is wired up in
//! both directions automatically, the outcome is applied through the same
//! consume-check used by everything else (so two reactions cannot eat the same
//! cell in one tick) and the equation shows up in the UI log for free.

use std::sync::OnceLock;

use crate::cell::{Cell, VARIANT_WET};
use crate::element::{Element, ELEMENT_COUNT};
use crate::physics::NEIGHBOURS8;
use crate::world::World;

/// What happens to one side of a reaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Leave it alone.
    Keep,
    /// Consume it, leaving air.
    Empty,
    /// Turn it into another element.
    Become(Element),
    /// Spend `n` units of the cell's lifetime (acid strength). When the lifetime
    /// runs out the cell becomes its residue.
    Weaken(u8),
    /// Soak the cell; only has an effect on `wettable` elements.
    Wet,
}

/// Extra side effect of a reaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Nothing beyond the outcomes.
    None,
    /// Detonate at the first cell, releasing a puff of `gas`.
    Explode { radius: u8, gas: Element },
}

/// One rule: `a` and `b` are neighbours, `chance` is out of 255 per tick.
pub struct Reaction {
    /// The element that triggers the rule.
    pub a: Element,
    /// The neighbouring element it reacts with.
    pub b: Element,
    /// Probability out of 255 per tick.
    pub chance: u8,
    /// What becomes of `a`.
    pub outcome_a: Outcome,
    /// What becomes of `b`.
    pub outcome_b: Outcome,
    /// Detonation, if any.
    pub effect: Effect,
    /// Balanced equation shown in the reaction log.
    pub equation: &'static str,
}

/// A simple rule with no side effects.
const fn rule(
    a: Element,
    b: Element,
    chance: u8,
    outcome_a: Outcome,
    outcome_b: Outcome,
    equation: &'static str,
) -> Reaction {
    Reaction {
        a,
        b,
        chance,
        outcome_a,
        outcome_b,
        effect: Effect::None,
        equation,
    }
}

/// A rule that detonates and releases `gas`.
#[allow(clippy::too_many_arguments)]
const fn boom(
    a: Element,
    b: Element,
    chance: u8,
    radius: u8,
    gas: Element,
    outcome_a: Outcome,
    outcome_b: Outcome,
    equation: &'static str,
) -> Reaction {
    Reaction {
        a,
        b,
        chance,
        outcome_a,
        outcome_b,
        effect: Effect::Explode { radius, gas },
        equation,
    }
}

/// Every rule in the lab. Order does not matter: the pair lookup is built from
/// this list at first use.
pub const REACTIONS: &[Reaction] = &[
    // --- Alkali and alkaline earth metals in water ---------------------------
    // 2 Na + 2 H2O -> 2 NaOH + H2. The classic demonstration: the metal races
    // over the surface, throws out hydrogen and detonates.
    boom(
        Element::Sodium,
        Element::Water,
        128,
        6,
        Element::Hydrogen,
        Outcome::Empty,
        Outcome::Become(Element::SaltWater),
        "2 Na + 2 H2O -> 2 NaOH + H2  (EXPLOSION)",
    ),
    // Potassium is the same reaction, one shell further down the group.
    boom(
        Element::Potassium,
        Element::Water,
        255,
        9,
        Element::Hydrogen,
        Outcome::Empty,
        Outcome::Become(Element::SaltWater),
        "2 K + 2 H2O -> 2 KOH + H2  (violent!)",
    ),
    // Lithium is mild: it dissolves and fizzes hydrogen without catching fire.
    rule(
        Element::Lithium,
        Element::Water,
        10,
        Outcome::Empty,
        Outcome::Keep,
        "2 Li + 2 H2O -> 2 LiOH + H2  (gentle fizz)",
    ),
    rule(
        Element::Lithium,
        Element::Water,
        55,
        Outcome::Keep,
        Outcome::Become(Element::Hydrogen),
        "2 Li + 2 H2O -> 2 LiOH + H2  (hydrogen bubbles)",
    ),
    rule(
        Element::Calcium,
        Element::Water,
        10,
        Outcome::Become(Element::Lime),
        Outcome::Keep,
        "Ca + 2 H2O -> Ca(OH)2 + H2  (slaked lime)",
    ),
    rule(
        Element::Calcium,
        Element::Water,
        30,
        Outcome::Keep,
        Outcome::Become(Element::Hydrogen),
        "Ca + 2 H2O -> Ca(OH)2 + H2  (bubbles)",
    ),
    rule(
        Element::Magnesium,
        Element::Water,
        2,
        Outcome::Become(Element::Lime),
        Outcome::Keep,
        "Mg + 2 H2O -> Mg(OH)2 + H2  (barely reacts)",
    ),
    rule(
        Element::Magnesium,
        Element::Water,
        4,
        Outcome::Keep,
        Outcome::Become(Element::Hydrogen),
        "Mg + 2 H2O -> Mg(OH)2 + H2  (barely reacts)",
    ),
    // Carbide + water: the carbide cannon trick. The solid dissolves quickly and
    // each drop of water it touches turns into a bubble of fuel gas.
    rule(
        Element::Carbide,
        Element::Water,
        60,
        Outcome::Become(Element::Lime),
        Outcome::Keep,
        "CaC2 + 2 H2O -> Ca(OH)2 + C2H2",
    ),
    rule(
        Element::Carbide,
        Element::Water,
        30,
        Outcome::Keep,
        Outcome::Become(Element::Acetylene),
        "CaC2 + 2 H2O -> Ca(OH)2 + C2H2  (fuel gas)",
    ),
    // --- Metals in acid ------------------------------------------------------
    boom(
        Element::Sodium,
        Element::Acid,
        255,
        8,
        Element::Hydrogen,
        Outcome::Empty,
        Outcome::Become(Element::Hydrogen),
        "2 Na + 2 HCl -> 2 NaCl + H2  (violent!)",
    ),
    boom(
        Element::Potassium,
        Element::Acid,
        255,
        10,
        Element::Hydrogen,
        Outcome::Empty,
        Outcome::Become(Element::Hydrogen),
        "2 K + 2 HCl -> 2 KCl + H2  (violent!)",
    ),
    rule(
        Element::Acid,
        Element::Metal,
        45,
        Outcome::Keep,
        Outcome::Empty,
        "Fe + 2 HCl -> FeCl2 + H2  (steel dissolved)",
    ),
    rule(
        Element::Acid,
        Element::Metal,
        20,
        Outcome::Weaken(3),
        Outcome::Keep,
        "acid spent: Fe + 2 HCl -> FeCl2 + H2",
    ),
    rule(
        Element::Acid,
        Element::Metal,
        6,
        Outcome::Become(Element::Hydrogen),
        Outcome::Keep,
        "Fe + 2 HCl -> FeCl2 + H2  (fizzing)",
    ),
    // --- Acid on stone, timber and tissue ------------------------------------
    rule(
        Element::Acid,
        Element::Stone,
        30,
        Outcome::Keep,
        Outcome::Empty,
        "CaCO3 + 2 HCl -> CaCl2 + H2O + CO2  (stone etched)",
    ),
    rule(
        Element::Acid,
        Element::Stone,
        15,
        Outcome::Weaken(2),
        Outcome::Keep,
        "acid spent: CaCO3 + 2 HCl -> CaCl2 + H2O + CO2",
    ),
    rule(
        Element::Acid,
        Element::Wood,
        8,
        Outcome::Weaken(2),
        Outcome::Become(Element::Ash),
        "acid chars timber (cellulose -> carbon)",
    ),
    rule(
        Element::Acid,
        Element::Plant,
        25,
        Outcome::Weaken(2),
        Outcome::Empty,
        "acid dissolves tissue",
    ),
    rule(
        Element::Acid,
        Element::Lime,
        40,
        Outcome::Weaken(2),
        Outcome::Empty,
        "Ca(OH)2 + 2 HCl -> CaCl2 + 2 H2O",
    ),
    rule(
        Element::Acid,
        Element::Water,
        6,
        Outcome::Become(Element::Water),
        Outcome::Keep,
        "acid diluted: HCl + H2O -> (dilute)",
    ),
    // --- Lava: quenched by water, melts and fuses things ---------------------
    rule(
        Element::Lava,
        Element::Water,
        255,
        Outcome::Become(Element::Stone),
        Outcome::Become(Element::Steam),
        "lava + H2O -> stone + steam  (quenched)",
    ),
    rule(
        Element::Lava,
        Element::Ice,
        200,
        Outcome::Keep,
        Outcome::Become(Element::Water),
        "ice melts on lava",
    ),
    rule(
        Element::Lava,
        Element::Ice,
        40,
        Outcome::Become(Element::Stone),
        Outcome::Keep,
        "lava chilled by ice -> stone",
    ),
    rule(
        Element::Lava,
        Element::Snow,
        255,
        Outcome::Keep,
        Outcome::Become(Element::Water),
        "snow melts on lava",
    ),
    rule(
        Element::Lava,
        Element::Sand,
        30,
        Outcome::Keep,
        Outcome::Become(Element::Glass),
        "SiO2 (sand) + heat -> glass",
    ),
    rule(
        Element::Lava,
        Element::Stone,
        4,
        Outcome::Keep,
        Outcome::Become(Element::Lava),
        "stone melts into lava",
    ),
    rule(
        Element::Lava,
        Element::Metal,
        8,
        Outcome::Keep,
        Outcome::Become(Element::Lava),
        "steel melts into lava",
    ),
    // --- Fire and water ------------------------------------------------------
    rule(
        Element::Fire,
        Element::Water,
        255,
        Outcome::Empty,
        Outcome::Keep,
        "H2O extinguishes the flame",
    ),
    rule(
        Element::Fire,
        Element::Water,
        90,
        Outcome::Keep,
        Outcome::Become(Element::Steam),
        "H2O + heat -> steam",
    ),
    rule(
        Element::Fire,
        Element::Ice,
        200,
        Outcome::Keep,
        Outcome::Become(Element::Water),
        "ice melts in the flame",
    ),
    rule(
        Element::Fire,
        Element::Snow,
        200,
        Outcome::Keep,
        Outcome::Become(Element::Water),
        "snow melts in the flame",
    ),
    // --- Soluble salts, snow and dampening -----------------------------------
    rule(
        Element::Salt,
        Element::Water,
        30,
        Outcome::Empty,
        Outcome::Become(Element::SaltWater),
        "NaCl(s) -> Na+ + Cl-  (dissolved)",
    ),
    rule(
        Element::Salt,
        Element::Ice,
        60,
        Outcome::Keep,
        Outcome::Become(Element::Water),
        "salt melts ice (de-icing)",
    ),
    rule(
        Element::Snow,
        Element::Water,
        30,
        Outcome::Become(Element::Water),
        Outcome::Keep,
        "snow melts in water",
    ),
    rule(
        Element::Gunpowder,
        Element::Water,
        128,
        Outcome::Wet,
        Outcome::Keep,
        "gunpowder dampened -> inert",
    ),
    rule(
        Element::Tnt,
        Element::Water,
        128,
        Outcome::Wet,
        Outcome::Keep,
        "TNT dampened -> inert",
    ),
    // --- Plants and lime -----------------------------------------------------
    rule(
        Element::Plant,
        Element::Water,
        4,
        Outcome::Keep,
        Outcome::Empty,
        "roots drink the water (transpiration)",
    ),
    rule(
        Element::Lime,
        Element::Water,
        20,
        Outcome::Keep,
        Outcome::Become(Element::Steam),
        "CaO + H2O -> Ca(OH)2 + heat  (slaking)",
    ),
    rule(
        Element::Lime,
        Element::Water,
        8,
        Outcome::Become(Element::Stone),
        Outcome::Keep,
        "slaked lime sets hard into stone",
    ),
];

/// Lazily built pair lookup: `slot[a * ELEMENT_COUNT + b]` lists every rule that
/// fires when `a` sits next to `b`, with a flag telling us the rule was
/// registered the other way round.
struct Lookup {
    slots: Vec<Vec<u16>>,
}

/// Marks a lookup entry as "the rule is stored with a and b the other way round".
const SWAPPED: u16 = 0x8000;

impl Lookup {
    fn build() -> Self {
        let mut slots = vec![Vec::new(); ELEMENT_COUNT * ELEMENT_COUNT];
        for (index, reaction) in REACTIONS.iter().enumerate() {
            let index = index as u16;
            slots[slot_of(reaction.a, reaction.b)].push(index);
            slots[slot_of(reaction.b, reaction.a)].push(index | SWAPPED);
        }
        Self { slots }
    }

    #[inline]
    fn for_pair(&self, a: Element, b: Element) -> &[u16] {
        &self.slots[slot_of(a, b)]
    }
}

#[inline]
fn slot_of(a: Element, b: Element) -> usize {
    usize::from(a.id()) * ELEMENT_COUNT + usize::from(b.id())
}

fn lookup() -> &'static Lookup {
    static LOOKUP: OnceLock<Lookup> = OnceLock::new();
    LOOKUP.get_or_init(Lookup::build)
}

/// Tries every rule that applies to the cell at `(x, y)`.
///
/// Returns `true` when the cell itself changed, in which case the caller skips
/// its movement phase for this tick - the new element gets a clean tick next
/// frame instead of inheriting the old one's momentum.
pub fn react(world: &mut World, x: i32, y: i32, element: Element) -> bool {
    if element == Element::Empty {
        return false;
    }
    let table = lookup();
    let mut self_changed = false;

    for (dx, dy) in NEIGHBOURS8 {
        let (nx, ny) = (x + dx, y + dy);
        let other = world.grid.element(nx, ny);
        if other == Element::Empty {
            continue;
        }
        for &entry in table.for_pair(element, other) {
            let reaction = &REACTIONS[usize::from(entry & !SWAPPED)];
            if !world.rng.chance(reaction.chance) {
                continue;
            }
            let swapped = entry & SWAPPED != 0;
            let a_pos = if swapped { (nx, ny) } else { (x, y) };
            let b_pos = if swapped { (x, y) } else { (nx, ny) };

            // Consumption guard: both cells must still be what the rule expects,
            // otherwise an earlier reaction in this same tick already ate one.
            if world.grid.element(a_pos.0, a_pos.1) != reaction.a
                || world.grid.element(b_pos.0, b_pos.1) != reaction.b
            {
                continue;
            }

            let a_changed = apply(world, a_pos, reaction.a, reaction.outcome_a);
            let b_changed = apply(world, b_pos, reaction.b, reaction.outcome_b);

            if let Effect::Explode { radius, gas } = reaction.effect {
                world.explode(a_pos.0, a_pos.1, i32::from(radius), gas);
            }

            let fired = a_changed || b_changed || reaction.effect != Effect::None;
            if fired {
                world.log_reaction(reaction.equation);
            }
            if if swapped { b_changed } else { a_changed } {
                self_changed = true;
            }
        }
    }

    self_changed
}

/// Applies one outcome. Returns `true` when the cell became a different element.
fn apply(world: &mut World, pos: (i32, i32), expected: Element, outcome: Outcome) -> bool {
    let (x, y) = pos;
    let cell = world.grid.cell(x, y);
    if cell.element != expected {
        return false;
    }
    match outcome {
        Outcome::Keep => false,
        Outcome::Empty => {
            world.grid.set(x, y, Cell::empty());
            true
        }
        Outcome::Become(element) => {
            world.grid.set(x, y, Cell::new(element));
            true
        }
        Outcome::Wet => {
            if cell.element.props().wettable && cell.variant != VARIANT_WET {
                world.grid.set(
                    x,
                    y,
                    Cell::raw(cell.element, VARIANT_WET, cell.life, cell.vel, cell.residue),
                );
                true
            } else {
                false
            }
        }
        Outcome::Weaken(amount) => {
            let mut cell = cell;
            cell.life = cell.life.saturating_sub(u16::from(amount));
            if cell.life == 0 {
                // Spent acid reverts to its residue (water).
                world.grid.set(x, y, Cell::new(cell.residue));
            } else {
                world.grid.set(x, y, cell);
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lab() -> World {
        World::new(24, 24, 2024)
    }

    fn put(world: &mut World, x: i32, y: i32, element: Element) {
        world.grid_mut().set_element(x, y, element);
    }

    #[test]
    fn table_is_well_formed() {
        assert!(REACTIONS.len() >= 30, "the lab feels empty");
        for reaction in REACTIONS {
            assert_ne!(reaction.a, reaction.b, "a rule reacts with itself");
            assert!(reaction.chance > 0, "a rule can never fire");
            assert!(!reaction.equation.is_empty());
            // Equations are embedded in JSON for the UI log.
            assert!(!reaction.equation.contains('"'), "{}", reaction.equation);
            assert!(!reaction.equation.contains('\\'), "{}", reaction.equation);
            assert!(
                reaction.outcome_a != Outcome::Keep
                    || reaction.outcome_b != Outcome::Keep
                    || reaction.effect != Effect::None,
                "{} does nothing at all",
                reaction.equation
            );
        }
    }

    #[test]
    fn lookup_finds_rules_in_both_directions() {
        let table = lookup();
        for (index, reaction) in REACTIONS.iter().enumerate() {
            let index = index as u16;
            assert!(
                table.for_pair(reaction.a, reaction.b).contains(&index),
                "{} missing forward",
                reaction.equation
            );
            assert!(
                table
                    .for_pair(reaction.b, reaction.a)
                    .contains(&(index | SWAPPED)),
                "{} missing reversed",
                reaction.equation
            );
        }
        assert!(table.for_pair(Element::Sand, Element::Sand).is_empty());
        assert!(table.for_pair(Element::Mercury, Element::Glass).is_empty());
    }

    #[test]
    fn every_element_has_a_lookup_slot() {
        let table = lookup();
        for a in Element::ALL {
            for b in Element::ALL {
                // Must not panic and must stay inside the table.
                let _ = table.for_pair(a, b);
            }
        }
        assert_eq!(table.slots.len(), ELEMENT_COUNT * ELEMENT_COUNT);
    }

    #[test]
    fn wetting_soaks_the_cell_and_marks_it() {
        let mut world = lab();
        put(&mut world, 5, 5, Element::Gunpowder);
        assert!(apply(&mut world, (5, 5), Element::Gunpowder, Outcome::Wet));
        let cell = world.grid().cell(5, 5);
        assert!(cell.is_wet());
        assert_eq!(cell.element, Element::Gunpowder);
        // Idempotent: a wet cell does not change again.
        assert!(!apply(&mut world, (5, 5), Element::Gunpowder, Outcome::Wet));
    }

    #[test]
    fn sand_ignores_wetting() {
        let mut world = lab();
        put(&mut world, 3, 3, Element::Sand);
        assert!(!apply(&mut world, (3, 3), Element::Sand, Outcome::Wet));
        assert!(!world.grid().cell(3, 3).is_wet());
    }

    #[test]
    fn weakening_spends_acid_into_water() {
        let mut world = lab();
        put(&mut world, 4, 4, Element::Acid);
        let start = world.grid().cell(4, 4).life;
        assert!(!apply(
            &mut world,
            (4, 4),
            Element::Acid,
            Outcome::Weaken(10)
        ));
        assert_eq!(world.grid().cell(4, 4).life, start - 10);

        world
            .grid_mut()
            .set(4, 4, Cell::raw(Element::Acid, 0, 4, 0, Element::Water));
        apply(&mut world, (4, 4), Element::Acid, Outcome::Weaken(4));
        assert_eq!(world.element(4, 4), Element::Water, "spent acid is water");
    }

    #[test]
    fn apply_respects_the_consumption_guard() {
        let mut world = lab();
        put(&mut world, 6, 6, Element::Water);
        // Pretend another reaction already consumed the expected element.
        assert!(!apply(&mut world, (6, 6), Element::Sodium, Outcome::Empty));
        assert_eq!(world.element(6, 6), Element::Water);
    }

    #[test]
    fn sodium_meets_water_and_detonates() {
        let mut world = lab();
        for x in 6..18 {
            for y in 8..14 {
                put(&mut world, x, y, Element::Water);
            }
        }
        put(&mut world, 12, 7, Element::Sodium);
        put(&mut world, 12, 8, Element::Sodium);

        world.step_n(240);

        assert_eq!(
            world.count(Element::Sodium),
            0,
            "the sodium must be consumed"
        );
        let log = world.reactions_json();
        assert!(log.contains("EXPLOSION"), "log was {log}");
        assert!(
            world.count(Element::Empty) > 0,
            "the blast must clear space"
        );
    }

    #[test]
    fn potassium_and_lithium_both_dissolve_in_water() {
        let mut world = lab();
        for x in 4..20 {
            put(&mut world, x, 12, Element::Water);
            put(&mut world, x, 13, Element::Water);
        }
        put(&mut world, 10, 11, Element::Potassium);
        put(&mut world, 14, 11, Element::Lithium);

        // Hydrogen is released and then burns off again, so watch for it while
        // the reaction happens instead of only looking at the end state.
        let mut saw_gas = false;
        for _ in 0..400 {
            world.step();
            if world.count(Element::Hydrogen) > 0 || world.count(Element::Steam) > 0 {
                saw_gas = true;
            }
        }

        assert!(saw_gas, "both metals release hydrogen or steam");
        assert_eq!(world.count(Element::Potassium), 0);
        assert_eq!(world.count(Element::Lithium), 0);
    }

    #[test]
    fn lava_and_water_make_stone_and_steam() {
        let mut world = lab();
        put(&mut world, 10, 10, Element::Lava);
        put(&mut world, 10, 11, Element::Water);
        world.step_n(30);
        assert!(world.count(Element::Steam) > 0 || world.count(Element::Water) == 0);
        assert!(
            world.count(Element::Stone) >= 1,
            "quenched lava becomes stone"
        );
    }

    #[test]
    fn acid_dissolves_metal() {
        let mut world = lab();
        for y in 8..14 {
            put(&mut world, 8, y, Element::Metal);
            put(&mut world, 9, y, Element::Acid);
        }
        let before = world.count(Element::Metal);
        world.step_n(300);
        assert!(
            world.count(Element::Metal) < before,
            "acid should have eaten into the steel"
        );
    }

    #[test]
    fn fire_is_extinguished_by_water() {
        let mut world = lab();
        put(&mut world, 10, 10, Element::Fire);
        put(&mut world, 10, 11, Element::Water);
        world.step_n(6);
        assert_eq!(world.count(Element::Fire), 0, "water must kill the flame");
    }

    #[test]
    fn dry_gunpowder_burns_but_wet_gunpowder_does_not() {
        let mut world = lab();
        // Dry: a flame next to it detonates.
        put(&mut world, 8, 8, Element::Gunpowder);
        put(&mut world, 8, 9, Element::Fire);
        world.step_n(4);
        assert_eq!(world.count(Element::Gunpowder), 0, "dry powder ignites");

        // Wet: the same flame cannot touch it.
        let mut soggy = lab();
        soggy.grid_mut().set(
            8,
            8,
            Cell::raw(
                Element::Gunpowder,
                crate::cell::VARIANT_WET,
                0,
                0,
                Element::Smoke,
            ),
        );
        soggy.grid_mut().set_element(8, 9, Element::Fire);
        soggy.grid_mut().set_element(8, 10, Element::Stone);
        soggy.step_n(40);
        assert_eq!(soggy.count(Element::Gunpowder), 1, "wet powder is inert");
    }

    #[test]
    fn salt_dissolves_into_brine_and_melts_ice() {
        let mut world = lab();
        put(&mut world, 5, 5, Element::Salt);
        put(&mut world, 5, 6, Element::Water);
        put(&mut world, 10, 10, Element::Salt);
        put(&mut world, 10, 11, Element::Ice);
        world.step_n(60);
        assert!(world.count(Element::SaltWater) > 0, "should form brine");
        assert_eq!(world.count(Element::Ice), 0, "salt de-ices");
    }

    #[test]
    fn carbide_fizzes_acetylene() {
        let mut world = lab();
        // A pool of water with a lump of calcium carbide dropped into it.
        for x in 5..11 {
            for y in 18..24 {
                put(&mut world, x, y, Element::Water);
            }
        }
        put(&mut world, 7, 14, Element::Carbide);

        let mut saw_gas = false;
        for _ in 0..120 {
            world.step();
            if world.count(Element::Acetylene) > 0 {
                saw_gas = true;
                break;
            }
        }

        assert!(saw_gas, "carbide should release acetylene bubbles");
        assert_eq!(world.count(Element::Carbide), 0, "the carbide is consumed");
        assert!(
            world.count(Element::Lime) > 0,
            "it leaves slaked lime behind"
        );
    }

    #[test]
    fn plants_grow_when_they_have_water() {
        let mut world = lab();
        // The shoot sits on the floor with its roots in the puddle.
        put(&mut world, 12, 22, Element::Plant);
        for x in 10..15 {
            put(&mut world, x, 23, Element::Water);
        }
        world.step_n(900);
        assert!(
            world.count(Element::Plant) > 1,
            "the plant should have grown with water available"
        );
    }
}

//! Movement rules, one function per behavioural category, plus ignition.
//!
//! Everything here works on the flat grid through `World`'s helpers, which stamp
//! both ends of every move. That single detail is what keeps the simulation
//! correct: a cell that is still inside its "already updated" stamp for the
//! current frame is skipped, so nothing can move twice in one tick no matter
//! which direction it travels.

use crate::cell::Cell;
use crate::element::{Category, Element};
use crate::world::{World, TERMINAL_FALL, TERMINAL_RISE};

/// Eight-neighbour offsets (Moore neighbourhood): chemistry and plant growth.
pub const NEIGHBOURS8: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Four-neighbour offsets (von Neumann): used where diagonal contact would be
/// too eager, like heat transfer.
pub const NEIGHBOURS4: [(i32, i32); 4] = [(0, -1), (-1, 0), (1, 0), (0, 1)];

/// Can `mover`, moving **down**, push `target` out of the way?
///
/// Air gives way to everything, fluids only give way to something strictly
/// heavier (equal densities would churn forever) and solids never budge. This one
/// rule gives us sand sinking in water, oil floating on water and mercury
/// sinking through both.
#[inline]
pub fn can_sink(mover: Element, target: Element) -> bool {
    if target == Element::Wall || mover == Element::Wall {
        return false;
    }
    match target.category() {
        Category::Empty => true,
        Category::Liquid | Category::Gas | Category::Fire | Category::Powder => {
            mover.density() > target.density()
        }
        Category::Solid => false,
    }
}

/// Can `mover`, moving **up**, pass through `target`?
///
/// Gases are lighter than the fluids above them, so they bubble through: that is
/// how hydrogen from calcium in water, or steam from lava, reaches the surface.
#[inline]
pub fn can_rise(mover: Element, target: Element) -> bool {
    if target == Element::Wall || mover == Element::Wall {
        return false;
    }
    match target.category() {
        Category::Empty => true,
        Category::Liquid | Category::Gas | Category::Fire => mover.density() < target.density(),
        Category::Powder | Category::Solid => false,
    }
}

/// How far a falling cell travels this tick: one cell, plus one more for every
/// six ticks of accumulated momentum, capped at terminal velocity.
#[inline]
pub fn fall_distance(vel: i8, max: i32) -> i32 {
    (1 + i32::from(vel) / 6).min(max)
}

/// How far a rising cell travels this tick, based on the element's buoyancy.
#[inline]
pub fn climb_distance(vel: i8, base: u8) -> i32 {
    let base = i32::from(base).max(1);
    (base + i32::from(vel >= 8)).min(TERMINAL_RISE)
}

/// A fair coin flip between "left" and "right", so piles and plumes grow evenly
/// instead of always leaning one way.
#[inline]
fn random_side(world: &mut World) -> i32 {
    if world.rng.flip() {
        1
    } else {
        -1
    }
}

/// Powders: fall straight down (accelerating), then slide down the diagonals.
/// That 45 degree slide is what makes sand form piles.
pub fn step_powder(world: &mut World, x: i32, y: i32) {
    let cell = world.grid.cell(x, y);

    let distance = fall_distance(cell.vel, TERMINAL_FALL);
    let mut steps = 0;
    while steps < distance && can_sink(cell.element, world.grid.element(x, y + steps + 1)) {
        steps += 1;
    }
    if steps > 0 {
        world.move_with_momentum((x, y), (x, y + steps));
        return;
    }

    let first = random_side(world);
    for side in [first, -first] {
        if can_sink(cell.element, world.grid.element(x + side, y + 1)) {
            world.move_cell((x, y), (x + side, y + 1));
            world.stop_momentum(x + side, y + 1);
            return;
        }
    }

    world.stop_momentum(x, y);
}

/// Liquids: fall, then slide diagonally, then flow sideways towards a hole, then
/// level out one cell at a time. The search radius is the element's `dispersion`,
/// which is why a puddle rushes into a pit but only creeps across flat ground.
pub fn step_liquid(world: &mut World, x: i32, y: i32) {
    let cell = world.grid.cell(x, y);
    let dispersion = i32::from(cell.element.props().dispersion).max(1);

    let distance = fall_distance(cell.vel, 2);
    let mut steps = 0;
    while steps < distance && can_sink(cell.element, world.grid.element(x, y + steps + 1)) {
        steps += 1;
    }
    if steps > 0 {
        world.move_with_momentum((x, y), (x, y + steps));
        return;
    }

    let first = random_side(world);
    for side in [first, -first] {
        if can_sink(cell.element, world.grid.element(x + side, y + 1)) {
            world.move_cell((x, y), (x + side, y + 1));
            world.stop_momentum(x + side, y + 1);
            return;
        }
    }

    // Flow towards the nearest place where this liquid could fall. Breaking out
    // of the loop early means a blocked path stops the search, so liquids cannot
    // tunnel through walls.
    for side in [first, -first] {
        for d in 1..=dispersion {
            let (nx, ny) = (x + side * d, y);
            if world.grid.element(nx, ny) != Element::Empty {
                break;
            }
            if can_sink(cell.element, world.grid.element(nx, ny + 1)) {
                world.move_cell((x, y), (nx, ny));
                world.stop_momentum(nx, ny);
                return;
            }
        }
    }

    for side in [first, -first] {
        if world.grid.element(x + side, y) == Element::Empty {
            world.move_cell((x, y), (x + side, y));
            world.stop_momentum(x + side, y);
            return;
        }
    }

    world.stop_momentum(x, y);
}

/// Gases: rise (accelerating), bubble through liquids, then drift sideways so
/// plumes spread out under a ceiling instead of stacking into a column.
pub fn step_gas(world: &mut World, x: i32, y: i32) {
    let cell = world.grid.cell(x, y);
    let distance = climb_distance(cell.vel, cell.element.props().rise);

    let mut steps = 0;
    while steps < distance && can_rise(cell.element, world.grid.element(x, y - steps - 1)) {
        steps += 1;
    }
    if steps > 0 {
        world.move_with_momentum((x, y), (x, y - steps));
        return;
    }

    let first = random_side(world);
    for side in [first, -first] {
        if can_rise(cell.element, world.grid.element(x + side, y - 1)) {
            world.move_cell((x, y), (x + side, y - 1));
            world.stop_momentum(x + side, y - 1);
            return;
        }
    }

    // Drift sideways so a plume spreads under a ceiling instead of stacking into
    // a column. Gases may slide a few cells while they have room.
    let drift = i32::from(cell.element.props().dispersion).max(1);
    for side in [first, -first] {
        if !world.rng.chance(150) {
            continue;
        }
        let mut reach = 0;
        for d in 1..=drift {
            if world.grid.element(x + side * d, y) != Element::Empty {
                break;
            }
            reach = d;
        }
        if reach > 0 {
            let target = (x + side * reach, y);
            world.move_cell((x, y), target);
            world.stop_momentum(target.0, target.1);
            return;
        }
    }

    world.stop_momentum(x, y);
}

/// Fire: a flame that still has fuel next to it stays put and eats through it;
/// only flames with nothing left to burn climb away.
///
/// Without this rule a flame would float off its fuel before it had a chance to
/// light it, which made "paint fire onto wood" work only by luck.
pub fn step_fire(world: &mut World, x: i32, y: i32) {
    if flames_are_fed(world, x, y) {
        return;
    }

    if world.frame % 2 == 0 && can_rise(Element::Fire, world.grid.element(x, y - 1)) {
        world.move_cell((x, y), (x, y - 1));
        return;
    }

    let first = random_side(world);
    for side in [first, -first] {
        if can_rise(Element::Fire, world.grid.element(x + side, y - 1)) {
            world.move_cell((x, y), (x + side, y - 1));
            return;
        }
    }

    for side in [first, -first] {
        if world.grid.element(x + side, y) == Element::Empty && world.rng.chance(70) {
            world.move_cell((x, y), (x + side, y));
            return;
        }
    }
}

/// Is there anything left for this flame to burn? Wet fuel does not count.
fn flames_are_fed(world: &World, x: i32, y: i32) -> bool {
    for (dx, dy) in NEIGHBOURS8 {
        let neighbour = world.grid.cell(x + dx, y + dy);
        if neighbour.element.props().flammable > 0 && !neighbour.is_wet() {
            return true;
        }
    }
    false
}

/// Solids stay put, except plants, which grow towards water.
pub fn step_solid(world: &mut World, x: i32, y: i32) {
    if world.grid.element(x, y) == Element::Plant {
        grow_plant(world, x, y);
    }
}

/// Plants creep upwards when a water cell is in reach. Growth is throttled by a
/// low chance and by crowding, so a vine spreads instead of filling the screen.
fn grow_plant(world: &mut World, x: i32, y: i32) {
    if world.grid.element(x, y - 1) != Element::Empty || !world.rng.chance(12) {
        return;
    }
    let mut water = false;
    let mut neighbours = 0;
    for (dx, dy) in NEIGHBOURS8 {
        match world.grid.element(x + dx, y + dy) {
            Element::Water => water = true,
            Element::Plant => neighbours += 1,
            _ => {}
        }
    }
    if !water || neighbours > 4 {
        return;
    }
    world.grid.set_element(x, y - 1, Element::Plant);
    world.grid.mark(x, y - 1, world.frame);
    world.log_reaction("6 CO2 + 6 H2O + light -> C6H12O6 + 6 O2  (growth)");
}

/// Heat transfer. Anything flammable next to a heat source may catch fire;
/// explosive materials detonate instead of burning, and wet material is inert.
pub fn ignite_neighbours(world: &mut World, x: i32, y: i32, source: Element) {
    for (dx, dy) in NEIGHBOURS4 {
        let (nx, ny) = (x + dx, y + dy);
        let target = world.grid.cell(nx, ny);
        let props = target.element.props();
        if props.flammable == 0 || target.is_wet() || target.element == source {
            continue;
        }
        if !world.rng.chance(props.flammable) {
            continue;
        }
        if props.explosive > 0 {
            world.log_reaction(match target.element {
                Element::Tnt => "detonation: C7H5N3O6 -> CO2 + H2O + N2  (huge bang)",
                Element::Gunpowder => "detonation: 2 KNO3 + S + 3 C -> K2S + N2 + 3 CO2",
                Element::Hydrogen => "detonation: 2 H2 + O2 -> 2 H2O  (big bang)",
                _ => "detonation: 2 C2H2 + 5 O2 -> 4 CO2 + 2 H2O",
            });
            world.explode(nx, ny, i32::from(props.explosive), props.burn_residue);
            continue;
        }
        // Burning fuel becomes a flame that outlives it and leaves its residue.
        // Magnesium reports itself as HOT so the renderer paints it white.
        world.grid.set(
            nx,
            ny,
            Cell::raw(
                Element::Fire,
                props.burn_variant,
                props.burn_life,
                0,
                props.burn_residue,
            ),
        );
        world.log_reaction(match target.element {
            Element::Wood => "combustion: C6H10O5 + 6 O2 -> 6 CO2 + 5 H2O  (wood + ash)",
            Element::Plant => "combustion: C6H12O6 + 6 O2 -> 6 CO2 + 6 H2O",
            Element::Oil => "combustion: 2 CnH2n + 3n O2 -> 2n CO2 + 2n H2O",
            Element::Magnesium => "2 Mg + O2 -> 2 MgO  (blinding white flame)",
            _ => "combustion: fuel + O2 -> CO2 + H2O",
        });
    }
}

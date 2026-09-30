//! Whole world behaviour tests: the movement rules, seen from the outside.
//!
//! These drive `World` exactly like the browser does (paint, then tick) and
//! assert on emergent behaviour: piles, levelling, layering, buoyancy and the
//! "nothing is ever updated twice" guarantee.

use alkalab::{can_rise, can_sink, Element, World};

/// Builds a world with a stone box so that falling material has something to
/// land on other than the border.
fn boxed_world(width: i32, height: i32, seed: u32) -> World {
    let mut world = World::new(width, height, seed);
    for x in 0..width {
        world.grid_mut().set_element(x, height - 1, Element::Stone);
        world.grid_mut().set_element(x, 0, Element::Stone);
    }
    for y in 0..height {
        world.grid_mut().set_element(0, y, Element::Stone);
        world.grid_mut().set_element(width - 1, y, Element::Stone);
    }
    world
}

/// Height of the stack of anything resting on `from` downwards.
fn column_height(world: &World, x: i32, from: i32) -> i32 {
    let mut height = 0;
    for y in (0..=from).rev() {
        if world.element(x, y) != Element::Empty {
            height += 1;
        } else {
            break;
        }
    }
    height
}

#[test]
fn displacement_table_is_sane() {
    // Air gives way to everything.
    assert!(can_sink(Element::Sand, Element::Empty));
    assert!(can_sink(Element::Water, Element::Empty));
    // Solids never budge.
    assert!(!can_sink(Element::Sand, Element::Stone));
    assert!(!can_sink(Element::Water, Element::Metal));
    assert!(!can_rise(Element::Steam, Element::Stone));
    // Density decides who sinks through whom.
    assert!(can_sink(Element::Sand, Element::Water));
    assert!(can_sink(Element::Water, Element::Oil));
    assert!(can_sink(Element::Mercury, Element::Water));
    assert!(!can_sink(Element::Oil, Element::Water));
    assert!(!can_sink(Element::Snow, Element::Water));
    // Gases bubble up through liquids, but never through solids.
    assert!(can_rise(Element::Hydrogen, Element::Water));
    assert!(can_rise(Element::Steam, Element::Water));
    assert!(can_rise(Element::Hydrogen, Element::Smoke));
    assert!(!can_rise(Element::Smoke, Element::Hydrogen));
    // Nothing moves through bedrock.
    assert!(!can_sink(Element::Sand, Element::Wall));
    assert!(!can_rise(Element::Hydrogen, Element::Wall));
}

#[test]
fn a_grain_falls_one_cell_at_first_then_accelerates() {
    let mut world = World::new(8, 40, 1);
    world.paint_circle(4, 2, 0, Element::Sand);
    assert_eq!(world.element(4, 2), Element::Sand);

    world.step();
    assert_eq!(
        world.element(4, 3),
        Element::Sand,
        "a single tick moves a fresh grain exactly one cell - never to the floor"
    );

    // Momentum builds up, so it covers more ground per tick later on.
    world.step_n(5);
    let after_six = (0..world.height())
        .find(|&y| world.element(4, y) == Element::Sand)
        .expect("the grain is somewhere");
    assert!(after_six > 6, "the grain should have accelerated");

    world.step_n(60);
    assert_eq!(world.element(4, world.height() - 1), Element::Sand);
}

#[test]
fn sand_never_teleports_to_the_floor() {
    // A grain in free fall may cover at most the terminal velocity per tick.
    let max_per_tick = i32::from(alkalab::world::MAX_MOMENTUM) / 6 + 1;
    let mut world = World::new(8, 200, 9);
    world.paint_circle(4, 1, 0, Element::Sand);
    let mut previous = 1;
    for tick in 0..140 {
        world.step();
        let y = (0..world.height())
            .find(|&y| world.element(4, y) == Element::Sand)
            .expect("the grain is somewhere");
        let travelled = y - previous;
        assert!(
            travelled <= max_per_tick,
            "tick {tick}: the grain moved {travelled} cells, more than terminal velocity"
        );
        previous = y;
    }
}

#[test]
fn sand_settles_into_a_pile() {
    let mut world = boxed_world(64, 40, 3);
    for x in 28..36 {
        for y in 4..12 {
            world.grid_mut().set_element(x, y, Element::Sand);
        }
    }
    let expected = world.count(Element::Sand);
    world.step_n(400);

    assert_eq!(world.count(Element::Sand), expected, "sand is conserved");

    // No grain may hang in mid-air once the pile is at rest.
    for y in 1..world.height() - 2 {
        for x in 1..world.width() - 1 {
            if world.element(x, y) == Element::Sand {
                assert_ne!(
                    world.element(x, y + 1),
                    Element::Empty,
                    "unsupported grain at ({x}, {y})"
                );
            }
        }
    }

    // The pile spreads out sideways - that is what the diagonal move is for.
    let footprint = (1..world.width() - 1)
        .filter(|&x| (1..world.height() - 1).any(|y| world.element(x, y) == Element::Sand))
        .count();
    assert!(
        footprint > 8,
        "a pile should spread out (footprint {footprint})"
    );
}

#[test]
fn water_levels_out() {
    let mut world = boxed_world(64, 32, 5);
    for y in 4..20 {
        for x in 30..34 {
            world.grid_mut().set_element(x, y, Element::Water);
        }
    }
    let expected = world.count(Element::Water);
    world.step_n(600);
    assert_eq!(world.count(Element::Water), expected, "water is conserved");

    // Every column holding water should reach roughly the same surface height.
    let heights: Vec<i32> = (1..world.width() - 1)
        .map(|x| column_height(&world, x, world.height() - 2))
        .filter(|&h| h > 0)
        .collect();
    assert!(
        heights.len() > 4,
        "the puddle should be wider than the column"
    );
    let lowest = *heights.iter().min().unwrap();
    let highest = *heights.iter().max().unwrap();
    assert!(
        highest - lowest <= 2,
        "a settled surface should be flat, got {lowest}..{highest}"
    );
}

#[test]
fn water_finds_a_drain_and_flows_through_it() {
    let mut world = World::new(32, 32, 11);
    // A stone shelf across rows 20 and 21, with a single cell drain at x = 16.
    for x in 1..31 {
        if x != 16 {
            world.grid_mut().set_element(x, 20, Element::Stone);
            world.grid_mut().set_element(x, 21, Element::Stone);
        }
    }
    for x in 2..30 {
        world.grid_mut().set_element(x, 18, Element::Water);
        world.grid_mut().set_element(x, 19, Element::Water);
    }
    let expected = world.count(Element::Water);

    world.step_n(400);

    assert_eq!(world.count(Element::Water), expected, "water is conserved");
    let drained = (22..32)
        .filter(|&y| world.element(16, y) == Element::Water)
        .count();
    assert!(drained > 0, "water should have poured through the drain");
}

#[test]
fn liquids_layer_by_density() {
    let mut world = boxed_world(24, 24, 13);
    // Three immiscible liquids, deliberately in the wrong order.
    for x in 2..22 {
        world.grid_mut().set_element(x, 6, Element::Mercury);
        world.grid_mut().set_element(x, 7, Element::Oil);
        world.grid_mut().set_element(x, 8, Element::Water);
        world.grid_mut().set_element(x, 9, Element::Water);
    }
    for _ in 0..900 {
        world.step();
    }

    // When it settles the order must be oil, water, mercury from top to bottom.
    // The three liquids started as four rows of cells, so they settle into four
    // rows above the stone floor.
    let first_liquid_row = world.height() - 5;
    let deepest_liquid_row = world.height() - 2;
    let column: Vec<Element> = (first_liquid_row..=deepest_liquid_row)
        .map(|y| world.element(12, y))
        .collect();
    assert_eq!(
        column,
        vec![
            Element::Oil,
            Element::Water,
            Element::Water,
            Element::Mercury
        ],
        "denser liquids must sink below lighter ones"
    );

    // Every column must read oil, then water, then mercury from top to bottom.
    // Checking the *order* rather than exact rows keeps the test honest about
    // what the simulation promises, without depending on the surface being flat.
    fn rank(element: Element) -> i32 {
        match element {
            Element::Oil => 0,
            Element::Water => 1,
            Element::Mercury => 2,
            other => panic!("unexpected liquid {other:?}"),
        }
    }
    for x in 2..22 {
        let column: Vec<Element> = (1..world.height() - 1)
            .map(|y| world.element(x, y))
            .filter(|e| matches!(e, Element::Oil | Element::Water | Element::Mercury))
            .collect();
        assert!(
            column.windows(2).all(|pair| rank(pair[0]) <= rank(pair[1])),
            "column {x} is layered wrongly: {column:?}"
        );
    }

    // Nothing was created or destroyed while they sorted themselves out.
    assert_eq!(world.count(Element::Oil), 20);
    assert_eq!(world.count(Element::Mercury), 20);
    assert_eq!(world.count(Element::Water), 40);
}

#[test]
fn sand_sinks_and_ash_floats_on_water() {
    let mut world = boxed_world(20, 24, 17);
    for x in 2..18 {
        for y in 12..20 {
            world.grid_mut().set_element(x, y, Element::Water);
        }
    }
    world.grid_mut().set_element(5, 2, Element::Sand);
    world.grid_mut().set_element(14, 2, Element::Ash);
    for _ in 0..300 {
        world.step();
    }

    let sand_y = (0..world.height())
        .find(|&y| world.element(5, y) == Element::Sand)
        .expect("sand stays in its column");
    let ash_y = (0..world.height())
        .find(|&y| world.element(14, y) == Element::Ash)
        .expect("ash stays in its column");

    assert!(
        sand_y >= 20,
        "sand is denser than water, so it must sink to the bottom (found at {sand_y})"
    );
    assert!(
        ash_y < sand_y,
        "ash is lighter than water, so it must stay above the sand ({ash_y} vs {sand_y})"
    );
    assert_eq!(
        world.element(14, ash_y + 1),
        Element::Water,
        "the ash should be riding on the water surface"
    );
}

#[test]
fn snow_melts_into_the_water_it_lands_on() {
    let mut world = boxed_world(16, 20, 29);
    for x in 2..14 {
        for y in 12..14 {
            world.grid_mut().set_element(x, y, Element::Water);
        }
    }
    let water = world.count(Element::Water);
    world.grid_mut().set_element(8, 4, Element::Snow);

    world.step_n(400);

    assert_eq!(
        world.count(Element::Snow),
        0,
        "snow melts once it is in water"
    );
    assert!(
        world.count(Element::Water) > water,
        "the melted snow becomes water"
    );
}

#[test]
fn gases_rise_and_steam_condenses_back_into_water() {
    let mut world = boxed_world(16, 32, 19);
    for x in 4..12 {
        world.grid_mut().set_element(x, 28, Element::Steam);
    }
    world.step_n(20);
    let steam_high = (0..world.height())
        .filter(|&y| (0..world.width()).any(|x| world.element(x, y) == Element::Steam))
        .min()
        .expect("steam is still around");
    assert!(steam_high < 26, "steam should have risen");

    // Steam has a finite lifetime and condenses.
    world.step_n(400);
    assert_eq!(world.count(Element::Steam), 0, "steam condenses");
    assert!(world.count(Element::Water) > 0, "condensation leaves water");
}

#[test]
fn nothing_escapes_the_border() {
    let mut world = World::new(24, 24, 23);
    world.paint_circle(12, 12, 6, Element::Water);
    world.paint_circle(6, 6, 3, Element::Sand);
    world.paint_circle(18, 6, 3, Element::Hydrogen);
    world.paint_circle(12, 18, 3, Element::Lava);
    world.paint_circle(4, 18, 2, Element::Oil);
    world.explode(12, 12, 8, Element::Smoke);
    world.step_n(200);

    for (x, y) in [
        (-1, 5),
        (-1, -1),
        (24, 5),
        (5, 24),
        (24, 24),
        (-1, 24),
        (24, -1),
    ] {
        assert_eq!(
            world.element(x, y),
            Element::Wall,
            "the border at ({x}, {y}) was breached"
        );
    }
}

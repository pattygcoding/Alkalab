//! Chemistry set pieces: the reactions that make the lab fun, plus the
//! guarantees around them (chains, walls surviving blasts, the reaction log).

use alkalab::{Element, World};

/// A world with an indestructible wall ring near the edge, so tests can prove
/// that explosions respect bedrock.
fn lab(seed: u32) -> World {
    World::new(48, 32, seed)
}

/// Fills a rectangular region.
fn fill(world: &mut World, x0: i32, y0: i32, x1: i32, y1: i32, element: Element) {
    for y in y0..y1 {
        for x in x0..x1 {
            world.grid_mut().set_element(x, y, element);
        }
    }
}

/// How many cells anywhere are not air? (Blast damage indicator.)
fn filled_cells(world: &World) -> u32 {
    world.particle_count()
}

#[test]
fn sodium_dropped_in_a_lake_detonates() {
    let mut world = lab(101);
    fill(&mut world, 8, 20, 40, 30, Element::Water);
    world.grid_mut().set_element(24, 18, Element::Sodium);
    let water_before = world.count(Element::Water);

    // The products are transient (hydrogen burns, steam condenses), so watch the
    // world while the reaction happens rather than only at the end.
    let mut saw_products = false;
    let mut log = String::new();
    for _ in 0..180 {
        world.step();
        log = world.reactions_json();
        if world.count(Element::SaltWater) > 0
            || world.count(Element::Steam) > 0
            || world.count(Element::Hydrogen) > 0
        {
            saw_products = true;
        }
    }

    assert_eq!(world.count(Element::Sodium), 0, "the metal is consumed");
    assert!(
        log.contains("2 Na + 2 H2O -> 2 NaOH + H2"),
        "the equation should be in the log: {log}"
    );
    assert!(
        saw_products,
        "the reaction should release hydrogen and leave NaOH(aq) or steam"
    );
    assert!(
        world.count(Element::Water) < water_before,
        "the blast destroys water around the metal"
    );
}

#[test]
fn potassium_is_a_bigger_bang_than_sodium() {
    // Same experiment, same seed, one variable. Measure how much of the lake the
    // blast displaced.
    fn blast_damage(element: Element) -> u32 {
        let mut world = lab(4242);
        fill(&mut world, 4, 4, 44, 28, Element::Stone);
        fill(&mut world, 10, 10, 38, 24, Element::Water);
        // Hollow out a pocket so the metal starts in the same place each time.
        world.grid_mut().set_element(24, 16, element);
        let before = filled_cells(&world);
        for _ in 0..60 {
            world.step();
        }
        before.saturating_sub(filled_cells(&world))
    }

    let sodium = blast_damage(Element::Sodium);
    let potassium = blast_damage(Element::Potassium);
    assert!(sodium > 0, "sodium must do some damage");
    assert!(
        potassium > sodium,
        "potassium should out-blast sodium ({potassium} vs {sodium})"
    );

    // And the table says so too, which is the property we actually rely on.
    let radius_of = |metal: Element| {
        alkalab::REACTIONS
            .iter()
            .find(|r| r.a == metal && r.b == Element::Water)
            .map(|r| match r.effect {
                alkalab::reaction::Effect::Explode { radius, .. } => radius,
                alkalab::reaction::Effect::None => 0,
            })
            .expect("an alkali metal must have a water rule")
    };
    assert!(radius_of(Element::Potassium) > radius_of(Element::Sodium));
}

#[test]
fn explosions_never_break_the_wall() {
    let mut world = lab(55);
    // A stone box with walls (indestructible) around the contents.
    fill(&mut world, 10, 8, 38, 24, Element::Wall);
    fill(&mut world, 12, 10, 36, 22, Element::Water);
    world.grid_mut().set_element(24, 16, Element::Potassium);
    let water_before = world.count(Element::Water);

    world.step_n(300);

    // The wall ring must still be complete.
    for x in 10..38 {
        assert_eq!(world.element(x, 8), Element::Wall, "top wall at {x}");
        assert_eq!(world.element(x, 23), Element::Wall, "bottom wall at {x}");
    }
    for y in 8..24 {
        assert_eq!(world.element(10, y), Element::Wall, "left wall at {y}");
        assert_eq!(world.element(37, y), Element::Wall, "right wall at {y}");
    }
    assert!(
        world.count(Element::Water) <= water_before * 8 / 10,
        "the lake should have been wrecked (before {water_before}, after {})",
        world.count(Element::Water)
    );
}

#[test]
fn tnt_chain_reaction_consumes_every_charge() {
    let mut world = lab(7);
    // A long row of charges sitting on a stone shelf.
    fill(&mut world, 2, 28, 46, 30, Element::Stone);
    for x in 4..44 {
        world.grid_mut().set_element(x, 27, Element::Tnt);
    }
    let charges = world.count(Element::Tnt);
    assert!(charges > 30, "not enough charges to be a chain");

    // Light one end.
    world.grid_mut().set_element(4, 26, Element::Fire);
    world.step_n(240);

    assert_eq!(
        world.count(Element::Tnt),
        0,
        "the chain should have consumed every charge"
    );
    let log = world.reactions_json();
    assert!(log.contains("detonation"), "the log should show the bang");
}

#[test]
fn a_hydrogen_pocket_burns_back_into_water() {
    let mut world = lab(77);
    // Hydrogen is lighter than air, so it needs an airtight vessel for this test.
    fill(&mut world, 14, 10, 34, 26, Element::Wall);
    fill(&mut world, 15, 11, 33, 25, Element::Hydrogen);
    world.grid_mut().set_element(24, 24, Element::Fire);
    world.step_n(600);
    assert_eq!(
        world.count(Element::Hydrogen),
        0,
        "hydrogen is consumed by the flame front"
    );
    assert!(
        world.count(Element::Water) > 0,
        "2 H2 + O2 -> 2 H2O, so the gas comes back as water"
    );
}

#[test]
fn dry_tnt_detonates_but_wet_tnt_does_not() {
    // A dry charge next to a spark.
    let mut dry = lab(31);
    dry.grid_mut().set_element(23, 20, Element::Stone);
    dry.grid_mut().set_element(25, 20, Element::Stone);
    dry.grid_mut().set_element(24, 20, Element::Tnt);
    dry.grid_mut().set_element(24, 19, Element::Fire);
    dry.step_n(6);
    assert_eq!(dry.count(Element::Tnt), 0, "dry TNT must detonate");

    // The same charge, soaked first and then attacked with a flame.
    let mut wet = lab(31);
    wet.grid_mut().set_element(23, 20, Element::Stone);
    wet.grid_mut().set_element(25, 20, Element::Stone);
    wet.grid_mut().set_element(24, 20, Element::Tnt);
    wet.grid_mut().set_element(24, 19, Element::Water);
    wet.step_n(30);
    assert!(
        wet.grid().cell(24, 20).is_wet(),
        "water should have dampened the charge"
    );

    wet.grid_mut().set_element(24, 19, Element::Fire);
    wet.step_n(90);
    assert_eq!(
        wet.count(Element::Tnt),
        1,
        "wet TNT must stay inert under a flame"
    );
}

#[test]
fn lava_quenched_by_water_turns_to_stone_and_steam() {
    let mut world = lab(88);
    fill(&mut world, 10, 20, 38, 24, Element::Water);
    fill(&mut world, 20, 14, 28, 16, Element::Lava);

    world.step_n(200);

    assert_eq!(world.count(Element::Lava), 0, "it all quenched");
    assert!(world.count(Element::Stone) > 0, "quenched lava is stone");
    assert!(world.count(Element::Steam) > 0, "and it flashes to steam");
}

#[test]
fn lava_fuses_sand_into_glass() {
    let mut world = lab(99);
    fill(&mut world, 16, 20, 32, 24, Element::Sand);
    fill(&mut world, 16, 14, 32, 17, Element::Lava);
    world.step_n(400);
    assert!(
        world.count(Element::Glass) > 0,
        "molten rock should fuse sand into glass"
    );
}

#[test]
fn acid_burns_a_hole_through_a_steel_plate() {
    let mut world = lab(121);
    // A steel plate with a drop of acid on top of it.
    fill(&mut world, 14, 20, 34, 22, Element::Metal);
    world.grid_mut().set_element(24, 19, Element::Acid);
    let plate = world.count(Element::Metal);

    world.step_n(400);

    assert!(
        world.count(Element::Metal) < plate,
        "acid must dissolve steel"
    );
    // Spent acid becomes water, and the fizz releases hydrogen.
    assert!(world.count(Element::Water) > 0 || world.count(Element::Hydrogen) > 0);
}

#[test]
fn wood_burns_into_ash_and_smoke() {
    let mut world = lab(141);
    fill(&mut world, 16, 24, 32, 30, Element::Wood);
    // Touch the timber directly, otherwise the flame just burns out in mid-air.
    world.grid_mut().set_element(24, 23, Element::Fire);
    let logs = world.count(Element::Wood);

    world.step_n(600);

    assert!(
        world.count(Element::Wood) < logs,
        "the fire should have eaten into the timber"
    );
    assert!(world.count(Element::Ash) > 0, "burning wood leaves ash");
}

#[test]
fn a_gunpowder_trail_ignites_from_one_spark() {
    let mut world = lab(161);
    fill(&mut world, 2, 26, 46, 28, Element::Stone);
    for x in 4..12 {
        world.grid_mut().set_element(x, 25, Element::Gunpowder);
    }
    world.grid_mut().set_element(4, 24, Element::Fire);

    world.step_n(120);

    assert_eq!(
        world.count(Element::Gunpowder),
        0,
        "the spark should have consumed the whole trail"
    );
}

#[test]
fn the_reaction_log_is_machine_readable() {
    let mut world = lab(181);
    fill(&mut world, 10, 20, 38, 26, Element::Water);
    world.grid_mut().set_element(24, 18, Element::Sodium);

    // Watch the log while the reaction happens: the ring buffer only keeps the
    // most recent entries, so we check as soon as the equation shows up.
    let mut log = String::new();
    let mut recorded = false;
    for _ in 0..60 {
        world.step();
        log = world.reactions_json();
        if log.contains("2 Na + 2 H2O") {
            recorded = true;
            break;
        }
    }

    assert!(recorded, "the sodium reaction should be logged: {log}");
    assert!(log.starts_with('[') && log.ends_with(']'), "log: {log}");
    assert!(log.contains("\"equation\""), "log: {log}");
    assert!(log.contains("\"count\""), "log: {log}");
    // Nothing that would break JSON.parse on the JavaScript side.
    assert!(!log.contains('\n'), "the log is a single JSON line");
    assert!(!log.contains("\\"), "no escaping needed: {log}");
}

#[test]
fn the_element_catalogue_describes_every_element() {
    let catalogue = World::element_catalog_json();
    assert!(catalogue.starts_with('[') && catalogue.ends_with(']'));
    assert_eq!(
        catalogue.matches("\"id\":").count(),
        alkalab::ELEMENT_COUNT,
        "every element must be in the catalogue"
    );
    for element in Element::ALL {
        assert!(
            catalogue.contains(element.name()),
            "{} is missing from the catalogue",
            element.name()
        );
    }
    assert!(catalogue.contains("\"color\":\"#"));
    assert!(catalogue.contains("\"formula\":\"H2O\""));
}

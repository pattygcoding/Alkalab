//! Invariants, determinism and a stress test.
//!
//! The engine is deterministic by design (own PRNG, integer maths, no
//! iterating a `HashMap`), which makes all of this reproducible: the same seed
//! and the same inputs always produce the same world, byte for byte.

use alkalab::{Element, Rng, World};

/// Every element except air, for abuse testing.
const MATERIALS: [Element; 23] = [
    Element::Wall,
    Element::Stone,
    Element::Glass,
    Element::Wood,
    Element::Plant,
    Element::Metal,
    Element::Ice,
    Element::Tnt,
    Element::Sand,
    Element::Salt,
    Element::Ash,
    Element::Snow,
    Element::Gunpowder,
    Element::Carbide,
    Element::Lime,
    Element::Water,
    Element::SaltWater,
    Element::Acid,
    Element::Oil,
    Element::Lava,
    Element::Mercury,
    Element::Sodium,
    Element::Potassium,
];

#[test]
fn the_same_seed_and_inputs_produce_the_same_world() {
    fn run(seed: u32) -> (u32, Vec<u8>) {
        let mut world = World::new(64, 48, seed);
        world.paint_circle(10, 10, 6, Element::Sand);
        world.paint_circle(30, 10, 6, Element::Water);
        world.paint_circle(50, 6, 4, Element::Hydrogen);
        world.paint_circle(20, 30, 5, Element::Lava);
        world.paint_circle(40, 30, 5, Element::Oil);
        world.step_n(240);
        (world.hash(), world.pixels().to_vec())
    }

    let (hash_a, pixels_a) = run(2024);
    let (hash_b, pixels_b) = run(2024);
    assert_eq!(hash_a, hash_b, "same seed, same result");
    assert_eq!(pixels_a, pixels_b, "same pixels too");

    let (hash_c, _) = run(2025);
    assert_ne!(hash_a, hash_c, "a different seed should diverge");
}

#[test]
fn the_world_state_is_deterministic_across_ticks() {
    let mut world = World::new(48, 32, 11);
    world.paint_circle(24, 8, 7, Element::Sand);
    world.step_n(50);
    let hash = world.hash();

    // Re-running the last step must land on the same state again.
    let mut fresh = World::new(48, 32, 11);
    fresh.paint_circle(24, 8, 7, Element::Sand);
    fresh.step_n(50);
    assert_eq!(fresh.hash(), hash);
}

#[test]
fn undo_restores_the_exact_previous_state() {
    let mut world = World::new(32, 24, 5);
    world.paint_circle(10, 10, 3, Element::Sand);
    world.step_n(20);
    let before = world.hash();

    assert!(!world.can_undo(), "nothing to undo yet");
    world.snapshot();
    world.paint_circle(16, 4, 5, Element::Tnt);
    world.explode(16, 4, 8, Element::Smoke);
    world.step_n(10);
    assert_ne!(world.hash(), before);

    assert!(world.can_undo());
    assert!(world.undo(), "undo should have something to do");
    assert_eq!(world.hash(), before, "undo restores the exact state");
    assert_eq!(world.count(Element::Tnt), 0, "the charges are gone again");
    assert!(!world.can_undo());
    assert!(!world.undo(), "an empty stack does nothing");
}

#[test]
fn pixel_buffer_is_always_a_full_opaque_rgba_image() {
    let mut world = World::new(48, 32, 3);
    let rng = &mut Rng::new(99);
    for i in 0..40 {
        let element = MATERIALS[i % MATERIALS.len()];
        let x = rng.below(48) as i32;
        let y = rng.below(32) as i32;
        world.paint_circle(x, y, 3, element);
        world.step_n(5);
        assert_eq!(world.pixels().len(), world.len() * 4);
        for chunk in world.pixels().chunks_exact(4) {
            assert_eq!(chunk[3], 255, "the shared buffer is always opaque");
        }
    }
}

#[test]
fn random_painting_never_breaks_the_engine() {
    // A fuzz test: paint everything everywhere, step a lot, and check that the
    // invariants still hold at the end.
    let mut world = World::new(80, 60, 4242);
    let mut rng = Rng::new(0xC0FF_EE00);

    for round in 0..120 {
        let element = MATERIALS[rng.below(MATERIALS.len() as u32) as usize];
        let x = rng.range_i32(-4, 84);
        let y = rng.range_i32(-4, 64);
        let radius = rng.range_i32(1, 6);
        if round % 7 == 0 {
            let x2 = rng.range_i32(-4, 84);
            let y2 = rng.range_i32(-4, 64);
            world.paint_line(x, y, x2, y2, radius, element);
        } else {
            world.paint_circle(x, y, radius, element);
        }
        if round % 11 == 0 {
            world.explode(x, y, rng.range_i32(1, 16), Element::Smoke);
        }
        world.step_n(10);
    }

    // Bedrock is forever.
    for (x, y) in [(-1, 0), (-1, -1), (80, 0), (0, 60), (80, 60)] {
        assert_eq!(world.element(x, y), Element::Wall, "border at ({x}, {y})");
    }
    assert!(world.particle_count() <= world.len() as u32);
    // No element id may fall outside the table.
    for (x, y) in world.grid().coords() {
        let id = world.element(x, y).id();
        assert!(usize::from(id) < alkalab::ELEMENT_COUNT);
    }
}

#[test]
fn a_full_size_world_ticks_quickly_enough() {
    // Not a benchmark, just a guard against accidental O(n^2) work: a busy
    // 320 x 200 world must simulate 100 ticks without hanging.
    let mut world = World::new(320, 200, 8);
    let mut rng = Rng::new(7);
    for i in 0..400 {
        let element = MATERIALS[i % MATERIALS.len()];
        world.paint_circle(rng.below(320) as i32, rng.below(200) as i32, 4, element);
    }
    let start = std::time::Instant::now();
    world.step_n(100);
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 30,
        "100 ticks took {elapsed:?}, something is wrong"
    );
}

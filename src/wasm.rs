//! The JavaScript facing surface.
//!
//! Compiled only for `wasm32`, so the host build (`cargo test`) stays a pure
//! `std` library. The API is deliberately small and flat: JavaScript owns the
//! DOM, input handling and the request-animation-frame loop, and Rust owns the
//! state buffer, the physics and the pixels.

use wasm_bindgen::prelude::*;

use crate::element::Element;
use crate::world::{World, DEFAULT_HEIGHT, DEFAULT_WIDTH};

/// The engine handle JavaScript talks to.
///
/// ```js
/// const engine = new Engine(320, 200, seed >>> 0);
/// const pixels = new Uint8ClampedArray(
///   wasm.memory.buffer,
///   engine.pixel_ptr,
///   engine.pixel_len,
/// );
/// ```
#[wasm_bindgen]
pub struct Engine {
    world: World,
}

#[wasm_bindgen]
impl Engine {
    /// Creates an engine with an explicit size and RNG seed.
    #[wasm_bindgen(constructor)]
    pub fn new(width: i32, height: i32, seed: u32) -> Engine {
        Engine {
            world: World::new(width, height, seed),
        }
    }

    /// Creates an engine at the default `320 x 200` size.
    pub fn create(seed: u32) -> Engine {
        Engine::new(DEFAULT_WIDTH, DEFAULT_HEIGHT, seed)
    }

    /// World width in cells.
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> i32 {
        self.world.width()
    }

    /// World height in cells.
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> i32 {
        self.world.height()
    }

    /// Ticks simulated so far.
    #[wasm_bindgen(getter)]
    pub fn frame(&self) -> u32 {
        self.world.frame()
    }

    /// Pointer to the shared RGBA buffer inside wasm linear memory.
    #[wasm_bindgen(getter)]
    pub fn pixel_ptr(&self) -> *const u8 {
        self.world.pixel_ptr()
    }

    /// Length of the shared RGBA buffer in bytes.
    #[wasm_bindgen(getter)]
    pub fn pixel_len(&self) -> usize {
        self.world.pixel_len()
    }

    /// How many cells are not air (shown in the HUD).
    #[wasm_bindgen(getter)]
    pub fn particle_count(&self) -> u32 {
        self.world.particle_count()
    }

    /// Is there a snapshot on the undo stack?
    #[wasm_bindgen(getter)]
    pub fn can_undo(&self) -> bool {
        self.world.can_undo()
    }

    /// Fingerprint of the whole state buffer.
    #[wasm_bindgen(getter)]
    pub fn hash(&self) -> u32 {
        self.world.hash()
    }

    /// Advances the simulation by one tick.
    pub fn step(&mut self) {
        self.world.step();
    }

    /// Advances by `ticks` ticks (capped, so a tab that was asleep cannot hang).
    pub fn step_n(&mut self, ticks: u32) {
        self.world.step_n(ticks.min(64));
    }

    /// Paints a round brush of `element` at `(x, y)`.
    pub fn paint(&mut self, x: i32, y: i32, radius: i32, element: u8) -> u32 {
        self.world
            .paint_circle(x, y, radius, Element::from_id(element))
    }

    /// Paints a stroke between two pointer positions, filling in the gaps of a
    /// fast drag.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_line(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        radius: i32,
        element: u8,
    ) -> u32 {
        self.world
            .paint_line(x0, y0, x1, y1, radius, Element::from_id(element))
    }

    /// Detonates a charge, releasing smoke (the "boom" tool).
    pub fn detonate(&mut self, x: i32, y: i32, radius: i32) {
        self.world.explode(x, y, radius, Element::Smoke);
        self.world.render();
    }

    /// Paints a filled rectangle, used by the square brush and the demos.
    pub fn paint_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, element: u8) -> u32 {
        self.world
            .paint_rect(x0, y0, x1, y1, Element::from_id(element))
    }

    /// Empties the world (undoable).
    pub fn clear(&mut self) {
        self.world.clear();
    }

    /// Pushes the current state onto the undo stack. Called once per brush
    /// stroke, not once per pointer move.
    pub fn snapshot(&mut self) {
        self.world.snapshot();
    }

    /// Restores the previous snapshot. Returns whether anything was undone.
    pub fn undo(&mut self) -> bool {
        self.world.undo()
    }

    /// Element id at `(x, y)`.
    pub fn element_at(&self, x: i32, y: i32) -> u8 {
        self.world.element(x, y).id()
    }

    /// How many cells are made of `element`.
    pub fn count_of(&self, element: u8) -> u32 {
        self.world.count(Element::from_id(element))
    }

    /// Recent chemical reactions as a JSON array, newest first.
    pub fn reactions(&self) -> String {
        self.world.reactions_json()
    }

    /// The element catalogue as a JSON array, so the UI palette is generated
    /// from the Rust property table instead of duplicating it in JavaScript.
    pub fn element_catalog(&self) -> String {
        World::element_catalog_json()
    }
}

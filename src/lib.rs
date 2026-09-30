//! # Alkalab
//!
//! A falling-sand / cellular-automata chemistry lab that compiles to
//! WebAssembly.
//!
//! The crate is split in two halves:
//!
//! * the simulation core ([`World`], [`Grid`], [`Element`]) which is plain `std`
//!   Rust, so it builds for any target and is unit tested with `cargo test` on
//!   the host;
//! * [`Engine`], the thin `wasm-bindgen` surface (compiled only for wasm32)
//!   that hands JavaScript a pointer to the shared RGBA pixel buffer.
//!
//! ```no_run
//! use alkalab::{Element, World};
//!
//! let mut world = World::new(64, 32, 42);
//! world.paint_circle(10, 4, 3, Element::Sand);
//! let painted = world.count(Element::Sand);
//! world.step_n(60);
//! assert_eq!(world.count(Element::Sand), painted, "sand is conserved");
//! assert_eq!(world.element(10, 31), Element::Sand, "sand reaches the floor");
//! ```

pub mod cell;
pub mod color;
pub mod element;
pub mod grid;
pub mod physics;
pub mod reaction;
pub mod render;
pub mod rng;
pub mod world;

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::Engine;

pub use cell::Cell;
pub use element::{Category, Element, Props, ELEMENT_COUNT};
pub use grid::Grid;
pub use physics::{can_rise, can_sink, climb_distance, fall_distance, NEIGHBOURS4, NEIGHBOURS8};
pub use reaction::{Outcome, Reaction, REACTIONS};
pub use rng::Rng;
pub use world::{LogEntry, World, DEFAULT_HEIGHT, DEFAULT_WIDTH};

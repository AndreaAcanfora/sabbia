//! sabbia: a falling-sand game. The engine is plain Rust; `wasm` exposes it to the browser.

pub mod element;
pub mod scene;
pub mod world;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub use element::{Element, Kind};
pub use world::World;

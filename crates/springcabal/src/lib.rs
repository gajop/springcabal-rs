//! Shared building blocks for SpringCabal's Rust/WASM games.
//!
//! Only what our Ludum Dare games have in common goes here: how they talk
//! between rules and UI, how their tests report, their music and screenshot
//! widgets, their input conventions. Anything useful to Spring games in
//! general belongs in the engine's `spring` / `spring-addons` crates instead.
#![no_std]

extern crate alloc;

pub mod keys;
pub mod message;

// These call into the engine, whose functions exist only on wasm32. The
// modules above are plain logic and are unit-tested on the host.
#[cfg(target_arch = "wasm32")]
pub mod check;
#[cfg(target_arch = "wasm32")]
pub mod menu;
#[cfg(target_arch = "wasm32")]
pub mod music;
#[cfg(target_arch = "wasm32")]
pub mod shots;

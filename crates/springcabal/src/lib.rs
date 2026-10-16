//! Shared building blocks for SpringCabal's Rust/WASM games.
//!
//! Only what our Ludum Dare games have in common goes here: how they talk
//! between rules and UI, how their tests report, their music and screenshot
//! widgets, their input conventions. Anything useful to Spring games in
//! general belongs in the engine's `spring` / `spring-addons` crates instead.
#![no_std]

extern crate alloc;

// Engine calls only work on wasm32 (on the host they panic), so the host
// unit tests cover the logic around them.
pub mod check;
#[cfg(any(target_arch = "wasm32", test))]
pub mod controls;
pub mod defs;
pub mod keys;
pub mod local;
pub mod menu;
pub mod message;
pub mod music;
pub mod shots;
pub mod verification;

#[cfg(target_arch = "wasm32")]
pub mod volumes;

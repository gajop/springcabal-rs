//! Player movement and a follow camera for SpringCabal's Rust/WASM games.
//!
//! Most of our games put the player in control of one unit: WASD or the
//! mouse moves it, and the camera follows it. This crate is that plumbing,
//! so a game only writes its rules (speeds, jumps, form switches...).
//!
//! UI side:
//! - [`keyboard`]: WASD / arrows to a direction, sent to the rules on change
//!   ([`KeyMovement`](keyboard::KeyMovement)).
//! - [`pointer`]: the mouse (`mouse`). What a screen position points at is
//!   the engine's `spring::pick`.
//! - [`hold`]: hold a button to keep moving towards the cursor.
//! - [`camera`]: [`View`](camera::View), `CameraPlacement`, and the
//!   `CameraFollow` camera (smoothing, lead, first-frame snap).
//!
//! Between them, [`protocol`]: the `movement|x|z` message.
//!
//! Rules side:
//! - [`goal`]: a held direction as a [`MoveGoal`](goal::MoveGoal) ahead of
//!   the unit, and [`Steering`](goal::Steering) (the direction, and when to
//!   stop).
//!
//! Engine helpers receive a scoped Spring reference on native and Wasm.
#![no_std]

extern crate alloc;

pub mod camera;
pub mod goal;
pub mod hold;
pub mod keyboard;
pub mod pointer;
pub mod protocol;

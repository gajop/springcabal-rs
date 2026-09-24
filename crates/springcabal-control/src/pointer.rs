//! UI side: the mouse, and what it points at on the map.
//!
//! What a screen position points at is the engine's `spring::pick`
//! (`trace_screen`, `ground_at`, `unit_at`); this module adds the mouse
//! itself. Screen coordinates are the engine's: x from the left, y from the
//! bottom, as `get_mouse_state` reports them and `spring::pick` takes them.

/// The mouse, while it is over the window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mouse {
    pub x: f32,
    pub y: f32,
    pub left: bool,
    pub middle: bool,
    pub right: bool,
}

#[cfg(target_arch = "wasm32")]
mod engine {
    use spring::Float3;

    use super::Mouse;

    /// The mouse, or `None` while it is off the window.
    pub fn mouse() -> Option<Mouse> {
        let mouse = spring::input::get_mouse_state(0).ok()?;
        (!mouse.offscreen && mouse.x.is_finite() && mouse.y.is_finite()).then_some(Mouse {
            x: mouse.x,
            y: mouse.y,
            left: mouse.left,
            middle: mouse.middle,
            right: mouse.right,
        })
    }

    /// The ground under the mouse, through any unit or feature on it. `None`
    /// off the window or off the map.
    pub fn ground_under_mouse() -> Option<Float3> {
        let mouse = mouse()?;
        spring::pick::ground_at(mouse.x, mouse.y)
    }
}

#[cfg(target_arch = "wasm32")]
pub use engine::*;

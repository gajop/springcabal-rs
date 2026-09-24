//! UI side: the mouse, and what it points at on the map.
//!
//! Screen coordinates are the engine's: x from the left, y from the bottom,
//! as `get_mouse_state` reports them and `trace_screen_ray` takes them.

use spring::Float3;

/// What a screen ray hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    Unit {
        id: i32,
        position: Float3,
    },
    Feature {
        id: i32,
        position: Float3,
    },
    Ground(Float3),
    /// Nothing on the map; the point where the ray crosses the ground's
    /// plane (only when sky hits were asked for).
    Sky(Float3),
}

impl Hit {
    /// Decode the engine's trace result: `hit_type` 1 unit, 2 feature,
    /// 3 ground, 4 sky, 0 nothing (whose position is a meaningless zero).
    pub fn from_trace(hit_type: i32, id: i32, position: Float3) -> Option<Self> {
        if !(position.x.is_finite() && position.y.is_finite() && position.z.is_finite()) {
            return None;
        }
        Some(match hit_type {
            1 => Self::Unit { id, position },
            2 => Self::Feature { id, position },
            3 => Self::Ground(position),
            4 => Self::Sky(position),
            _ => return None,
        })
    }

    pub fn position(self) -> Float3 {
        match self {
            Self::Unit { position, .. } | Self::Feature { position, .. } => position,
            Self::Ground(position) | Self::Sky(position) => position,
        }
    }

    pub fn unit(self) -> Option<i32> {
        match self {
            Self::Unit { id, .. } => Some(id),
            _ => None,
        }
    }

    pub fn ground(self) -> Option<Float3> {
        match self {
            Self::Ground(position) => Some(position),
            _ => None,
        }
    }
}

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
    use spring::camera::{TraceScreenRayOptions, trace_screen_ray};

    use super::{Hit, Mouse};

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

    /// Trace a ray from the camera through screen point `(x, y)`. With
    /// `ground_only`, units and features don't stop the ray: the hit is the
    /// ground behind them. `None` when the ray misses the map.
    pub fn trace(x: f32, y: f32, ground_only: bool) -> Option<Hit> {
        let trace = trace_screen_ray(
            x,
            y,
            TraceScreenRayOptions {
                only_coords: ground_only,
                use_minimap: false,
                include_sky: false,
                ignore_water: false,
                height_offset: 0.0,
            },
        )
        .ok()?;
        Hit::from_trace(trace.hit_type, trace.hit_id, trace.hit_pos)
    }

    /// The ground under screen point `(x, y)`, through any unit or feature on
    /// it. `None` off the map.
    pub fn ground_at(x: f32, y: f32) -> Option<Float3> {
        trace(x, y, true)?.ground()
    }

    /// The unit under screen point `(x, y)`.
    pub fn unit_at(x: f32, y: f32) -> Option<i32> {
        trace(x, y, false)?.unit()
    }

    /// The ground under the mouse.
    pub fn ground_under_mouse() -> Option<Float3> {
        let mouse = mouse()?;
        ground_at(mouse.x, mouse.y)
    }
}

#[cfg(target_arch = "wasm32")]
pub use engine::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_hit_types() {
        let at = Float3::new(1.0, 2.0, 3.0);
        assert_eq!(Hit::from_trace(3, -1, at), Some(Hit::Ground(at)));
        assert_eq!(Hit::from_trace(1, 7, at).and_then(Hit::unit), Some(7));
        assert_eq!(Hit::from_trace(1, 7, at).and_then(Hit::ground), None);
        // A miss reports a zero position that must not be used.
        assert_eq!(Hit::from_trace(0, -1, Float3::ZERO), None);
        assert_eq!(
            Hit::from_trace(3, -1, Float3::new(f32::NAN, 0.0, 0.0)),
            None
        );
        assert_eq!(Hit::from_trace(4, -1, at).map(Hit::position), Some(at));
    }
}

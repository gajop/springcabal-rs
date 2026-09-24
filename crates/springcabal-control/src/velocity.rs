//! Rules side: steering a unit by setting its velocity directly
//! (`MoveCtrl`), for games that want instant, exact movement instead of the
//! unit's own acceleration and turning (see [`goal`](crate::goal) for that).
//!
//! While MoveCtrl is enabled the engine's movement type is switched off: the
//! unit moves by exactly the velocity set, follows the ground only if asked
//! to, and ignores move orders.

use spring::Float3;

/// A held direction as a velocity of `speed` elmos per frame.
///
/// ```ignore
/// const DRIVE: VelocityDrive = VelocityDrive::new(3.0);
/// DRIVE.enable(unit_id)?; // once, e.g. in UnitCreated
/// DRIVE.apply(unit_id, steering.direction().unwrap_or((0.0, 0.0)))?; // every frame
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VelocityDrive {
    speed: f32,
    face: bool,
}

impl VelocityDrive {
    /// Moves at `speed` and turns the unit to face where it moves.
    pub const fn new(speed: f32) -> Self {
        Self { speed, face: true }
    }

    pub const fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Whether to turn the unit to face its direction of movement.
    pub const fn facing(mut self, face: bool) -> Self {
        self.face = face;
        self
    }

    /// The velocity for `direction` (normalized; zero for none).
    pub fn velocity(&self, (dx, dz): (f32, f32)) -> Float3 {
        let length = (dx * dx + dz * dz).sqrt();
        if length.is_nan() || length <= f32::EPSILON {
            return Float3::ZERO;
        }
        Float3::new(dx / length * self.speed, 0.0, dz / length * self.speed)
    }
}

#[cfg(target_arch = "wasm32")]
impl VelocityDrive {
    /// Hand the unit's movement to MoveCtrl, keeping it on the ground.
    pub fn enable(&self, unit_id: i32) -> spring::Result<bool> {
        let enabled = spring::move_ctrl::move_ctrl(unit_id, true)?;
        spring::move_ctrl::set_track_ground(unit_id, true)?;
        Ok(enabled)
    }

    /// Give the unit its own movement back.
    pub fn disable(&self, unit_id: i32) -> spring::Result<bool> {
        spring::move_ctrl::move_ctrl(unit_id, false)
    }

    /// Move in `direction` this frame; `(0, 0)` stops the unit.
    pub fn apply(&self, unit_id: i32, direction: (f32, f32)) -> spring::Result<bool> {
        let velocity = self.velocity(direction);
        let moved = spring::move_ctrl::set_velocity(unit_id, velocity)?;
        if self.face && velocity != Float3::ZERO {
            spring::move_ctrl::set_heading(unit_id, i32::from(heading(direction)))?;
        }
        Ok(moved)
    }
}

/// The engine heading (-32768..32767, 0 facing +z) of a direction on the
/// map: the engine's own `GetHeadingFromVector`, as Lua's MoveCtrl code gets
/// it.
pub fn heading((dx, dz): (f32, f32)) -> i16 {
    Float3::new(dx, 0.0, dz).heading()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_is_normalized() {
        let drive = VelocityDrive::new(2.0);
        assert_eq!(drive.velocity((1.0, 0.0)), Float3::new(2.0, 0.0, 0.0));
        let diagonal = drive.velocity((1.0, 1.0));
        assert!((diagonal.x - core::f32::consts::SQRT_2).abs() < 1e-5);
        assert_eq!(drive.velocity((0.0, 0.0)), Float3::ZERO);
    }

    #[test]
    fn headings() {
        assert_eq!(heading((0.0, 1.0)), 0);
        assert_eq!(heading((1.0, 0.0)), 16_384);
        assert_eq!(heading((-1.0, 0.0)), -16_384);
        assert_eq!(heading((0.0, -1.0)), -32_767);
    }
}

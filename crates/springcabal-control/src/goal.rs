//! Rules side: steering a unit with a held direction, as a move goal a
//! short way ahead of it.
//!
//! The unit keeps its own movement physics (acceleration, turning, slopes,
//! collisions); the goal only says where to head. Re-issue it every frame
//! (or every few) while a direction is held, so the goal stays ahead of the
//! unit. For direct control of the velocity instead, see
//! [`velocity`](crate::velocity).

#[cfg(target_arch = "wasm32")]
use spring::Float3;

/// Where a held direction sends a unit: `reach` elmos ahead of it.
///
/// ```ignore
/// const STEER: MoveGoal = MoveGoal::ahead(120.0).radius(16.0);
/// if let Some(direction) = steering.direction() {
///     STEER.apply(unit_id, direction)?;
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveGoal {
    reach: f32,
    radius: f32,
    raw: bool,
    normalize: bool,
    bounds: Bounds,
    height: GoalHeight,
}

/// Where a goal may lie.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bounds {
    Unbounded,
    /// Inside this rectangle.
    Rect(Rect, Clamp),
    /// Inside the map, `clearance` elmos from its edges.
    Map {
        clearance: f32,
        clamp: Clamp,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub min_x: f32,
    pub min_z: f32,
    pub max_x: f32,
    pub max_z: f32,
}

/// How a goal outside the bounds is brought back in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clamp {
    /// Clamp x and z separately. Near an edge a diagonal turns into movement
    /// along the edge.
    PerAxis,
    /// Shorten the offset until it fits, keeping its heading. Near an edge a
    /// diagonal slows down instead of turning, and the unit stops at the edge.
    KeepHeading,
}

/// The height of the goal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoalHeight {
    /// The ground under the goal.
    Ground,
    /// The unit's own height.
    Unit,
}

impl MoveGoal {
    /// A goal `reach` elmos ahead, in the direction's heading.
    ///
    /// Defaults: goal radius 16, a raw goal (straight there, no path
    /// finding), the direction normalized, kept inside the map with its
    /// heading (1 elmo clearance), at ground height.
    pub const fn ahead(reach: f32) -> Self {
        Self {
            reach,
            radius: 16.0,
            raw: true,
            normalize: true,
            bounds: Bounds::Map {
                clearance: 1.0,
                clamp: Clamp::KeepHeading,
            },
            height: GoalHeight::Ground,
        }
    }

    pub const fn reach(mut self, reach: f32) -> Self {
        self.reach = reach;
        self
    }

    /// How close counts as arrived.
    pub const fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// `false` lets the path finder route around obstacles, which suits long
    /// goals; `true` heads straight for it.
    pub const fn raw(mut self, raw: bool) -> Self {
        self.raw = raw;
        self
    }

    /// Scale the direction by `reach` as it is, so a diagonal of two keys
    /// reaches √2 times as far.
    pub const fn unnormalized(mut self) -> Self {
        self.normalize = false;
        self
    }

    pub const fn bounds(mut self, bounds: Bounds) -> Self {
        self.bounds = bounds;
        self
    }

    pub const fn height(mut self, height: GoalHeight) -> Self {
        self.height = height;
        self
    }

    /// The goal's x and z for a unit at `(x, z)` heading in `direction`,
    /// clamped to `limits`. `None` for no direction.
    pub fn offset(
        &self,
        (x, z): (f32, f32),
        (dx, dz): (f32, f32),
        limits: Option<(Rect, Clamp)>,
    ) -> Option<(f32, f32)> {
        let (dx, dz) = if self.normalize {
            let length = (dx * dx + dz * dz).sqrt();
            if length.is_nan() || length <= f32::EPSILON {
                return None;
            }
            (dx / length * self.reach, dz / length * self.reach)
        } else {
            if (dx == 0.0 && dz == 0.0) || !dx.is_finite() || !dz.is_finite() {
                return None;
            }
            (self.reach * dx, self.reach * dz)
        };
        let Some((rect, clamp)) = limits else {
            return Some((x + dx, z + dz));
        };
        Some(match clamp {
            Clamp::PerAxis => (
                (x + dx).clamp(rect.min_x, rect.max_x),
                (z + dz).clamp(rect.min_z, rect.max_z),
            ),
            Clamp::KeepHeading => {
                let mut scale: f32 = 1.0;
                let mut limit = |start: f32, delta: f32, lo: f32, hi: f32| {
                    if delta > 0.0 && start + delta > hi {
                        scale = scale.min((hi - start) / delta);
                    } else if delta < 0.0 && start + delta < lo {
                        scale = scale.min((lo - start) / delta);
                    }
                };
                limit(x, dx, rect.min_x, rect.max_x);
                limit(z, dz, rect.min_z, rect.max_z);
                let scale = scale.max(0.0);
                (x + dx * scale, z + dz * scale)
            }
        })
    }
}

#[cfg(target_arch = "wasm32")]
impl MoveGoal {
    /// The goal for a unit at `from` heading in `direction`, or `None` for
    /// no direction.
    pub fn goal(&self, from: Float3, direction: (f32, f32)) -> Option<Float3> {
        let limits = match self.bounds {
            Bounds::Unbounded => None,
            Bounds::Rect(rect, clamp) => Some((rect, clamp)),
            Bounds::Map { clearance, clamp } => map_rect(clearance).map(|rect| (rect, clamp)),
        };
        let (x, z) = self.offset((from.x, from.z), direction, limits)?;
        let y = match self.height {
            GoalHeight::Ground => spring::get_ground_height(x, z).unwrap_or(0.0),
            GoalHeight::Unit => from.y,
        };
        Some(Float3 { x, y, z })
    }

    /// Send the unit towards `goal` with this radius and rawness.
    pub fn set(&self, unit_id: i32, goal: Float3) -> spring::Result<bool> {
        spring::set_unit_move_goal(unit_id, goal, self.radius, 0.0, self.raw)
    }

    /// Head the unit in `direction` from where it stands. `Ok(false)` when
    /// there is no direction or the engine refused the goal.
    pub fn apply(&self, unit_id: i32, direction: (f32, f32)) -> spring::Result<bool> {
        let position = spring::get_unit_position(unit_id, false, false)?;
        self.apply_from(unit_id, position, direction)
    }

    /// Like [`apply`](Self::apply), from a position the caller already has
    /// (the unit's mid position, say).
    pub fn apply_from(
        &self,
        unit_id: i32,
        from: Float3,
        direction: (f32, f32),
    ) -> spring::Result<bool> {
        match self.goal(from, direction) {
            Some(goal) => self.set(unit_id, goal),
            None => Ok(false),
        }
    }
}

/// The map in elmos, `clearance` from each edge.
#[cfg(target_arch = "wasm32")]
fn map_rect(clearance: f32) -> Option<Rect> {
    // Heightmap points: one more than the map's squares.
    const SQUARE_SIZE: f32 = 8.0;
    let [points_x, points_z] = spring::get_height_map_size().ok()?;
    Some(Rect {
        min_x: clearance,
        min_z: clearance,
        max_x: (points_x - 1) as f32 * SQUARE_SIZE - clearance,
        max_z: (points_z - 1) as f32 * SQUARE_SIZE - clearance,
    })
}

/// The held direction on the rules side, and when to stop.
///
/// Keep one per controlled unit; feed it the decoded message with
/// [`set`](Self::set). While a direction is held, apply it every frame (see
/// [`MoveGoal`]). Once it is released, [`take_stop`](Self::take_stop) says
/// once that the unit should stop (e.g. a `STOP` order, so it doesn't coast
/// on to the last goal).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Steering {
    direction: Option<(f32, f32)>,
    stopped: bool,
}

impl Default for Steering {
    fn default() -> Self {
        Self::new()
    }
}

impl Steering {
    pub const fn new() -> Self {
        Self {
            direction: None,
            stopped: true,
        }
    }

    /// The direction from the latest message; `(0, 0)` releases it.
    pub fn set(&mut self, (x, z): (f32, f32)) {
        if x == 0.0 && z == 0.0 {
            self.direction = None;
        } else {
            self.direction = Some((x, z));
            self.stopped = false;
        }
    }

    /// Release the direction, e.g. when input is no longer accepted.
    pub fn release(&mut self) {
        self.direction = None;
    }

    /// Release the direction without asking for a stop, e.g. when the game
    /// takes over the unit.
    pub fn release_quietly(&mut self) {
        self.direction = None;
        self.stopped = true;
    }

    /// The held direction, or `None` when released.
    pub fn direction(&self) -> Option<(f32, f32)> {
        self.direction
    }

    /// `true` once after the direction is released.
    pub fn take_stop(&mut self) -> bool {
        let stop = self.direction.is_none() && !self.stopped;
        self.stopped |= stop;
        stop
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: Rect = Rect {
        min_x: 1.0,
        min_z: 1.0,
        max_x: 999.0,
        max_z: 999.0,
    };

    #[test]
    fn normalized_diagonal() {
        let goal = MoveGoal::ahead(100.0);
        let (x, z) = goal.offset((500.0, 500.0), (1.0, -1.0), None).unwrap();
        assert!((x - 570.710_7).abs() < 1e-3 && (z - 429.289_3).abs() < 1e-3);
        assert_eq!(goal.offset((0.0, 0.0), (0.0, 0.0), None), None);
    }

    #[test]
    fn unnormalized_scales() {
        let goal = MoveGoal::ahead(32.0).unnormalized();
        assert_eq!(
            goal.offset((10.0, 10.0), (1.0, 1.0), None),
            Some((42.0, 42.0))
        );
        assert_eq!(goal.offset((10.0, 10.0), (0.0, 0.0), None), None);
    }

    #[test]
    fn clamps() {
        let goal = MoveGoal::ahead(100.0).unnormalized();
        // 50 elmos from the right edge, heading right and down.
        let per_axis = goal.offset((949.0, 500.0), (1.0, 1.0), Some((MAP, Clamp::PerAxis)));
        assert_eq!(per_axis, Some((999.0, 600.0)));
        let heading = goal.offset((949.0, 500.0), (1.0, 1.0), Some((MAP, Clamp::KeepHeading)));
        assert_eq!(heading, Some((999.0, 550.0)));
        // Already past the edge: stays put.
        let outside = goal.offset((0.0, 500.0), (-1.0, 0.0), Some((MAP, Clamp::KeepHeading)));
        assert_eq!(outside, Some((0.0, 500.0)));
    }

    #[test]
    fn steering_stops_once_after_moving() {
        let mut steering = Steering::new();
        assert!(!steering.take_stop());
        steering.set((1.0, 0.0));
        assert_eq!(steering.direction(), Some((1.0, 0.0)));
        assert!(!steering.take_stop());
        steering.set((0.0, 0.0));
        assert_eq!(steering.direction(), None);
        assert!(steering.take_stop());
        assert!(!steering.take_stop());
        steering.set((0.0, 1.0));
        steering.release_quietly();
        assert!(!steering.take_stop());
    }
}

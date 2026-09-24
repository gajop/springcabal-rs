//! UI side: placing the camera, and a camera that follows a unit.
//!
//! [`View`] describes a camera (controller mode, pitch, height or distance)
//! independently of where it looks. [`CameraPlacement`] puts it somewhere
//! once. [`CameraFollow`] keeps it on a unit: it samples a point of the unit
//! each update, leads it by the unit's velocity, smooths the result, snaps
//! on the first frame and whenever the unit changes, and hands it to the
//! engine either as a full camera state or as a camera target.

use spring::Float3;

/// Spring camera controllers, by the `mode` number the engine uses.
pub mod mode {
    /// Overhead ("ta"): `height` above the point looked at, tilted by
    /// `angle`.
    pub const OVERHEAD: i32 = 1;
    /// Spring: orbits the point at `dist`, rotated by `rx` / `ry`.
    pub const SPRING: i32 = 2;
}

/// A camera, apart from where it looks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub mode: i32,
    /// Where the camera looks: `(0, y, z)` for no rotation around the
    /// vertical axis.
    pub direction: Float3,
    /// Field of view in degrees.
    pub fov: f32,
    /// Overhead: the camera's height above the point.
    pub height: f32,
    /// Overhead: the tilt.
    pub angle: f32,
    /// Spring: distance from the point. Zero keeps the current one.
    pub dist: f32,
    /// Spring: rotation around the x (pitch) and y (yaw) axes.
    pub rx: f32,
    pub ry: f32,
}

impl View {
    const DEFAULT: Self = Self {
        mode: mode::OVERHEAD,
        direction: Float3::new(0.0, -1.0, 0.0),
        fov: 45.0,
        height: 0.0,
        angle: 0.0,
        dist: 0.0,
        rx: 0.0,
        ry: 0.0,
    };

    /// The overhead camera, `height` above the point and tilted by `angle`.
    pub const fn overhead(height: f32, angle: f32) -> Self {
        Self {
            mode: mode::OVERHEAD,
            height,
            angle,
            ..Self::DEFAULT
        }
    }

    /// The spring camera, `dist` from the point and pitched by `rx`.
    pub const fn spring(dist: f32, rx: f32) -> Self {
        Self {
            mode: mode::SPRING,
            dist,
            rx,
            ..Self::DEFAULT
        }
    }

    /// Look along `(0, y, z)`: down (`y` < 0) and away from the viewer
    /// (`z` < 0).
    pub const fn pitch(mut self, y: f32, z: f32) -> Self {
        self.direction = Float3::new(0.0, y, z);
        self
    }

    pub const fn fov(mut self, degrees: f32) -> Self {
        self.fov = degrees;
        self
    }

    pub const fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub const fn angle(mut self, angle: f32) -> Self {
        self.angle = angle;
        self
    }

    pub const fn ry(mut self, ry: f32) -> Self {
        self.ry = ry;
        self
    }

    /// Perpendicular to the direction, in the vertical plane (for a
    /// direction without yaw).
    pub const fn up(&self) -> Float3 {
        Float3::new(0.0, -self.direction.z, self.direction.y)
    }

    /// The engine's name for the controller.
    pub const fn controller_name(&self) -> &'static str {
        if self.mode == mode::SPRING {
            "spring"
        } else {
            "ta"
        }
    }
}

/// Which point of the unit the camera follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aim {
    /// The simulated position (moves in sim-frame steps).
    Position,
    MidPosition,
    /// The drawn position, interpolated between sim frames: smoother at
    /// high frame rates.
    ViewPosition,
    ViewMidPosition,
}

/// How the camera catches up with the point it follows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Smoothing {
    /// Stay on it.
    None,
    /// Close the gap exponentially with time constant `tau` seconds,
    /// independently of the frame rate.
    Exponential { tau: f32 },
    /// Move by the point's average movement over the last `updates`
    /// updates (at most [`Smoother::MAX_AVERAGE`]). Per update, not per
    /// second.
    AverageMove { updates: usize },
}

/// How the followed point reaches the engine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Output {
    /// A full camera state at the point, with this view.
    State {
        view: View,
        /// The camera's height from the point's: see [`FocusY`].
        y: FocusY,
        /// Seconds the engine takes to move there.
        transition: f32,
    },
    /// Move the current camera's target (`SetCameraTarget`), keeping its
    /// controller, direction and distance.
    Target { transition: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FocusY {
    /// The point's height plus this.
    Offset(f32),
    /// Always this height.
    Fixed(f32),
}

/// A follow camera's settings.
///
/// ```ignore
/// const FOLLOW: Follow = Follow::state(View::overhead(2000.0, 0.4).pitch(-0.92, -0.39), 0.4)
///     .aim(Aim::ViewMidPosition)
///     .lead(12.0)
///     .smoothing(Smoothing::AverageMove { updates: 12 });
/// handler.add(Box::new(CameraFollowWidget::new(FOLLOW, |global: &Global| global.player())));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Follow {
    start: Option<(View, Float3)>,
    aim: Aim,
    lead: f32,
    smoothing: Smoothing,
    output: Output,
    snap_updates: u8,
    engine_tracker: bool,
    block_zoom: bool,
}

impl Follow {
    const fn new(output: Output) -> Self {
        Self {
            start: None,
            aim: Aim::ViewPosition,
            lead: 0.0,
            smoothing: Smoothing::None,
            output,
            snap_updates: 1,
            engine_tracker: false,
            block_zoom: false,
        }
    }

    /// Follow with full camera states of `view`, moving there over
    /// `transition` seconds.
    pub const fn state(view: View, transition: f32) -> Self {
        Self::new(Output::State {
            view,
            y: FocusY::Offset(0.0),
            transition,
        })
    }

    /// Follow by moving the camera's target, keeping whatever view it has
    /// (usually the one [`start`](Self::start) placed).
    pub const fn target(transition: f32) -> Self {
        Self::new(Output::Target { transition })
    }

    /// Place the camera with `view` at `position` first, before there is a
    /// unit to follow. Retried until the engine accepts it or following
    /// starts.
    pub const fn start(mut self, view: View, position: Float3) -> Self {
        self.start = Some((view, position));
        self
    }

    pub const fn aim(mut self, aim: Aim) -> Self {
        self.aim = aim;
        self
    }

    /// Look ahead of the unit by `frames` times its velocity (elmos per sim
    /// frame), so the player sees where it is going.
    pub const fn lead(mut self, frames: f32) -> Self {
        self.lead = frames;
        self
    }

    pub const fn smoothing(mut self, smoothing: Smoothing) -> Self {
        self.smoothing = smoothing;
        self
    }

    /// With [`state`](Self::state): the camera's height, from the point's.
    pub const fn focus_y(mut self, y: FocusY) -> Self {
        if let Output::State {
            view, transition, ..
        } = self.output
        {
            self.output = Output::State {
                view,
                y,
                transition,
            };
        }
        self
    }

    /// How many accepted updates jump straight to the unit, with no
    /// transition, after the camera first finds it or it changes (default 1).
    /// Without it the first frames sweep from wherever the camera was.
    pub const fn snap_updates(mut self, updates: u8) -> Self {
        self.snap_updates = updates;
        self
    }

    /// Also arm the engine's unit tracker (`track`, mode 1) on the unit,
    /// which keeps the camera on it between our updates.
    pub const fn engine_tracker(mut self) -> Self {
        self.engine_tracker = true;
        self
    }

    /// Claim the mouse wheel, so the engine doesn't zoom the camera away.
    pub const fn block_zoom(mut self) -> Self {
        self.block_zoom = true;
        self
    }
}

/// The followed point, smoothed. Engine-free, so it can be tested.
pub struct Smoother {
    smoothing: Smoothing,
    focus: Float3,
    last: Float3,
    moves: [Float3; Self::MAX_AVERAGE],
    sum: Float3,
    next: usize,
}

impl Smoother {
    /// The longest [`Smoothing::AverageMove`] window.
    pub const MAX_AVERAGE: usize = 32;

    /// Start on `point`.
    pub fn new(smoothing: Smoothing, point: Float3) -> Self {
        Self {
            smoothing,
            focus: point,
            last: point,
            moves: [Float3::ZERO; Self::MAX_AVERAGE],
            sum: Float3::ZERO,
            next: 0,
        }
    }

    pub fn focus(&self) -> Float3 {
        self.focus
    }

    /// Follow `point` for an update `dt` seconds long; returns the new
    /// focus.
    pub fn step(&mut self, point: Float3, dt: f32) -> Float3 {
        match self.smoothing {
            Smoothing::None => self.focus = point,
            Smoothing::Exponential { tau } => {
                if dt.is_finite() && dt > 0.0 {
                    let alpha = 1.0 - libm::expf(-(dt / tau));
                    let focus = &mut self.focus;
                    focus.x += (point.x - focus.x) * alpha;
                    focus.y += (point.y - focus.y) * alpha;
                    focus.z += (point.z - focus.z) * alpha;
                }
            }
            Smoothing::AverageMove { updates } => {
                let count = updates.clamp(1, Self::MAX_AVERAGE);
                let delta = Float3::new(
                    point.x - self.last.x,
                    point.y - self.last.y,
                    point.z - self.last.z,
                );
                self.last = point;
                let previous = core::mem::replace(&mut self.moves[self.next], delta);
                self.next = (self.next + 1) % count;
                self.sum.x += delta.x - previous.x;
                self.sum.y += delta.y - previous.y;
                self.sum.z += delta.z - previous.z;
                let count = count as f32;
                self.focus.x += self.sum.x / count;
                self.focus.y += self.sum.y / count;
                self.focus.z += self.sum.z / count;
            }
        }
        self.focus
    }
}

#[cfg(target_arch = "wasm32")]
pub use engine::*;

#[cfg(target_arch = "wasm32")]
mod engine {
    use core::cell::{Cell, RefCell};

    use spring::Float3;
    use spring_addons::{AddonContext, EventResult, Widget};

    use super::{Aim, FocusY, Follow, Output, Smoother, View};

    impl View {
        /// The engine camera state for this view at `position`.
        pub const fn state(&self, position: Float3) -> spring::CameraState<'static> {
            spring::CameraState {
                name: self.controller_name(),
                position,
                direction: self.direction,
                up: self.up(),
                right: Float3::new(1.0, 0.0, 0.0),
                fov: self.fov,
                rx: self.rx,
                ry: self.ry,
                rz: 0.0,
                dist: self.dist,
                height: self.height,
                angle: self.angle,
                mode: self.mode,
            }
        }
    }

    /// Set the camera to `view` at `position` at once. Returns whether the
    /// engine accepted it.
    pub fn place_camera(view: &View, position: Float3) -> bool {
        spring::set_camera_state(view.state(position), 0.0, 1.0, 1.0).is_ok_and(|ok| ok)
    }

    /// A widget that places the camera once, retrying until the engine
    /// accepts it. For games whose camera doesn't move.
    pub struct CameraPlacement {
        view: View,
        position: Float3,
        placed: Cell<bool>,
        block_zoom: bool,
    }

    impl CameraPlacement {
        pub const fn new(view: View, position: Float3) -> Self {
            Self {
                view,
                position,
                placed: Cell::new(false),
                block_zoom: false,
            }
        }

        /// Claim the mouse wheel, so the engine doesn't zoom.
        pub const fn block_zoom(mut self) -> Self {
            self.block_zoom = true;
            self
        }
    }

    impl<G> Widget<G> for CameraPlacement {
        fn name(&self) -> &'static str {
            "CameraPlacement"
        }

        fn update(&self, _ctx: &AddonContext<'_, G>, _dt: f32) {
            if !self.placed.get() {
                self.placed.set(place_camera(&self.view, self.position));
            }
        }

        fn mouse_wheel(&self, _ctx: &AddonContext<'_, G>, _up: bool, _value: f32) -> EventResult {
            self.block_zoom.into()
        }
    }

    #[derive(Default)]
    struct State {
        placed: bool,
        /// The unit followed, and the smoothed point.
        followed: Option<(i32, Smoother)>,
        snaps_left: u8,
        /// The unit the engine tracker was armed on.
        tracked: Option<i32>,
    }

    /// A camera that follows a unit. Embed it in a widget and call
    /// [`update`](Self::update) every update, or use [`CameraFollowWidget`].
    pub struct CameraFollow {
        config: Follow,
        state: RefCell<State>,
    }

    impl CameraFollow {
        pub fn new(config: Follow) -> Self {
            Self {
                config,
                state: RefCell::new(State::default()),
            }
        }

        pub fn config(&self) -> &Follow {
            &self.config
        }

        /// Follow `unit` (or nobody) for an update `dt` seconds long.
        pub fn update(&self, unit: Option<i32>, dt: f32) {
            let config = &self.config;
            if let Some((view, position)) = config.start {
                let state = &mut *self.state.borrow_mut();
                // Following takes over from the start camera, so stop
                // retrying it then.
                if !state.placed && state.followed.is_none() {
                    state.placed = place_camera(&view, position);
                }
            }
            let Some(unit) = unit else {
                let mut state = self.state.borrow_mut();
                state.followed = None;
                state.tracked = None;
                return;
            };
            let Some(point) = self.point(unit) else {
                return;
            };
            if config.engine_tracker && self.state.borrow().tracked != Some(unit) {
                arm_tracker(unit);
                self.state.borrow_mut().tracked = Some(unit);
            }

            let (focus, snap) = {
                let state = &mut *self.state.borrow_mut();
                let focus = match &mut state.followed {
                    Some((followed, smoother)) if *followed == unit => smoother.step(point, dt),
                    followed => {
                        *followed = Some((unit, Smoother::new(config.smoothing, point)));
                        state.snaps_left = config.snap_updates;
                        point
                    }
                };
                (focus, state.snaps_left > 0)
            };

            let accepted = match config.output {
                Output::State {
                    view,
                    y,
                    transition,
                } => {
                    let y = match y {
                        FocusY::Offset(offset) => focus.y + offset,
                        FocusY::Fixed(y) => y,
                    };
                    let position = Float3::new(focus.x, y, focus.z);
                    let transition = if snap { 0.0 } else { transition };
                    spring::set_camera_state(view.state(position), transition, 1.0, 1.0)
                }
                Output::Target { transition } => spring::camera::set_camera_target(
                    focus,
                    spring::camera::SetCameraTargetOptions {
                        transition_time: Some(if snap { 0.0 } else { transition }),
                        dir_x: None,
                        dir_y: None,
                        dir_z: None,
                    },
                ),
            };
            if snap && accepted.is_ok_and(|ok| ok) {
                let mut state = self.state.borrow_mut();
                state.snaps_left = state.snaps_left.saturating_sub(1);
            }
        }

        /// Place the start view again and snap to the unit, e.g. when a
        /// round restarts.
        pub fn reset(&self) {
            *self.state.borrow_mut() = State::default();
        }

        /// The unit's point this update, led by its velocity.
        fn point(&self, unit: i32) -> Option<Float3> {
            let position = match self.config.aim {
                Aim::Position | Aim::MidPosition => {
                    spring::get_unit_position(unit, self.config.aim == Aim::MidPosition, false)
                }
                Aim::ViewPosition | Aim::ViewMidPosition => {
                    spring::unit_rendering::get_unit_view_position(
                        unit,
                        self.config.aim == Aim::ViewMidPosition,
                    )
                }
            }
            .ok()?;
            let lead = self.config.lead;
            if lead == 0.0 {
                return Some(position);
            }
            let velocity = spring::get_unit_velocity(unit).unwrap_or_default();
            Some(Float3::new(
                position.x + velocity.x * lead,
                position.y + velocity.y * lead,
                position.z + velocity.z * lead,
            ))
        }
    }

    /// The engine's tracker follows the selected unit, so select it, arm the
    /// tracker, and select nothing again.
    fn arm_tracker(unit: i32) {
        let _ = spring::selection::select_unit_array(&[unit], false);
        for command in ["trackoff", "track", "trackmode 1"] {
            if let Err(error) = spring::send_commands(command, "") {
                spring::log_warning!("springcabal", "camera `{command}` failed: {error:?}");
            }
        }
        let _ = spring::selection::select_unit_array(&[], false);
    }

    /// A widget with a [`CameraFollow`] on the unit `target` names.
    pub struct CameraFollowWidget<G> {
        camera: CameraFollow,
        target: fn(&G) -> Option<i32>,
    }

    impl<G> CameraFollowWidget<G> {
        pub fn new(config: Follow, target: fn(&G) -> Option<i32>) -> Self {
            Self {
                camera: CameraFollow::new(config),
                target,
            }
        }
    }

    impl<G> Widget<G> for CameraFollowWidget<G> {
        fn name(&self) -> &'static str {
            "CameraFollow"
        }

        fn update(&self, ctx: &AddonContext<'_, G>, dt: f32) {
            self.camera.update((self.target)(ctx.global()), dt);
        }

        fn mouse_wheel(&self, _ctx: &AddonContext<'_, G>, _up: bool, _value: f32) -> EventResult {
            self.camera.config.block_zoom.into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_state() {
        let view = View::overhead(2000.0, 0.4).pitch(-0.9, -0.4);
        assert_eq!((view.controller_name(), view.mode), ("ta", mode::OVERHEAD));
        assert_eq!(view.up(), Float3::new(0.0, 0.4, -0.9));
        assert_eq!(View::spring(500.0, 2.5).controller_name(), "spring");
    }

    #[test]
    fn exponential_is_frame_rate_independent() {
        let target = Float3::new(100.0, 0.0, 0.0);
        let smoothing = Smoothing::Exponential { tau: 0.1 };
        let mut once = Smoother::new(smoothing, Float3::ZERO);
        let mut twice = Smoother::new(smoothing, Float3::ZERO);
        once.step(target, 0.2);
        twice.step(target, 0.1);
        twice.step(target, 0.1);
        assert!((once.focus().x - twice.focus().x).abs() < 1e-3);
        assert!(once.focus().x > 80.0 && once.focus().x < 100.0);
        // A zero or broken dt doesn't move it.
        assert_eq!(once.step(target, f32::NAN), once.focus());
    }

    #[test]
    fn average_move_catches_up() {
        let mut smoother = Smoother::new(Smoothing::AverageMove { updates: 4 }, Float3::ZERO);
        // The point moves 4 per update: the camera ramps up to that speed.
        let xs: alloc::vec::Vec<f32> = (1..=6)
            .map(|i| smoother.step(Float3::new(4.0 * i as f32, 0.0, 0.0), 0.0).x)
            .collect();
        assert_eq!(xs, [1.0, 3.0, 6.0, 10.0, 14.0, 18.0]);
    }

    #[test]
    fn follow_builder() {
        let follow = Follow::state(View::overhead(1.0, 0.0), 0.4).focus_y(FocusY::Fixed(0.0));
        assert!(matches!(
            follow.output,
            Output::State {
                y: FocusY::Fixed(_),
                transition: 0.4,
                ..
            }
        ));
    }
}

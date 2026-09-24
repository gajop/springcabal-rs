//! Screenshots on a schedule, for the visual verification suites.
//!
//! The suites compare the screenshots with goldens and quit the engine
//! themselves once they see the done marker (`vf_quit_when`), so the widget
//! only takes the shots and says when it's finished.

#[cfg(target_arch = "wasm32")]
use alloc::format;
use alloc::vec::Vec;
use core::cell::Cell;

#[cfg(target_arch = "wasm32")]
use spring::ResultExt;
#[cfg(target_arch = "wasm32")]
use spring_addons::{AddonContext, Widget};

/// What the schedule counts in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Clock {
    /// Game frames: repeatable across machines. Needs the game to be running.
    GameFrame,
    /// Widget updates (draw frames): also runs before the game starts, but
    /// depends on the frame rate.
    Update,
}

/// Takes one screenshot at each listed time, then logs `done_marker`.
///
/// ```ignore
/// handler.add(Screenshots::new(Clock::GameFrame, &[25, 90, 210]).done_marker("MYCUBE VISUAL E2E PASSED"));
/// ```
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub struct Screenshots {
    clock: Clock,
    at: Vec<u32>,
    done_marker: Option<&'static str>,
    quit_after: Option<u32>,
    quit_at: Cell<Option<u32>>,
    updates: Cell<u32>,
    taken: Cell<usize>,
}

impl Screenshots {
    /// `at` must be strictly ascending; anything else panics, so a
    /// mistyped schedule fails the run instead of skipping shots.
    pub fn new(clock: Clock, at: &[u32]) -> Self {
        assert!(
            at.windows(2).all(|pair| pair[0] < pair[1]),
            "screenshot times must be strictly ascending: {at:?}"
        );
        Self {
            clock,
            at: at.to_vec(),
            done_marker: None,
            quit_after: None,
            quit_at: Cell::new(None),
            updates: Cell::new(0),
            taken: Cell::new(0),
        }
    }

    /// Echo this line after the last screenshot.
    pub fn done_marker(mut self, marker: &'static str) -> Self {
        self.done_marker = Some(marker);
        self
    }

    /// Quit the engine this many clock ticks after the last screenshot, for
    /// suites that don't quit on the done marker themselves.
    pub fn quit_after(mut self, ticks: u32) -> Self {
        self.quit_after = Some(ticks);
        self
    }

    pub fn is_done(&self) -> bool {
        self.taken.get() >= self.at.len()
    }
}

#[cfg(target_arch = "wasm32")]
impl Screenshots {
    fn now(&self) -> Option<u32> {
        match self.clock {
            // Before the game starts the engine reports frame -1 (`u32::MAX`).
            Clock::GameFrame => spring::game::frame()
                .ok()
                .filter(|frame| i32::try_from(*frame).is_ok()),
            Clock::Update => {
                let updates = self.updates.get();
                self.updates.set(updates + 1);
                Some(updates)
            }
        }
    }

    fn tick(&self) {
        let Some(now) = self.now() else { return };
        match self.step(now) {
            Action::Wait => {}
            Action::Quit => {
                spring::system_control::quit(0).log_err("quit after screenshots");
            }
            Action::Shoot { index, last } => {
                screenshot(&format!("{} at {now}", index + 1));
                if last && let Some(marker) = self.done_marker {
                    spring::echo(marker, "").log_err("screenshots done marker");
                }
            }
        }
    }
}

impl Screenshots {
    /// Advance the schedule to `now` and say what to do.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn step(&self, now: u32) -> Action {
        if self.quit_at.get().is_some_and(|at| now >= at) {
            self.quit_at.set(None);
            return Action::Quit;
        }
        let index = self.taken.get();
        let Some(&due) = self.at.get(index) else {
            return Action::Wait;
        };
        if now < due {
            return Action::Wait;
        }
        self.taken.set(index + 1);
        let last = self.is_done();
        if last {
            self.quit_at.set(self.quit_after.map(|ticks| now + ticks));
        }
        Action::Shoot { index, last }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Debug, PartialEq)]
enum Action {
    Wait,
    /// Take shot `index`; `last` if it ends the schedule.
    Shoot {
        index: usize,
        last: bool,
    },
    Quit,
}

#[cfg(target_arch = "wasm32")]
impl<G> Widget<G> for Screenshots {
    fn name(&self) -> &'static str {
        "DbgScreenshots"
    }

    fn update(&self, _ctx: &AddonContext<'_, G>, _dt: f32) {
        self.tick();
    }
}

#[cfg(target_arch = "wasm32")]
/// Take a screenshot now and log `E2E screenshot {label}` so a failing golden
/// can be traced to the moment it was taken.
pub fn screenshot(label: &str) {
    spring::echo(&format!("E2E screenshot {label}"), "").log_err("screenshot label");
    if let Err(error) = spring::send_command("screenshot") {
        spring::log_warning!("springcabal", "screenshot {label} failed: {error:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shoots_in_order_then_quits() {
        let shots = Screenshots::new(Clock::Update, &[2, 5]).quit_after(3);
        let actions: Vec<_> = (0..10).map(|now| shots.step(now)).collect();
        assert_eq!(
            actions,
            [
                Action::Wait,
                Action::Wait,
                Action::Shoot {
                    index: 0,
                    last: false
                },
                Action::Wait,
                Action::Wait,
                Action::Shoot {
                    index: 1,
                    last: true
                },
                Action::Wait,
                Action::Wait,
                Action::Quit,
                Action::Wait,
            ]
        );
        assert!(shots.is_done());
    }

    #[test]
    fn a_late_clock_takes_one_shot_per_tick() {
        let shots = Screenshots::new(Clock::GameFrame, &[1, 2]);
        assert_eq!(
            shots.step(50),
            Action::Shoot {
                index: 0,
                last: false
            }
        );
        assert_eq!(
            shots.step(51),
            Action::Shoot {
                index: 1,
                last: true
            }
        );
        assert_eq!(shots.step(52), Action::Wait);
    }

    #[test]
    #[should_panic(expected = "strictly ascending")]
    fn unordered_schedule_is_refused() {
        let _ = Screenshots::new(Clock::Update, &[5, 2]);
    }
}

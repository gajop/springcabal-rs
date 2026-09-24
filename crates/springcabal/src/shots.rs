//! Screenshots on a schedule, for the visual verification suites.
//!
//! The suites compare the screenshots with goldens and quit the engine
//! themselves once they see the done marker (`vf_quit_when`), so the widget
//! only takes the shots and says when it's finished.

use alloc::format;
use alloc::vec::Vec;
use core::cell::Cell;

use spring::ResultExt;
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
    /// `at` must be ascending.
    pub fn new(clock: Clock, at: &[u32]) -> Self {
        debug_assert!(at.windows(2).all(|pair| pair[0] < pair[1]));
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
        if self.quit_at.get().is_some_and(|at| now >= at) {
            self.quit_at.set(None);
            spring::system_control::quit(0).log_err("quit after screenshots");
            return;
        }
        let index = self.taken.get();
        let Some(&due) = self.at.get(index) else {
            return;
        };
        if now < due {
            return;
        }
        screenshot(&format!("{} at {now}", index + 1));
        self.taken.set(index + 1);
        if self.is_done() {
            if let Some(marker) = self.done_marker {
                spring::echo(marker, "").log_err("screenshots done marker");
            }
            self.quit_at.set(self.quit_after.map(|ticks| now + ticks));
        }
    }
}

impl<G> Widget<G> for Screenshots {
    fn name(&self) -> &'static str {
        "dbg_screenshots"
    }

    fn update(&self, _ctx: &AddonContext<'_, G>, _dt: f32) {
        self.tick();
    }
}

/// Take a screenshot now and log `E2E screenshot {label}` so a failing golden
/// can be traced to the moment it was taken.
pub fn screenshot(label: &str) {
    spring::echo(&format!("E2E screenshot {label}"), "").log_err("screenshot label");
    if let Err(error) = spring::send_command("screenshot") {
        spring::log_warning!("springcabal", "screenshot {label} failed: {error:?}");
    }
}

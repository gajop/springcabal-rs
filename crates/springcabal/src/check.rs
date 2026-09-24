//! Assertion bookkeeping for the in-game integration tests.
//!
//! The verification suites (`springcabal-games/verification/games/*.sh`) read
//! these exact lines from the log, so their format is part of the contract:
//!
//! - `{tag} ASSERTION PASSED ({n}): {message}`
//! - `{tag} ASSERTION FAILED: {message}`
//! - `{tag} ALL {title} PASSED ({n} assertions)` when every check passed
//! - `{tag} {title} FAILED ({failed} of {total})` otherwise

use alloc::format;
use core::fmt::Display;

/// One test suite's running tally. Keep it in the test gadget that owns it.
pub struct Suite {
    tag: &'static str,
    title: &'static str,
    passed: u32,
    failed: u32,
    min_passed: u32,
    finished: bool,
}

impl Suite {
    /// `tag` prefixes every line (e.g. `"[TTLD-Rust-Test]"`); `title` names
    /// the suite in the final line (e.g. `"TO THE LAST DROP INTEGRATION TESTS"`).
    pub const fn new(tag: &'static str, title: &'static str) -> Self {
        Self {
            tag,
            title,
            passed: 0,
            failed: 0,
            min_passed: 0,
            finished: false,
        }
    }

    /// Fail the suite at [`finish`](Self::finish) unless at least `count`
    /// checks passed, so a run that silently skipped checks can't pass.
    pub const fn require_at_least(mut self, count: u32) -> Self {
        self.min_passed = count;
        self
    }

    /// Record one check and log it. Returns `ok`.
    pub fn check(&mut self, ok: bool, message: impl Display) -> bool {
        if ok {
            self.passed += 1;
            echo(&format!(
                "{} ASSERTION PASSED ({}): {message}",
                self.tag, self.passed
            ));
        } else {
            self.failed += 1;
            echo(&format!("{} ASSERTION FAILED: {message}", self.tag));
        }
        ok
    }

    /// Log the final line once. Returns whether the suite passed.
    pub fn finish(&mut self) -> bool {
        let ok = self.failed == 0 && self.passed >= self.min_passed;
        if self.finished {
            return ok;
        }
        self.finished = true;
        if ok {
            echo(&format!(
                "{} ALL {} PASSED ({} assertions)",
                self.tag, self.title, self.passed
            ));
        } else if self.failed == 0 {
            echo(&format!(
                "{} {} FAILED (only {} of at least {} checks ran)",
                self.tag, self.title, self.passed, self.min_passed
            ));
        } else {
            echo(&format!(
                "{} {} FAILED ({} of {})",
                self.tag,
                self.title,
                self.failed,
                self.passed + self.failed
            ));
        }
        ok
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn passed(&self) -> u32 {
        self.passed
    }

    pub fn failed(&self) -> u32 {
        self.failed
    }
}

fn echo(line: &str) {
    // `echo` goes to the infolog at every log level, which the suites read.
    if spring::echo(line, "").is_err() {
        let _ = spring::log("springcabal", spring::log_level::ERROR, line);
    }
}

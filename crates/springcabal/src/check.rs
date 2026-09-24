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
///
/// It passes only when every check passed and at least as many ran as the
/// suite expects, so a run that silently skipped checks can't pass.
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
    /// the suite in the final line (e.g. `"TO THE LAST DROP INTEGRATION TESTS"`);
    /// `expected` is how many checks a full run makes. It must be at least 1.
    pub const fn new(tag: &'static str, title: &'static str, expected: u32) -> Self {
        assert!(expected > 0, "a suite must expect at least one check");
        Self {
            tag,
            title,
            passed: 0,
            failed: 0,
            min_passed: expected,
            finished: false,
        }
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

#[cfg(target_arch = "wasm32")]
fn echo(line: &str) {
    // `echo` goes to the infolog at every log level, which the suites read.
    if spring::echo(line, "").is_err() {
        spring::log_error!("springcabal", "{line}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn echo(line: &str) {
    #[cfg(not(test))]
    let _ = line;
    #[cfg(test)]
    tests::LINES.with(|lines| lines.borrow_mut().push(alloc::string::String::from(line)));
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use alloc::string::String;
    use alloc::vec::Vec;
    use core::cell::RefCell;

    std::thread_local! {
        pub(super) static LINES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn lines() -> Vec<String> {
        LINES.with(|lines| lines.take())
    }

    #[test]
    fn passes_with_the_expected_count() {
        let mut suite = Suite::new("[T]", "T TESTS", 2);
        assert!(suite.check(true, "one"));
        assert!(suite.check(true, "two"));
        assert!(suite.finish());
        // The final line is logged once.
        assert!(suite.finish());
        assert_eq!(
            lines(),
            [
                "[T] ASSERTION PASSED (1): one",
                "[T] ASSERTION PASSED (2): two",
                "[T] ALL T TESTS PASSED (2 assertions)",
            ]
        );
    }

    #[test]
    fn fails_when_checks_are_missing() {
        let mut suite = Suite::new("[T]", "T TESTS", 3);
        suite.check(true, "one");
        assert!(!suite.finish());
        assert_eq!(
            lines().last().map(String::as_str),
            Some("[T] T TESTS FAILED (only 1 of at least 3 checks ran)")
        );
    }

    #[test]
    fn fails_on_a_failed_check() {
        let mut suite = Suite::new("[T]", "T TESTS", 1);
        assert!(!suite.check(false, "bad"));
        suite.check(true, "good");
        assert!(!suite.finish());
        assert_eq!(
            lines(),
            [
                "[T] ASSERTION FAILED: bad",
                "[T] ASSERTION PASSED (1): good",
                "[T] T TESTS FAILED (1 of 2)",
            ]
        );
        assert_eq!((suite.passed(), suite.failed()), (1, 1));
    }

    #[test]
    #[should_panic(expected = "at least one check")]
    fn zero_expected_is_refused() {
        let _ = Suite::new("[T]", "T TESTS", 0);
    }
}

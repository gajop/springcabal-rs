//! Module-wide state for a single-threaded Wasm guest.
//!
//! A Core Wasm guest runs on one thread, so a `static` may hold a `Cell`,
//! `RefCell` or `OnceCell`. Rust can't know that, so [`Local`] says it once
//! here instead of every game repeating `thread_local!` or
//! `unsafe impl Sync`. On the host (unit tests run on several threads) a
//! `Local` is not `Sync`, so it can only be a local value there.

use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::ops::Deref;

pub struct Local<T>(T);

// SAFETY: Core Wasm guests are single-threaded (no `atomics` target feature),
// so a `Local` is never reached from two threads.
#[cfg(target_arch = "wasm32")]
unsafe impl<T> Sync for Local<T> {}

impl<T> Local<T> {
    pub const fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for Local<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

/// Remembers which keys have been seen, e.g. to log a failure once per
/// message or per track rather than once per call site.
pub struct Seen(RefCell<Vec<String>>);

impl Default for Seen {
    fn default() -> Self {
        Self::new()
    }
}

impl Seen {
    pub const fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }

    /// `true` the first time `key` comes up.
    pub fn first(&self, key: &str) -> bool {
        let mut seen = self.0.borrow_mut();
        if seen.iter().any(|known| known == key) {
            return false;
        }
        seen.push(String::from(key));
        true
    }

    /// Forget `key`, so it counts as new again.
    pub fn forget(&self, key: &str) {
        self.0.borrow_mut().retain(|known| known != key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seen_once_per_key() {
        let seen = Local::new(Seen::new());
        assert!(seen.first("a"));
        assert!(!seen.first("a"));
        assert!(seen.first("b"));
        seen.forget("a");
        assert!(seen.first("a"));
    }
}

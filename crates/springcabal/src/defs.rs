//! Engine defs resolved once.
//!
//! Unit and weapon defs never change during a game, so each module looks up
//! the ids and per-def values it needs on first use and keeps them:
//!
//! ```ignore
//! static DEFS: DefCache<Defs> = DefCache::new();
//!
//! pub fn defs(spring: &Spring<'_, RulesSynced>) -> &'static Defs {
//!     DEFS.get_or_init(|| Defs::resolve(spring))
//! }
//! ```
//!
//! Resolve lazily, not in a `static` initializer or before the engine has
//! loaded the defs: the first `get_or_init` runs `resolve`, which must not
//! initialize the same cache recursively.

use std::sync::OnceLock;

pub struct DefCache<T> {
    value: OnceLock<T>,
}

impl<T> DefCache<T> {
    pub const fn new() -> Self {
        Self {
            value: OnceLock::new(),
        }
    }

    /// The defs, resolved on the first call.
    pub fn get_or_init(&self, resolve: impl FnOnce() -> T) -> &T {
        self.value.get_or_init(resolve)
    }
}

impl<T> Default for DefCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicU32, Ordering};

    static CALLS: AtomicU32 = AtomicU32::new(0);

    fn resolve() -> u32 {
        CALLS.fetch_add(1, Ordering::Relaxed) + 7
    }

    #[test]
    fn resolves_once() {
        let cache = DefCache::new();
        assert_eq!(*cache.get_or_init(resolve), 7);
        assert_eq!(*cache.get_or_init(resolve), 7);
        assert_eq!(CALLS.load(Ordering::Relaxed), 1);
    }
}

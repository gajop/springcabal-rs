//! Engine defs resolved once.
//!
//! Unit and weapon defs never change during a game, so each module looks up
//! the ids and per-def values it needs on first use and keeps them:
//!
//! ```ignore
//! static DEFS: DefCache<Defs> = DefCache::new(Defs::resolve);
//!
//! pub fn defs() -> &'static Defs {
//!     DEFS.get()
//! }
//! ```
//!
//! Resolve lazily, not in a `static` initializer or before the engine has
//! loaded the defs: the first `get` runs `resolve`, which must not call
//! `get` on the same cache.

use core::cell::OnceCell;

use crate::local::Local;

pub struct DefCache<T> {
    value: Local<OnceCell<T>>,
    resolve: fn() -> T,
}

impl<T> DefCache<T> {
    pub const fn new(resolve: fn() -> T) -> Self {
        Self {
            value: Local::new(OnceCell::new()),
            resolve,
        }
    }

    /// The defs, resolved on the first call.
    pub fn get(&self) -> &T {
        self.value.get_or_init(self.resolve)
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
        let cache = DefCache::new(resolve);
        assert_eq!(*cache.get(), 7);
        assert_eq!(*cache.get(), 7);
        assert_eq!(CALLS.load(Ordering::Relaxed), 1);
    }
}

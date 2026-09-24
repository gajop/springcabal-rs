//! UI side: hold a mouse button to keep moving towards the cursor.
//!
//! While the button is down the game samples the ground under the cursor
//! every update and asks [`HoldTarget::hold`] whether the rules need to
//! hear about it: they do when the command changes or the point has moved
//! far enough. When the button comes up, [`HoldTarget::release`] says
//! whether to send the stop.

use core::cell::Cell;

/// The command held and the last point sent for it. `K` names the command
/// (move, attack, ...).
pub struct HoldTarget<K: Copy> {
    last: Cell<Option<(K, f32, f32)>>,
    resend_distance: f32,
}

impl<K: Copy + PartialEq> HoldTarget<K> {
    /// Send the point again once it has moved `resend_distance` elmos.
    pub const fn new(resend_distance: f32) -> Self {
        Self {
            last: Cell::new(None),
            resend_distance,
        }
    }

    /// Hold `kind` towards `(x, z)`. Returns `true`, and remembers the point,
    /// when it should be sent.
    pub fn hold(&self, kind: K, (x, z): (f32, f32)) -> bool {
        let send = self.last.get().is_none_or(|(last, last_x, last_z)| {
            let (dx, dz) = (x - last_x, z - last_z);
            last != kind || dx * dx + dz * dz > self.resend_distance * self.resend_distance
        });
        if send {
            self.last.set(Some((kind, x, z)));
        }
        send
    }

    /// The button came up. Returns `true` when something was held, so the
    /// stop should be sent.
    pub fn release(&self) -> bool {
        self.last.take().is_some()
    }

    /// The command held, if any.
    pub fn held(&self) -> Option<K> {
        self.last.get().map(|(kind, _, _)| kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sends_on_change_and_distance() {
        let hold = HoldTarget::new(8.0);
        assert!(hold.hold("move", (0.0, 0.0)));
        assert!(!hold.hold("move", (5.0, 5.0)));
        assert!(hold.hold("move", (9.0, 0.0)));
        assert!(hold.hold("attack", (9.0, 0.0)));
        assert_eq!(hold.held(), Some("attack"));
        assert!(hold.release());
        assert!(!hold.release());
        assert!(hold.hold("attack", (9.0, 0.0)));
    }
}

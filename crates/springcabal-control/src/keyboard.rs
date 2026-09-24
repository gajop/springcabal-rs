//! UI side: WASD / arrow keys to a movement message, sent on change.

use spring_addons::KeyEvent;
use springcabal::keys::MoveKeys;
use springcabal::message::SendOnChange;

/// The held movement keys and the direction last sent to the rules.
///
/// Embed it in the game's input widget and feed it every key press and
/// release; call [`send`](Self::send) when a key changed it and from
/// `update`, which retries a send that failed.
///
/// ```ignore
/// fn key_press(&self, _ctx, event: &KeyEvent) -> EventResult {
///     if self.movement.key(event, true) {
///         self.movement.send(send_direction);
///         return EventResult::Handled;
///     }
///     ...
/// }
/// ```
pub struct KeyMovement {
    keys: MoveKeys,
    sent: SendOnChange<(i32, i32)>,
}

impl Default for KeyMovement {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyMovement {
    /// No key held, and the rules assumed to know it.
    pub const fn new() -> Self {
        Self {
            keys: MoveKeys::new(),
            sent: SendOnChange::assuming_sent((0, 0)),
        }
    }

    /// Record a key press (`down`) or release. Returns whether `event` is a
    /// movement key, which the caller should claim even when the direction
    /// didn't change (a key repeat).
    pub fn key(&self, event: &KeyEvent<'_>, down: bool) -> bool {
        self.keys.set(event, down);
        MoveKeys::is_move_key(event)
    }

    /// `(x, z)`, each -1, 0 or 1: x to the right, z towards the camera.
    pub fn direction(&self) -> (i32, i32) {
        self.keys.axis()
    }

    /// Release every key, e.g. when the chat console takes the keyboard. The
    /// next [`send`](Self::send) sends the stop.
    pub fn release_all(&self) {
        self.keys.clear();
    }

    /// Send the direction with `send` if it differs from the last one sent.
    /// `send` reports success; a failed send is retried on the next call.
    pub fn send(&self, send: impl FnOnce((i32, i32)) -> bool) -> bool {
        self.sent
            .send_if_changed(self.direction(), |&direction| send(direction))
    }

    /// Forget what was sent, so the next [`send`](Self::send) sends again
    /// (after the rules were reloaded).
    pub fn resend(&self) {
        self.sent.reset();
    }
}

/// Send `movement|x|z` ([`protocol`](crate::protocol)) to the rules.
/// Returns whether the engine took it.
#[cfg(target_arch = "wasm32")]
pub fn send_direction(direction: (i32, i32)) -> bool {
    let message = crate::protocol::encode_direction(direction);
    match spring::send_lua_rules_msg(&message) {
        Ok(_) => true,
        Err(error) => {
            spring::log_warning!("springcabal", "`{message}` not sent: {error:?}");
            false
        }
    }
}

/// A widget that sends the WASD / arrow direction to the rules as
/// `movement|x|z`, for games with nothing else to do with those keys.
#[cfg(target_arch = "wasm32")]
#[derive(Default)]
pub struct KeyMoveWidget {
    movement: KeyMovement,
}

#[cfg(target_arch = "wasm32")]
impl KeyMoveWidget {
    pub const fn new() -> Self {
        Self {
            movement: KeyMovement::new(),
        }
    }

    fn key(&self, event: &KeyEvent<'_>, down: bool) -> spring_addons::EventResult {
        let claimed = self.movement.key(event, down);
        if claimed {
            self.movement.send(send_direction);
        }
        claimed.into()
    }
}

#[cfg(target_arch = "wasm32")]
impl<G> spring_addons::Widget<G> for KeyMoveWidget {
    fn name(&self) -> &'static str {
        "KeyMove"
    }

    fn update(&self, _ctx: &spring_addons::AddonContext<'_, G>, _dt: f32) {
        self.movement.send(send_direction);
    }

    fn key_press(
        &self,
        _ctx: &spring_addons::AddonContext<'_, G>,
        event: &KeyEvent<'_>,
    ) -> spring_addons::EventResult {
        self.key(event, true)
    }

    fn key_release(
        &self,
        _ctx: &spring_addons::AddonContext<'_, G>,
        event: &KeyEvent<'_>,
    ) -> spring_addons::EventResult {
        self.key(event, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: i32) -> KeyEvent<'static> {
        KeyEvent {
            key_code: code,
            alt: false,
            ctrl: false,
            meta: false,
            shift: false,
            is_repeat: false,
            label: &[],
            utf32_char: 0,
            scan_code: 0,
        }
    }

    #[test]
    fn sends_changes_only() {
        let movement = KeyMovement::new();
        let mut sent = alloc::vec::Vec::new();
        assert!(movement.key(&key(i32::from(b'w')), true));
        assert!(movement.send(|d| {
            sent.push(d);
            true
        }));
        // A repeat is claimed but changes nothing.
        assert!(movement.key(&key(i32::from(b'w')), true));
        assert!(!movement.send(|_| true));
        assert!(!movement.key(&key(i32::from(b'x')), true));
        movement.release_all();
        assert!(movement.send(|d| {
            sent.push(d);
            true
        }));
        assert_eq!(sent, [(0, -1), (0, 0)]);
    }
}

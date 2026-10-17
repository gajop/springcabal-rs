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
    pub fn key<E: spring::capabilities::Local>(
        &self,
        spring: &spring::Spring<'_, E>,
        event: &KeyEvent<'_>,
        down: bool,
    ) -> bool {
        self.keys.set(spring, event, down);
        MoveKeys::is_move_key(event)
    }

    /// `(x, z)`, each -1, 0 or 1: x to the right, z towards the camera.
    pub fn direction<E: spring::capabilities::Local>(
        &self,
        spring: &spring::Spring<'_, E>,
    ) -> (i32, i32) {
        self.keys.axis(spring)
    }

    /// Release every key, e.g. when the chat console takes the keyboard. The
    /// next [`send`](Self::send) sends the stop.
    pub fn release_all(&self) {
        self.keys.clear();
    }

    /// Send the direction with `send` if it differs from the last one sent.
    /// `send` reports success; a failed send is retried on the next call.
    pub fn send<E: spring::capabilities::Local>(
        &self,
        spring: &spring::Spring<'_, E>,
        send: impl FnOnce((i32, i32)) -> bool,
    ) -> bool {
        self.sent
            .send_if_changed(self.direction(spring), |&direction| send(direction))
    }

    /// Forget what was sent, so the next [`send`](Self::send) sends again
    /// (after the rules were reloaded).
    pub fn resend(&self) {
        self.sent.reset();
    }
}

/// Send `movement|x|z` ([`protocol`](crate::protocol)) to the rules.
/// Returns whether the engine took it (a failure is logged).
pub fn send_direction<E: spring::capabilities::Game>(
    spring: &spring::Spring<'_, E>,
    direction: (i32, i32),
) -> bool {
    springcabal::message::send_to_rules(spring, &crate::protocol::encode_direction(direction))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
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

    fn with_spring(call: impl FnOnce(&spring::Spring<'_, spring::Ui>)) {
        use spring::__sys as sys;
        unsafe extern "C" fn writing(
            _: *const sys::IsUserWritingQuery,
            result: *mut sys::IsUserWritingResult,
        ) {
            unsafe {
                *result = sys::IsUserWritingResult {
                    error: core::ptr::null(),
                    writing: false,
                };
            }
        }
        let messages = sys::MessagesApi {
            IsUserWriting: Some(writing),
            ..Default::default()
        };
        let api = sys::NativeInterface {
            messages: &messages,
            ..Default::default()
        };
        let context = unsafe { spring::ModuleContext::<spring::Ui>::from_native(&api) }.unwrap();
        context.with_spring(|spring| call(spring)).unwrap();
    }

    #[test]
    fn sends_changes_only() {
        with_spring(|spring| {
            let movement = KeyMovement::new();
            let mut sent = alloc::vec::Vec::new();
            assert!(movement.key(spring, &key(i32::from(b'w')), true));
            assert!(movement.send(spring, |d| {
                sent.push(d);
                true
            }));
            // A repeat is claimed but changes nothing.
            assert!(movement.key(spring, &key(i32::from(b'w')), true));
            assert!(!movement.send(spring, |_| true));
            assert!(!movement.key(spring, &key(i32::from(b'x')), true));
            movement.release_all();
            assert!(movement.send(spring, |d| {
                sent.push(d);
                true
            }));
            assert_eq!(sent, [(0, -1), (0, 0)]);
        });
    }
}

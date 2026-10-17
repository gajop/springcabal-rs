//! Keyboard helpers shared by the games' input widgets. Key codes and names
//! are the engine's: `spring_addons::keys` and `KeyEvent::matches`.

use core::cell::Cell;

use spring_addons::KeyEvent;

/// Held WASD / arrow keys as a movement direction.
///
/// Feed it every key press and release; it answers with the direction on
/// the map (x to the right, z towards the camera, like screen space) and
/// reports whether it changed, so input is sent only on change.
#[derive(Default)]
pub struct MoveKeys {
    held: Cell<u8>,
}

const A: u8 = 1;
const D: u8 = 2;
const W: u8 = 4;
const S: u8 = 8;
const LEFT: u8 = 16;
const RIGHT: u8 = 32;
const UP: u8 = 64;
const DOWN: u8 = 128;

impl MoveKeys {
    pub const fn new() -> Self {
        Self { held: Cell::new(0) }
    }

    pub fn set<E: spring::capabilities::Local>(
        &self,
        spring: &spring::Spring<'_, E>,
        key: &KeyEvent<'_>,
        down: bool,
    ) -> bool {
        if spring.messages().is_user_writing().unwrap_or(false) {
            self.clear();
            return false;
        }
        self.set_held(key, down)
    }

    pub fn axis<E: spring::capabilities::Local>(
        &self,
        spring: &spring::Spring<'_, E>,
    ) -> (i32, i32) {
        if spring.messages().is_user_writing().unwrap_or(false) {
            self.clear();
            return (0, 0);
        }
        self.raw_axis()
    }

    /// Record a key press (`down`) or release. Returns `true` when the key is
    /// a movement key and the direction changed.
    fn set_held(&self, key: &KeyEvent<'_>, down: bool) -> bool {
        let Some(bit) = key_bit(key) else {
            return false;
        };
        let before = self.raw_axis();
        let held = self.held.get();
        self.held.set(if down { held | bit } else { held & !bit });
        self.raw_axis() != before
    }

    /// Whether `key` is one of the movement keys (held or not), so a caller
    /// can claim it even when [`set`](Self::set) reports no change.
    pub fn is_move_key(key: &KeyEvent<'_>) -> bool {
        key_bit(key).is_some()
    }

    /// `(x, z)`, each -1, 0 or 1. Opposite keys cancel out.
    fn raw_axis(&self) -> (i32, i32) {
        let held = self.held.get();
        let pair = |negative: u8, positive: u8| {
            i32::from(held & positive != 0) - i32::from(held & negative != 0)
        };
        (pair(A | LEFT, D | RIGHT), pair(W | UP, S | DOWN))
    }

    /// Release everything (e.g. when the window loses focus or the game
    /// restarts).
    pub fn clear(&self) {
        self.held.set(0);
    }
}

fn key_bit(key: &KeyEvent<'_>) -> Option<u8> {
    [
        ("a", A),
        ("left", LEFT),
        ("d", D),
        ("right", RIGHT),
        ("w", W),
        ("up", UP),
        ("s", S),
        ("down", DOWN),
    ]
    .into_iter()
    .find(|(symbol, _)| key.matches(symbol))
    .map(|(_, bit)| bit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use spring_addons::keys;

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
    fn opposite_keys_cancel() {
        let held = MoveKeys::new();
        assert!(held.set_held(&key(i32::from(b'a')), true));
        assert_eq!(held.raw_axis(), (-1, 0));
        assert!(held.set_held(&key(keys::RIGHT), true));
        assert_eq!(held.raw_axis(), (0, 0));
        assert!(held.set_held(&key(i32::from(b'a')), false));
        assert_eq!(held.raw_axis(), (1, 0));
        assert!(!held.set_held(&key(i32::from(b'x')), true));
        assert!(held.set_held(&key(i32::from(b'W')), true));
        assert_eq!(held.raw_axis(), (1, -1));
    }

    #[test]
    fn releasing_one_alias_keeps_the_other_held() {
        for (letter, arrow, direction) in [
            (b'a', keys::LEFT, (-1, 0)),
            (b'd', keys::RIGHT, (1, 0)),
            (b'w', keys::UP, (0, -1)),
            (b's', keys::DOWN, (0, 1)),
        ] {
            let held = MoveKeys::new();
            assert!(held.set_held(&key(i32::from(letter)), true));
            assert!(!held.set_held(&key(arrow), true));
            assert!(!held.set_held(&key(i32::from(letter)), false));
            assert_eq!(held.raw_axis(), direction);
            assert!(held.set_held(&key(arrow), false));
            assert_eq!(held.raw_axis(), (0, 0));
        }
    }

    #[test]
    fn arrows_are_the_engine_codes() {
        let held = MoveKeys::new();
        assert!(held.set_held(&key(keys::UP), true));
        assert!(held.set_held(&key(keys::LEFT), true));
        assert_eq!(held.raw_axis(), (-1, -1));
        // The SDL1 codes the engine no longer sends.
        assert!(!MoveKeys::is_move_key(&key(273)));
    }
}

//! Keyboard helpers shared by the games' input widgets.

use core::cell::Cell;

use spring_addons::KeyEvent;

/// SDL key codes the games bind that `KeyEvent::matches` doesn't cover.
pub mod code {
    pub const UP: i32 = 273;
    pub const DOWN: i32 = 274;
    pub const RIGHT: i32 = 275;
    pub const LEFT: i32 = 276;
}

/// Held WASD / arrow keys as a movement direction.
///
/// Feed it every key press and release; it answers with the direction on
/// the map (x to the right, z towards the camera, like screen space) and
/// reports whether it changed, so input is sent only on change.
#[derive(Default)]
pub struct MoveKeys {
    held: Cell<u8>,
}

const LEFT: u8 = 1;
const RIGHT: u8 = 2;
const UP: u8 = 4;
const DOWN: u8 = 8;

impl MoveKeys {
    pub const fn new() -> Self {
        Self { held: Cell::new(0) }
    }

    /// Record a key press (`down`) or release. Returns `true` when the key is
    /// a movement key and the direction changed.
    pub fn set(&self, key: &KeyEvent<'_>, down: bool) -> bool {
        let Some(bit) = direction_bit(key) else {
            return false;
        };
        let before = self.axis();
        let held = self.held.get();
        self.held.set(if down { held | bit } else { held & !bit });
        self.axis() != before
    }

    /// `(x, z)`, each -1, 0 or 1. Opposite keys cancel out.
    pub fn axis(&self) -> (i32, i32) {
        let held = self.held.get();
        let pair = |negative: u8, positive: u8| {
            i32::from(held & positive != 0) - i32::from(held & negative != 0)
        };
        (pair(LEFT, RIGHT), pair(UP, DOWN))
    }

    /// Release everything (e.g. when the window loses focus or the game
    /// restarts).
    pub fn clear(&self) {
        self.held.set(0);
    }
}

fn direction_bit(key: &KeyEvent<'_>) -> Option<u8> {
    match key.key_code {
        code::LEFT => return Some(LEFT),
        code::RIGHT => return Some(RIGHT),
        code::UP => return Some(UP),
        code::DOWN => return Some(DOWN),
        _ => {}
    }
    [("a", LEFT), ("d", RIGHT), ("w", UP), ("s", DOWN)]
        .into_iter()
        .find(|(symbol, _)| key.matches(symbol))
        .map(|(_, bit)| bit)
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
    fn opposite_keys_cancel() {
        let keys = MoveKeys::new();
        assert!(keys.set(&key(i32::from(b'a')), true));
        assert_eq!(keys.axis(), (-1, 0));
        assert!(keys.set(&key(code::RIGHT), true));
        assert_eq!(keys.axis(), (0, 0));
        assert!(keys.set(&key(i32::from(b'a')), false));
        assert_eq!(keys.axis(), (1, 0));
        assert!(!keys.set(&key(i32::from(b'x')), true));
        assert!(keys.set(&key(i32::from(b'W')), true));
        assert_eq!(keys.axis(), (1, -1));
    }
}

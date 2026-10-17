//! Leaving the game. Restarting a round is up to each game.

use spring_addons::KeyEvent;

/// Ctrl+Q: every game's "back to the menu" key. Some platforms report
/// Ctrl+Q as the control character DC1 (17) instead of `q`, which the
/// engine's `KeyEvent::matches` doesn't know.
pub fn is_quit_to_menu(key: &KeyEvent<'_>) -> bool {
    const CTRL_Q: i32 = 17;
    key.ctrl && !key.alt && (key.matches("q") || key.key_code == CTRL_Q)
}

/// Leave the game and go back to the menu (the lobby that started it).
pub fn quit_to_menu<E: spring::capabilities::Game>(spring: &spring::Spring<'_, E>) {
    use spring::ResultExt;
    spring
        .messages()
        .send_commands("reloadforce", "")
        .log_err(spring, "quit to menu");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: i32, ctrl: bool, alt: bool) -> KeyEvent<'static> {
        KeyEvent {
            key_code: code,
            alt,
            ctrl,
            meta: false,
            shift: false,
            is_repeat: false,
            label: &[],
            utf32_char: 0,
            scan_code: 0,
        }
    }

    #[test]
    fn ctrl_q_only() {
        let q = i32::from(b'q');
        assert!(is_quit_to_menu(&key(q, true, false)));
        assert!(is_quit_to_menu(&key(17, true, false)));
        assert!(!is_quit_to_menu(&key(q, false, false)));
        assert!(!is_quit_to_menu(&key(q, true, true)));
        assert!(!is_quit_to_menu(&key(i32::from(b'w'), true, false)));
    }
}

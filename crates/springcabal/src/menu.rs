//! Leaving the game.

use spring_addons::KeyEvent;

/// Ctrl+Q: every game's "back to the menu" key. Some platforms report
/// Ctrl+Q as the control character DC1 (17) instead of `q`.
pub fn is_quit_to_menu(key: &KeyEvent<'_>) -> bool {
    const CTRL_Q: i32 = 17;
    key.ctrl && !key.alt && (key.matches("q") || key.key_code == CTRL_Q)
}

/// Leave the game and go back to the menu (the lobby that started it).
pub fn quit_to_menu() {
    if let Err(error) = spring::send_command("reloadforce") {
        spring::log_warning!("springcabal", "quit to menu failed: {error:?}");
    }
}

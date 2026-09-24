//! Leaving the game.

use spring_addons::KeyEvent;

/// Ctrl+Q: every game's "back to the menu" key.
pub fn is_quit_to_menu(key: &KeyEvent<'_>) -> bool {
    key.ctrl && !key.alt && key.matches("q")
}

/// Leave the game and go back to the menu (the lobby that started it).
pub fn quit_to_menu() {
    if let Err(error) = spring::send_command("reloadforce") {
        spring::log_warning!("springcabal", "quit to menu failed: {error:?}");
    }
}

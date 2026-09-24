//! Restarting a round by reloading the rules.
//!
//! What a restart means stays with each game. Some reload the rules
//! ([`reload_rules`]), some reload the whole game with a start script
//! (`spring::reload`), and some restart the round in place through a rules
//! message, keeping the modules loaded. Only the first is generic enough to
//! live here; the UI side (closing the end screen, restarting the music,
//! forgetting what was sent) is still the game's to do.

/// Reload the synced rules (`luarules reload`), which starts the round over.
///
/// The engine only accepts the reload while cheating is enabled, so this
/// enables it around the reload with [`with_cheats`] and leaves it as it
/// was. The reload is refused before the first game frame.
pub fn reload_rules() {
    with_cheats(|| {
        if let Err(error) = spring::send_commands("luarules reload", "") {
            spring::log_warning!("springcabal", "reloading the rules failed: {error:?}");
        }
    });
}

/// Run `action`, typically a cheat-only command, with cheating enabled, and
/// restore the previous cheat state afterwards.
///
/// `cheat` toggles when given no argument, so an unconditional
/// `cheat`, `...`, `cheat` would switch cheating off for a player who had
/// it on. This sets it explicitly and only when it was off. The commands are
/// synced actions, applied in the order they are sent.
pub fn with_cheats<R>(action: impl FnOnce() -> R) -> R {
    let cheating = spring::game::is_cheating_enabled(0).unwrap_or(false);
    if !cheating {
        send("cheat 1");
    }
    let result = action();
    if !cheating {
        send("cheat 0");
    }
    result
}

fn send(command: &str) {
    if let Err(error) = spring::send_commands(command, "") {
        spring::log_warning!("springcabal", "`{command}` failed: {error:?}");
    }
}

use spring_addons::KeyEvent;

#[derive(Default)]
pub struct GameControls {
    pause_alias: Option<&'static str>,
    keep_rts_bindings: bool,
}

impl GameControls {
    pub const fn new() -> Self {
        Self {
            pause_alias: None,
            keep_rts_bindings: false,
        }
    }

    pub const fn with_pause_key(mut self, key: &'static str) -> Self {
        self.pause_alias = Some(key);
        self
    }

    pub const fn with_rts_controls(mut self) -> Self {
        self.keep_rts_bindings = true;
        self
    }

    fn command(&self, event: &KeyEvent<'_>) -> Option<&'static str> {
        if event.matches("pause") || self.pause_alias.is_some_and(|key| event.matches(key)) {
            Some("pause")
        } else if event.matches("f12") {
            Some("screenshot")
        } else if event.alt && !event.ctrl && !event.meta {
            if ["=", "+", "numpad+", "insert"]
                .iter()
                .any(|key| event.matches(key))
            {
                Some("speedup")
            } else if ["-", "numpad-", "delete"]
                .iter()
                .any(|key| event.matches(key))
            {
                Some("slowdown")
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl<G> spring_addons::ui::Widget<G> for GameControls {
    fn name(&self) -> &'static str {
        "GameControls"
    }

    fn init(&self, _ctx: &spring_addons::AddonContext<'_, G>) {
        use spring::ResultExt;

        if !self.keep_rts_bindings {
            for command in [
                "unbindall",
                "unbind enter chat",
                "bind Any+tab edit_complete",
                "bind Any+backspace edit_backspace",
                "bind Any+delete edit_delete",
                "bind Any+home edit_home",
                "bind Alt+left edit_home",
                "bind Any+end edit_end",
                "bind Alt+right edit_end",
                "bind Any+up edit_prev_line",
                "bind Any+down edit_next_line",
                "bind Any+left edit_prev_char",
                "bind Any+right edit_next_char",
                "bind Ctrl+left edit_prev_word",
                "bind Ctrl+right edit_next_word",
                "bind Any+enter edit_return",
                "bind Any+escape edit_escape",
                "bind Ctrl+v pastetext",
            ] {
                spring::send_command(command).log_err_once("setting chat controls");
            }
        }
        for command in [
            "bind Any+enter chat",
            "bind Any+pause pause",
            "bind Any+f12 screenshot",
            "bind Alt+= speedup",
            "bind Alt++ speedup",
            "bind Alt+- slowdown",
            "bind Alt+numpad+ speedup",
            "bind Alt+numpad- slowdown",
            "bind Alt+insert speedup",
            "bind Alt+delete slowdown",
            "bind Ctrl+q reloadforce",
        ] {
            spring::send_command(command).log_err_once("setting game controls");
        }
    }

    fn shutdown(&self, _ctx: &spring_addons::AddonContext<'_, G>) {
        use spring::ResultExt;

        spring::send_command("keyreload").log_err_once("restoring key bindings");
    }

    fn key_press(
        &self,
        _ctx: &spring_addons::AddonContext<'_, G>,
        event: &KeyEvent<'_>,
    ) -> spring_addons::EventResult {
        use spring::ResultExt;

        let Some(command) = self.command(event) else {
            return spring_addons::EventResult::Ignored;
        };
        if !event.is_repeat || matches!(command, "speedup" | "slowdown") {
            spring::send_command(command).log_err_once("handling a game control key");
        }
        spring_addons::EventResult::Handled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key_code: i32) -> KeyEvent<'static> {
        KeyEvent {
            key_code,
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
    fn utility_keys_leave_gameplay_keys_alone() {
        let controls = GameControls::new();
        assert_eq!(controls.command(&key(0x4000_0048)), Some("pause"));
        assert_eq!(controls.command(&key(0x4000_0045)), Some("screenshot"));
        assert_eq!(controls.command(&key(13)), None);
        assert_eq!(controls.command(&key(0x4000_0058)), None);
        for key_code in [i32::from(b'p'), i32::from(b's'), i32::from(b' ')] {
            assert_eq!(controls.command(&key(key_code)), None);
        }
    }

    #[test]
    fn pause_alias_is_explicit() {
        let controls = GameControls::new().with_pause_key("p");
        assert_eq!(controls.command(&key(i32::from(b'p'))), Some("pause"));
        assert_eq!(controls.command(&key(i32::from(b' '))), None);
    }

    #[test]
    fn alt_speed_keys_use_engine_speed_commands() {
        let controls = GameControls::new();
        for (key_code, command) in [
            (i32::from(b'='), "speedup"),
            (i32::from(b'+'), "speedup"),
            (spring_addons::keys::KEYPAD_PLUS, "speedup"),
            (spring_addons::keys::INSERT, "speedup"),
            (i32::from(b'-'), "slowdown"),
            (spring_addons::keys::KEYPAD_MINUS, "slowdown"),
            (spring_addons::keys::DELETE, "slowdown"),
        ] {
            let mut event = key(key_code);
            assert_eq!(controls.command(&event), None);
            event.alt = true;
            assert_eq!(controls.command(&event), Some(command));
            event.is_repeat = true;
            assert_eq!(controls.command(&event), Some(command));
            event.ctrl = true;
            assert_eq!(controls.command(&event), None);
        }
    }
}

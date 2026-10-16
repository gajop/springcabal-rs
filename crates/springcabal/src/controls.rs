use spring_addons::KeyEvent;

#[derive(Default)]
pub struct GameControls {
    pause_alias: Option<&'static str>,
}

impl GameControls {
    pub const fn new() -> Self {
        Self { pause_alias: None }
    }

    pub const fn with_pause_key(mut self, key: &'static str) -> Self {
        self.pause_alias = Some(key);
        self
    }

    fn command(&self, event: &KeyEvent<'_>) -> Option<&'static str> {
        if event.matches("pause") || self.pause_alias.is_some_and(|key| event.matches(key)) {
            Some("pause")
        } else if event.matches("f12") {
            Some("screenshot")
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

        for command in [
            "unbindall",
            "unbindkeyset enter",
            "bind Any+pause pause",
            "bind Any+f12 screenshot",
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
        if !event.is_repeat {
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
}

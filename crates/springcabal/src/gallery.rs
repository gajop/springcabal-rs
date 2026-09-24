//! Gallery mode, for the verification galleries
//! (`verification/fixtures/gallery/gallery.lua` in springcabal-games).
//!
//! They set the `gallery` modoption, place the camera themselves and capture
//! single units and effects, so a game's UI module registers only the
//! widgets that draw what a player sees in the world: no HUD, RmlUi
//! document, input, camera or sound. The galleries fail a module that
//! doesn't log `gallery mode:`.

use alloc::boxed::Box;
use alloc::vec::Vec;

use spring_addons::ui::{Widget, WidgetHandler};

/// Whether the `gallery` modoption is set.
#[cfg(target_arch = "wasm32")]
pub fn active() -> bool {
    spring::game_options::get_mod_option("gallery")
        .ok()
        .flatten()
        .is_some_and(|mode| !mode.is_empty())
}

/// Registers the gallery's widgets, then logs which:
///
/// ```ignore
/// Gallery::new(handler)
///     .widget(HideDefaultInterface::new(HIDDEN_INTERFACE))
///     .widget(GfxFireWidget::new())
///     .announce("ttld-ui"); // gallery mode: HideDefaultInterface GfxFire
/// ```
pub struct Gallery<'a, G> {
    handler: &'a mut WidgetHandler<G>,
    names: Vec<&'static str>,
}

impl<'a, G> Gallery<'a, G> {
    pub fn new(handler: &'a mut WidgetHandler<G>) -> Self {
        Self {
            handler,
            names: Vec::new(),
        }
    }

    pub fn widget(mut self, widget: impl Widget<G> + 'static) -> Self {
        self.names.push(widget.name());
        self.handler.add(Box::new(widget));
        self
    }

    /// The names of the widgets added, in order.
    pub fn names(&self) -> &[&'static str] {
        &self.names
    }

    /// Log `gallery mode: <widget names>` under `section`.
    #[cfg(target_arch = "wasm32")]
    pub fn announce(self, section: &str) {
        spring::log_notice!(section, "gallery mode: {}", self.names.join(" "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Named(&'static str);

    impl Widget<()> for Named {
        fn name(&self) -> &'static str {
            self.0
        }
    }

    #[test]
    fn names_in_order() {
        let mut handler = WidgetHandler::new(());
        let gallery = Gallery::new(&mut handler)
            .widget(Named("A"))
            .widget(Named("B"));
        assert_eq!(gallery.names(), ["A", "B"]);
    }
}

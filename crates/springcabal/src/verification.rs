use alloc::boxed::Box;
use alloc::vec::Vec;

use spring_addons::ui::{Widget, WidgetHandler};

#[cfg(target_arch = "wasm32")]
pub fn world_only() -> bool {
    spring::game_options::get_mod_option("gallery")
        .ok()
        .flatten()
        .is_some_and(|mode| !mode.is_empty())
}

pub struct WorldWidgets<'a, G> {
    handler: &'a mut WidgetHandler<G>,
    names: Vec<&'static str>,
}

impl<'a, G> WorldWidgets<'a, G> {
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

    pub fn names(&self) -> &[&'static str] {
        &self.names
    }

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
        let gallery = WorldWidgets::new(&mut handler)
            .widget(Named("A"))
            .widget(Named("B"));
        assert_eq!(gallery.names(), ["A", "B"]);
    }
}

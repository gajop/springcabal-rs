use alloc::boxed::Box;
use alloc::vec::Vec;

use spring_addons::ui::{Widget, WidgetHandler};

pub fn world_only<E: spring::capabilities::Game>(spring: &spring::Spring<'_, E>) -> bool {
    spring
        .game()
        .get_mod_option("gallery")
        .is_ok_and(|mode| mode.exists && !mode.value.is_empty())
}

pub struct WorldWidgets<'a, G, E: spring::capabilities::Local = spring::Ui> {
    handler: &'a mut WidgetHandler<G, E>,
    names: Vec<&'static str>,
}

impl<'a, G, E: spring::capabilities::Local> WorldWidgets<'a, G, E> {
    pub fn new(handler: &'a mut WidgetHandler<G, E>) -> Self {
        Self {
            handler,
            names: Vec::new(),
        }
    }

    pub fn widget(mut self, widget: impl Widget<G, E> + 'static) -> Self {
        self.names.push(widget.name());
        self.handler.add(Box::new(widget));
        self
    }

    pub fn names(&self) -> &[&'static str] {
        &self.names
    }

    pub fn announce(self, spring: &spring::Spring<'_, E>, section: &str) {
        spring::log_notice!(spring, section, "gallery mode: {}", self.names.join(" "));
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

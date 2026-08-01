use crate::prelude::*;

pub struct KeyBindingStory;

impl Render for KeyBindingStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "Key combinations",
            vec![example(
                "Various bindings",
                h_flex()
                    .gap(Spacing::Large.pixels())
                    .items_center()
                    .child(KeyBinding::new(["Cmd", "S"]))
                    .child(KeyBinding::new(["Ctrl", "Shift", "P"]))
                    .child(KeyBinding::new(["Esc"]))
                    .into_any_element(),
            )],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| KeyBindingStory).into()
}

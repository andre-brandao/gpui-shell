use crate::layout::{example, example_group};
use crate::prelude::*;

pub struct KeybindingHintStory;

impl Render for KeybindingHintStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "KeybindingHint variants",
            vec![
                example(
                    "Standalone",
                    h_flex()
                        .child(KeybindingHint::new(KeyBinding::new(["Ctrl", "S"])))
                        .into_any_element(),
                ),
                example(
                    "With prefix",
                    h_flex()
                        .child(KeybindingHint::with_prefix(
                            "Press",
                            KeyBinding::new(["Enter"]),
                        ))
                        .into_any_element(),
                ),
                example(
                    "With suffix",
                    h_flex()
                        .child(KeybindingHint::with_suffix(
                            KeyBinding::new(["Ctrl", "Shift", "P"]),
                            "to open command palette",
                        ))
                        .into_any_element(),
                ),
                example(
                    "Prefix + suffix",
                    h_flex()
                        .child(
                            KeybindingHint::with_prefix("Hit", KeyBinding::new(["Esc"]))
                                .suffix("to dismiss"),
                        )
                        .into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| KeybindingHintStory).into()
}

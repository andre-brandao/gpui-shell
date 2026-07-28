use crate::prelude::*;

pub struct CheckboxStory {
    small: ToggleState,
    default: ToggleState,
    large: ToggleState,
    tri: ToggleState,
}

impl CheckboxStory {
    fn new() -> Self {
        Self {
            small: ToggleState::Selected,
            default: ToggleState::Unselected,
            large: ToggleState::Selected,
            tri: ToggleState::Indeterminate,
        }
    }
}

fn toggle_setter<F>(
    weak: &WeakEntity<CheckboxStory>,
    set: F,
) -> impl Fn(&ToggleState, &mut Window, &mut App) + 'static
where
    F: Fn(&mut CheckboxStory, ToggleState) + 'static,
{
    let weak = weak.clone();
    move |state, _window, cx| {
        let state = *state;
        weak.update(cx, |this, cx| {
            set(this, state);
            cx.notify();
        })
        .ok();
    }
}

impl Render for CheckboxStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();

        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Sizes (interactive)",
                vec![
                    example(
                        "Small",
                        Checkbox::new("cb-sm", self.small)
                            .size(CheckboxSize::Small)
                            .label("Small")
                            .on_click(toggle_setter(&weak, |this, s| this.small = s))
                            .into_any_element(),
                    ),
                    example(
                        "Default",
                        Checkbox::new("cb-def", self.default)
                            .label("Default")
                            .on_click(toggle_setter(&weak, |this, s| this.default = s))
                            .into_any_element(),
                    ),
                    example(
                        "Large",
                        Checkbox::new("cb-lg", self.large)
                            .size(CheckboxSize::Large)
                            .label("Large")
                            .on_click(toggle_setter(&weak, |this, s| this.large = s))
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "States",
                vec![
                    example(
                        "Tri-state (indeterminate)",
                        Checkbox::new("cb-tri", self.tri)
                            .label("Tri-state")
                            .on_click(toggle_setter(&weak, |this, s| this.tri = s))
                            .into_any_element(),
                    ),
                    example(
                        "Disabled",
                        Checkbox::new("cb-dis", true)
                            .label("Disabled")
                            .disabled(true)
                            .into_any_element(),
                    ),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| CheckboxStory::new()).into()
}

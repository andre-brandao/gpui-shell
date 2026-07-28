use crate::layout::{example, example_group};
use crate::prelude::*;

pub struct RadioStory {
    selected: usize,
}

impl RadioStory {
    fn new() -> Self {
        Self { selected: 0 }
    }
}

impl Render for RadioStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();
        let options = ["Alpha", "Beta", "Gamma"];

        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Radio group (interactive)",
                options
                    .iter()
                    .enumerate()
                    .map(|(i, &name)| {
                        let state = if self.selected == i {
                            ToggleState::Selected
                        } else {
                            ToggleState::Unselected
                        };
                        let w = weak.clone();
                        example(
                            name,
                            Radio::new(SharedString::from(format!("r-{i}")), state)
                                .label(name)
                                .on_click(move |_, _, cx| {
                                    w.update(cx, |this, cx| {
                                        this.selected = i;
                                        cx.notify();
                                    })
                                    .ok();
                                })
                                .into_any_element(),
                        )
                    })
                    .collect(),
            ))
            .child(example_group(
                "States",
                vec![
                    example(
                        "Disabled unselected",
                        Radio::new("r-dis-off", false)
                            .label("Can't pick")
                            .disabled(true)
                            .into_any_element(),
                    ),
                    example(
                        "Disabled selected",
                        Radio::new("r-dis-on", true)
                            .label("Locked in")
                            .disabled(true)
                            .into_any_element(),
                    ),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| RadioStory::new()).into()
}

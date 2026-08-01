use crate::prelude::*;

pub struct VariableListStory {
    scroll_handle: VariableListScrollHandle,
}

const ROW_COUNT: usize = 2_000;

impl VariableListStory {
    fn new() -> Self {
        Self {
            scroll_handle: VariableListScrollHandle::new(ROW_COUNT),
        }
    }
}

impl Render for VariableListStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let handle = self.scroll_handle.clone();
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "Large list with variable row heights (2,000 rows)",
            vec![example(
                "Every third row is taller - gpui::list measures rows lazily",
                v_flex()
                    .h(px(320.0))
                    .w_full()
                    .child(
                        VariableList::new(handle, |ix, _window, _cx| {
                            let tall = ix % 3 == 0;
                            let py = if tall {
                                Spacing::Large.pixels()
                            } else {
                                Spacing::Small.pixels()
                            };
                            h_flex()
                                .px(Spacing::Medium.pixels())
                                .py(py)
                                .gap(Spacing::Small.pixels())
                                .child(Label::new(format!(
                                    "row {ix:>5} - {}",
                                    if tall { "tall" } else { "short" }
                                )))
                                .into_any_element()
                        })
                        .scrollbar()
                        .h_full(),
                    )
                    .into_any_element(),
            )],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| VariableListStory::new()).into()
}

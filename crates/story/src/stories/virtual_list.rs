use crate::prelude::*;

pub struct VirtualListStory {
    scroll_handle: VirtualListScrollHandle,
}

impl VirtualListStory {
    fn new() -> Self {
        Self {
            scroll_handle: VirtualListScrollHandle::new(),
        }
    }
}

const ROW_COUNT: usize = 5_000;

impl Render for VirtualListStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let handle = self.scroll_handle.clone();
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "Large list (5,000 rows)",
            vec![example(
                "Only the visible slice is built each frame",
                v_flex()
                    .h(px(320.0))
                    .w_full()
                    .child(
                        VirtualList::new("vlist", ROW_COUNT, move |range, _w, _cx| {
                            range
                                .map(|ix| {
                                    h_flex()
                                        .px(Spacing::Medium.pixels())
                                        .py(Spacing::Small.pixels())
                                        .gap(Spacing::Small.pixels())
                                        .child(Label::new(format!("row {ix:>5}")))
                                        .into_any_element()
                                })
                                .collect()
                        })
                        .track_scroll(handle.clone())
                        .scrollbar()
                        .h_full(),
                    )
                    .into_any_element(),
            )],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| VirtualListStory::new()).into()
}

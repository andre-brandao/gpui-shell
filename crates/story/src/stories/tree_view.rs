use crate::prelude::*;

pub struct TreeViewStory {
    root_expanded: bool,
    selected_index: Option<usize>,
}

impl TreeViewStory {
    fn new() -> Self {
        Self {
            root_expanded: true,
            selected_index: None,
        }
    }
}

impl Render for TreeViewStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();
        let selected = self.selected_index;

        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "Tree view",
            vec![example(
                "File tree",
                v_flex()
                    .w(px(260.0))
                    .child(
                        TreeViewItem::new("root", "src")
                            .root_item(true)
                            .expanded(self.root_expanded)
                            .toggle_state(selected == Some(0))
                            .on_toggle({
                                let weak = weak.clone();
                                move |_, _, cx| {
                                    weak.update(cx, |this, cx| {
                                        this.root_expanded = !this.root_expanded;
                                        cx.notify();
                                    })
                                    .ok();
                                }
                            })
                            .on_click({
                                let weak = weak.clone();
                                move |_, _, cx| {
                                    weak.update(cx, |this, cx| {
                                        this.selected_index = Some(0);
                                        cx.notify();
                                    })
                                    .ok();
                                }
                            }),
                    )
                    .when(self.root_expanded, |this| {
                        this.child(
                            TreeViewItem::new("leaf-1", "main.rs")
                                .toggle_state(selected == Some(1))
                                .on_click({
                                    let weak = weak.clone();
                                    move |_, _, cx| {
                                        weak.update(cx, |this, cx| {
                                            this.selected_index = Some(1);
                                            cx.notify();
                                        })
                                        .ok();
                                    }
                                }),
                        )
                        .child(
                            TreeViewItem::new("leaf-2", "lib.rs")
                                .toggle_state(selected == Some(2))
                                .on_click({
                                    let weak = weak.clone();
                                    move |_, _, cx| {
                                        weak.update(cx, |this, cx| {
                                            this.selected_index = Some(2);
                                            cx.notify();
                                        })
                                        .ok();
                                    }
                                }),
                        )
                        .child(
                            TreeViewItem::new("leaf-3", "utils.rs")
                                .toggle_state(selected == Some(3))
                                .disabled(true),
                        )
                    })
                    .into_any_element(),
            )],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| TreeViewStory::new()).into()
}

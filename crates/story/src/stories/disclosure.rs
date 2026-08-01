use crate::prelude::*;

pub struct DisclosureStory {
    open: bool,
}

impl DisclosureStory {
    fn new() -> Self {
        Self { open: true }
    }
}

impl Render for DisclosureStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();

        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "Interactive disclosure",
            vec![
                example(
                    "Click to expand/collapse",
                    v_flex()
                        .gap(Spacing::Small.pixels())
                        .child(
                            h_flex()
                                .gap(Spacing::XSmall.pixels())
                                .items_center()
                                .child(Disclosure::new("disc-1", self.open).on_click({
                                    let weak = weak.clone();
                                    move |_event, _window, cx| {
                                        weak.update(cx, |this, cx| {
                                            this.open = !this.open;
                                            cx.notify();
                                        })
                                        .ok();
                                    }
                                }))
                                .child(Label::new("Advanced settings")),
                        )
                        .when(self.open, |this| {
                            this.child(
                                v_flex()
                                    .pl(px(24.0))
                                    .gap(Spacing::XSmall.pixels())
                                    .child(Label::new("Setting one").color(Color::Muted))
                                    .child(Label::new("Setting two").color(Color::Muted))
                                    .child(Label::new("Setting three").color(Color::Muted)),
                            )
                        })
                        .into_any_element(),
                ),
                example(
                    "Disabled",
                    h_flex()
                        .gap(Spacing::XSmall.pixels())
                        .items_center()
                        .child(Disclosure::new("disc-disabled", false).disabled(true))
                        .child(Label::new("Disabled section").color(Color::Disabled))
                        .into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| DisclosureStory::new()).into()
}

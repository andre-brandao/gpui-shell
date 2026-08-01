use crate::prelude::*;

pub struct CopyButtonStory;

impl Render for CopyButtonStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "CopyButton",
            vec![
                example(
                    "Default",
                    h_flex()
                        .gap(Spacing::Small.pixels())
                        .child(Label::new("some-api-key-1234").size(TextSize::Small))
                        .child(CopyButton::new("copy-default", "some-api-key-1234"))
                        .into_any_element(),
                ),
                example(
                    "Custom tooltip",
                    h_flex()
                        .gap(Spacing::Small.pixels())
                        .child(Label::new("secret value").size(TextSize::Small))
                        .child(
                            CopyButton::new("copy-tooltip", "secret value")
                                .tooltip_label("Copy secret"),
                        )
                        .into_any_element(),
                ),
                example(
                    "Sizes",
                    h_flex()
                        .gap(Spacing::Large.pixels())
                        .items_center()
                        .child(
                            h_flex()
                                .gap(Spacing::XSmall.pixels())
                                .child(Label::new("XSmall").size(TextSize::XSmall))
                                .child(
                                    CopyButton::new("copy-xs", "text").icon_size(IconSize::XSmall),
                                ),
                        )
                        .child(
                            h_flex()
                                .gap(Spacing::XSmall.pixels())
                                .child(Label::new("Small").size(TextSize::Small))
                                .child(CopyButton::new("copy-sm", "text")),
                        )
                        .child(
                            h_flex()
                                .gap(Spacing::XSmall.pixels())
                                .child(Label::new("Medium"))
                                .child(
                                    CopyButton::new("copy-md", "text").icon_size(IconSize::Medium),
                                ),
                        )
                        .into_any_element(),
                ),
                example(
                    "Disabled",
                    h_flex()
                        .gap(Spacing::Small.pixels())
                        .child(Label::new("disabled").size(TextSize::Small))
                        .child(CopyButton::new("copy-disabled", "disabled").disabled(true))
                        .into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| CopyButtonStory).into()
}

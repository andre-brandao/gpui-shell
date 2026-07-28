use crate::prelude::*;

pub struct DecoratedIconStory;

impl Render for DecoratedIconStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();

        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "DecoratedIcon",
            vec![
                example(
                    "With status dot",
                    h_flex()
                        .gap(Spacing::Large.pixels())
                        .items_center()
                        .child(DecoratedIcon::new(
                            Icon::new(IconName::File),
                            IconSize::Medium,
                            Some(IconDecoration::dot(colors.status.info)),
                        ))
                        .child(DecoratedIcon::new(
                            Icon::new(IconName::File),
                            IconSize::Medium,
                            Some(IconDecoration::dot(colors.status.warning)),
                        ))
                        .child(DecoratedIcon::new(
                            Icon::new(IconName::File),
                            IconSize::Medium,
                            Some(IconDecoration::dot(colors.status.error)),
                        ))
                        .child(DecoratedIcon::new(
                            Icon::new(IconName::File),
                            IconSize::Medium,
                            Some(IconDecoration::dot(colors.status.success)),
                        ))
                        .into_any_element(),
                ),
                example(
                    "Without decoration",
                    DecoratedIcon::new(Icon::new(IconName::Folder), IconSize::Medium, None)
                        .into_any_element(),
                ),
                example(
                    "Larger icons",
                    h_flex()
                        .gap(Spacing::Large.pixels())
                        .items_center()
                        .child(DecoratedIcon::new(
                            Icon::new(IconName::Folder),
                            IconSize::XLarge,
                            Some(IconDecoration::dot(colors.status.info).size(px(10.0))),
                        ))
                        .into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| DecoratedIconStory).into()
}

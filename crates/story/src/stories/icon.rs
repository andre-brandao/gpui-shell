use crate::prelude::*;

pub struct IconStory;

impl Render for IconStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Common icons",
                vec![example(
                    "Selection",
                    h_flex()
                        .gap(Spacing::Large.pixels())
                        .child(Icon::new(IconName::Check))
                        .child(Icon::new(IconName::Close))
                        .child(Icon::new(IconName::Plus))
                        .child(Icon::new(IconName::Dash))
                        .child(Icon::new(IconName::ChevronDown))
                        .child(Icon::new(IconName::ChevronRight))
                        .child(Icon::new(IconName::MagnifyingGlass))
                        .child(Icon::new(IconName::Settings))
                        .into_any_element(),
                )],
            ))
            .child(example_group(
                "Sizes",
                vec![
                    example(
                        "XSmall",
                        Icon::new(IconName::Settings)
                            .size(IconSize::XSmall)
                            .into_any_element(),
                    ),
                    example(
                        "Small",
                        Icon::new(IconName::Settings)
                            .size(IconSize::Small)
                            .into_any_element(),
                    ),
                    example(
                        "Medium (default)",
                        Icon::new(IconName::Settings)
                            .size(IconSize::Medium)
                            .into_any_element(),
                    ),
                    example(
                        "Large",
                        Icon::new(IconName::Settings)
                            .size(IconSize::XLarge)
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Colors",
                vec![example(
                    "Semantic colors",
                    h_flex()
                        .gap(Spacing::Large.pixels())
                        .child(Icon::new(IconName::Info).color(Color::Info))
                        .child(Icon::new(IconName::Check).color(Color::Success))
                        .child(Icon::new(IconName::Warning).color(Color::Warning))
                        .child(Icon::new(IconName::XCircle).color(Color::Error))
                        .child(Icon::new(IconName::Settings).color(Color::Muted))
                        .into_any_element(),
                )],
            ))
            .child(example_group(
                "External SVGs",
                vec![example(
                    "Icon::from_path (story AssetSource + shell fallback)",
                    h_flex()
                        .gap(Spacing::Large.pixels())
                        .child(Icon::from_path("demo/story_mark.svg").color(Color::Accent))
                        .child(Icon::from_path("icons/star.svg").color(Color::Warning))
                        .child(Icon::from_path("icons/heart.svg").color(Color::Error))
                        .into_any_element(),
                )],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| IconStory).into()
}

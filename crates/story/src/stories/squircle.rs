use crate::prelude::*;

pub struct SquircleStory;

impl Render for SquircleStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let fill = cx.theme().colors().element_background;

        let rounded = gpui::div()
            .w(px(160.0))
            .h(px(160.0))
            .rounded(px(56.0))
            .bg(fill);

        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Shape comparison (r=56, 160px) \u{2014} watch corner shoulders",
                vec![
                    example("Rounded rect", rounded.into_any_element()),
                    example(
                        "Squircle smoothing=0",
                        Squircle::new()
                            .size(px(160.0))
                            .fill(SquircleFill::Muted)
                            .corner_radius(px(56.0))
                            .corner_smoothing(0.0)
                            .into_any_element(),
                    ),
                    example(
                        "Squircle smoothing=0.6",
                        Squircle::new()
                            .size(px(160.0))
                            .fill(SquircleFill::Muted)
                            .corner_radius(px(56.0))
                            .corner_smoothing(0.6)
                            .into_any_element(),
                    ),
                    example(
                        "Squircle smoothing=1.0 (default)",
                        Squircle::new()
                            .size(px(160.0))
                            .fill(SquircleFill::Muted)
                            .corner_radius(px(56.0))
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Fills",
                vec![
                    example(
                        "Elevated (default)",
                        Squircle::new()
                            .size(px(64.0))
                            .bordered(true)
                            .into_any_element(),
                    ),
                    example(
                        "Surface + bordered",
                        Squircle::new()
                            .size(px(64.0))
                            .fill(SquircleFill::Surface)
                            .bordered(true)
                            .into_any_element(),
                    ),
                    example(
                        "Muted",
                        Squircle::new()
                            .size(px(64.0))
                            .fill(SquircleFill::Muted)
                            .into_any_element(),
                    ),
                    example(
                        "Accent",
                        Squircle::new()
                            .size(px(64.0))
                            .fill(SquircleFill::Accent)
                            .into_any_element(),
                    ),
                    example(
                        "Transparent + border",
                        Squircle::new()
                            .size(px(64.0))
                            .fill(SquircleFill::Transparent)
                            .bordered(true)
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Sizes",
                vec![
                    example(
                        "32",
                        Squircle::new()
                            .size(px(32.0))
                            .bordered(true)
                            .into_any_element(),
                    ),
                    example(
                        "48",
                        Squircle::new()
                            .size(px(48.0))
                            .bordered(true)
                            .into_any_element(),
                    ),
                    example(
                        "64",
                        Squircle::new()
                            .size(px(64.0))
                            .bordered(true)
                            .into_any_element(),
                    ),
                    example(
                        "96",
                        Squircle::new()
                            .size(px(96.0))
                            .bordered(true)
                            .into_any_element(),
                    ),
                    example(
                        "96 x 64",
                        Squircle::new()
                            .width(px(96.0))
                            .height(px(64.0))
                            .bordered(true)
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Framed content",
                vec![
                    example(
                        "Icon",
                        Squircle::new()
                            .size(px(64.0))
                            .fill(SquircleFill::Muted)
                            .child(Icon::new(IconName::Folder))
                            .into_any_element(),
                    ),
                    example(
                        "Label",
                        Squircle::new()
                            .size(px(64.0))
                            .fill(SquircleFill::Muted)
                            .child(Label::new("Aa"))
                            .into_any_element(),
                    ),
                    example(
                        "Status icon",
                        Squircle::new()
                            .size(px(64.0))
                            .fill(SquircleFill::Muted)
                            .child(Icon::new(IconName::Check).color(Color::Success))
                            .into_any_element(),
                    ),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| SquircleStory).into()
}

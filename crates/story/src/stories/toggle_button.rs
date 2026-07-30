use crate::prelude::*;

pub struct ToggleButtonStory;

impl Render for ToggleButtonStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Transparent (default)",
                vec![
                    example(
                        "Simple",
                        ToggleButtonGroup::new(
                            "transparent_simple",
                            [
                                ToggleButtonSimple::new("First", |_, _, _| {}),
                                ToggleButtonSimple::new("Second", |_, _, _| {}),
                                ToggleButtonSimple::new("Third", |_, _, _| {}),
                            ],
                        )
                        .selected_index(1)
                        .into_any_element(),
                    ),
                    example(
                        "With icons",
                        ToggleButtonGroup::new(
                            "transparent_icons",
                            [
                                ToggleButtonWithIcon::new("List", IconName::Menu, |_, _, _| {}),
                                ToggleButtonWithIcon::new("Grid", IconName::Layout, |_, _, _| {}),
                                ToggleButtonWithIcon::new("Board", IconName::Table, |_, _, _| {}),
                            ],
                        )
                        .selected_index(0)
                        .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Outlined",
                vec![
                    example(
                        "Simple",
                        ToggleButtonGroup::new(
                            "outlined_simple",
                            [
                                ToggleButtonSimple::new("Day", |_, _, _| {}),
                                ToggleButtonSimple::new("Week", |_, _, _| {}),
                                ToggleButtonSimple::new("Month", |_, _, _| {}),
                            ],
                        )
                        .style(ToggleButtonGroupStyle::Outlined)
                        .selected_index(0)
                        .into_any_element(),
                    ),
                    example(
                        "Two buttons",
                        ToggleButtonGroup::new(
                            "outlined_two",
                            [
                                ToggleButtonSimple::new("On", |_, _, _| {}),
                                ToggleButtonSimple::new("Off", |_, _, _| {}),
                            ],
                        )
                        .style(ToggleButtonGroupStyle::Outlined)
                        .selected_index(0)
                        .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Filled",
                vec![
                    example(
                        "Simple",
                        ToggleButtonGroup::new(
                            "filled_simple",
                            [
                                ToggleButtonSimple::new("All", |_, _, _| {}),
                                ToggleButtonSimple::new("Active", |_, _, _| {}),
                                ToggleButtonSimple::new("Archived", |_, _, _| {}),
                            ],
                        )
                        .style(ToggleButtonGroupStyle::Filled)
                        .selected_index(2)
                        .into_any_element(),
                    ),
                    example(
                        "With icons",
                        ToggleButtonGroup::new(
                            "filled_icons",
                            [
                                ToggleButtonWithIcon::new("Code", IconName::Code, |_, _, _| {}),
                                ToggleButtonWithIcon::new("Preview", IconName::Eye, |_, _, _| {}),
                            ],
                        )
                        .style(ToggleButtonGroupStyle::Filled)
                        .selected_index(0)
                        .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Auto width",
                vec![example(
                    "Shrinks to fit",
                    ToggleButtonGroup::new(
                        "auto_width",
                        [
                            ToggleButtonSimple::new("A", |_, _, _| {}),
                            ToggleButtonSimple::new("B", |_, _, _| {}),
                            ToggleButtonSimple::new("C", |_, _, _| {}),
                        ],
                    )
                    .auto_width()
                    .style(ToggleButtonGroupStyle::Outlined)
                    .selected_index(1)
                    .into_any_element(),
                )],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| ToggleButtonStory).into()
}

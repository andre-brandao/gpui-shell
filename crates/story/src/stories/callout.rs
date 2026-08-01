use crate::prelude::*;

pub struct CalloutStory;

impl Render for CalloutStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Severities",
                vec![
                    example(
                        "Info",
                        Callout::new()
                            .severity(Severity::Info)
                            .title("Heads up")
                            .description("This is an informational message.")
                            .into_any_element(),
                    ),
                    example(
                        "Success",
                        Callout::new()
                            .severity(Severity::Success)
                            .title("All good")
                            .description("Operation completed successfully.")
                            .into_any_element(),
                    ),
                    example(
                        "Warning",
                        Callout::new()
                            .severity(Severity::Warning)
                            .title("Caution")
                            .description("Please review your settings before continuing.")
                            .into_any_element(),
                    ),
                    example(
                        "Error",
                        Callout::new()
                            .severity(Severity::Error)
                            .title("Something went wrong")
                            .description("Failed to connect to the server. Check your network.")
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Border positions",
                vec![
                    example(
                        "Top (default)",
                        Callout::new()
                            .severity(Severity::Info)
                            .title("Top border")
                            .border_position(BorderPosition::Top)
                            .into_any_element(),
                    ),
                    example(
                        "Bottom",
                        Callout::new()
                            .severity(Severity::Warning)
                            .title("Bottom border")
                            .border_position(BorderPosition::Bottom)
                            .into_any_element(),
                    ),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| CalloutStory).into()
}

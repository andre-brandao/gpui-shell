use crate::prelude::*;

pub struct SwitchStory {
    notifications: ToggleState,
    autosave: ToggleState,
    telemetry: ToggleState,
}

impl SwitchStory {
    fn new() -> Self {
        Self {
            notifications: ToggleState::Selected,
            autosave: ToggleState::Unselected,
            telemetry: ToggleState::Selected,
        }
    }
}

fn toggle_setter<F>(
    weak: &WeakEntity<SwitchStory>,
    set: F,
) -> impl Fn(&ToggleState, &mut Window, &mut App) + 'static
where
    F: Fn(&mut SwitchStory, ToggleState) + 'static,
{
    let weak = weak.clone();
    move |state, _window, cx| {
        let state = *state;
        weak.update(cx, |this, cx| {
            set(this, state);
            cx.notify();
        })
        .ok();
    }
}

impl Render for SwitchStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();

        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Interactive switches",
                vec![
                    example(
                        "Notifications",
                        Switch::new("sw-notif", self.notifications)
                            .label("Notifications")
                            .on_click(toggle_setter(&weak, |this, s| this.notifications = s))
                            .into_any_element(),
                    ),
                    example(
                        "Auto-save",
                        Switch::new("sw-auto", self.autosave)
                            .label("Auto-save")
                            .on_click(toggle_setter(&weak, |this, s| this.autosave = s))
                            .into_any_element(),
                    ),
                    example(
                        "Telemetry",
                        Switch::new("sw-tel", self.telemetry)
                            .label("Telemetry")
                            .on_click(toggle_setter(&weak, |this, s| this.telemetry = s))
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "States",
                vec![example(
                    "Disabled",
                    Switch::new("sw-dis", false)
                        .label("Disabled")
                        .disabled(true)
                        .into_any_element(),
                )],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| SwitchStory::new()).into()
}

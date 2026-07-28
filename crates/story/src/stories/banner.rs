use crate::prelude::*;

pub struct BannerStory {
    info_visible: bool,
}

impl BannerStory {
    fn new() -> Self {
        Self { info_visible: true }
    }
}

impl Render for BannerStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();

        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Banner severities",
                vec![
                    example(
                        "Info (dismissible)",
                        v_flex()
                            .gap(Spacing::Small.pixels())
                            .when(self.info_visible, |this| {
                                this.child(
                                    Banner::new(Severity::Info, "New version available")
                                        .description("Engram 0.2 is ready to install.")
                                        .action(
                                            Button::new("banner-update", "Update")
                                                .style(ButtonStyle::Tinted(TintColor::Accent))
                                                .size(ButtonSize::Compact),
                                        )
                                        .on_dismiss({
                                            let weak = weak.clone();
                                            move |_, _, cx| {
                                                weak.update(cx, |this, cx| {
                                                    this.info_visible = false;
                                                    cx.notify();
                                                })
                                                .ok();
                                            }
                                        }),
                                )
                            })
                            .when(!self.info_visible, |this| {
                                this.child(
                                    Button::new("banner-restore", "Restore banner")
                                        .style(ButtonStyle::Subtle)
                                        .size(ButtonSize::Compact)
                                        .on_click({
                                            let weak = weak.clone();
                                            move |_, _, cx| {
                                                weak.update(cx, |this, cx| {
                                                    this.info_visible = true;
                                                    cx.notify();
                                                })
                                                .ok();
                                            }
                                        }),
                                )
                            })
                            .into_any_element(),
                    ),
                    example(
                        "Success",
                        Banner::new(Severity::Success, "All checks passed").into_any_element(),
                    ),
                    example(
                        "Warning",
                        Banner::new(Severity::Warning, "Disk usage at 90%")
                            .description("Consider freeing up some space.")
                            .into_any_element(),
                    ),
                    example(
                        "Error",
                        Banner::new(Severity::Error, "Build failed")
                            .description("3 tests failed in `engram-ui`.")
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Notification (toast style)",
                vec![
                    example(
                        "Success",
                        Notification::new(Severity::Success, "Saved")
                            .description("Your changes were saved automatically.")
                            .into_any_element(),
                    ),
                    example(
                        "Error",
                        Notification::new(Severity::Error, "Sync failed")
                            .description("Check your network connection and retry.")
                            .into_any_element(),
                    ),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| BannerStory::new()).into()
}

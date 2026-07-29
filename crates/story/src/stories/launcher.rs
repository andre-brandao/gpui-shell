//! Launcher - the shell's command surface.
//!
//! Bespoke, not a primitive: it lives in `ui::patterns`, so it is imported
//! by name rather than coming in through `use ui::*`. The frame is all the
//! library owns; the rows below are stand-ins for whatever the app feeds it.

use gpui::{AnyElement, div};
use ui::patterns::{LauncherFrame, footer_hints};

use crate::prelude::*;

const MATCHES: &[(IconName, &str, &str)] = &[
    (IconName::Globe, "Firefox", "Web browser"),
    (IconName::Terminal, "Foot", "Wayland terminal emulator"),
    (IconName::Image, "Firewatch", "Wallpaper collection"),
    (IconName::Settings, "Firewall", "Configure nftables rules"),
];

pub struct LauncherStory {
    input: InputBuffer,
    selected: usize,
}

impl LauncherStory {
    fn new() -> Self {
        Self {
            input: InputBuffer::new("fire"),
            selected: 0,
        }
    }

    /// The frame at a realistic size - it fills its parent, so a story has
    /// to give it one.
    fn frame(&self, cx: &Context<Self>) -> AnyElement {
        let weak = cx.entity().downgrade();

        div()
            .w(px(560.))
            .h(px(320.))
            .child(
                LauncherFrame::new(render_input_line(&self.input, "Search apps...", cx))
                    .badge("Applications")
                    .hints("@ apps · $ shell · ! web · ? help")
                    .actions(footer_hints(vec![("Open", "Enter"), ("Close", "Esc")], cx))
                    .children(MATCHES.iter().enumerate().map(|(ix, (icon, name, desc))| {
                        let weak = weak.clone();
                        ListItem::new(SharedString::from(format!("match-{name}")))
                            .spacing(ListItemSpacing::Sparse)
                            .toggle_state(ix == self.selected)
                            .start_slot(Icon::new(*icon))
                            .child(
                                v_flex().child(Label::new(*name)).child(
                                    Label::new(*desc).size(TextSize::Small).color(Color::Muted),
                                ),
                            )
                            .on_click(move |_event, _window, cx| {
                                weak.update(cx, |this, cx| {
                                    this.selected = ix;
                                    cx.notify();
                                })
                                .ok();
                            })
                            .into_any_element()
                    })),
            )
            .into_any_element()
    }
}

impl Render for LauncherStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::XLarge.pixels()).child(example_group(
            "Launcher",
            vec![
                example("Query, matches, hints", self.frame(cx))
                    .description("Click a row to move the selection."),
                example(
                    "Slots are optional",
                    div()
                        .w(px(560.))
                        .h(px(140.))
                        .child(
                            LauncherFrame::new(render_input_line(
                                &InputBuffer::default(),
                                "Type a command...",
                                cx,
                            ))
                            .child(
                                v_flex()
                                    .p(Spacing::XLarge.pixels())
                                    .child(Label::new("No badge, no footer.").color(Color::Muted)),
                            ),
                        )
                        .into_any_element(),
                )
                .description("Drop the badge and the hints and the footer bar goes with them."),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| LauncherStory::new()).into()
}

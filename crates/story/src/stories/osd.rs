//! OSD - the pill that flashes on a volume or brightness change.

use gpui::{AnyElement, div};
use ui::IconName;
use ui::patterns::OsdIndicator;

use crate::prelude::*;

/// Same icons the shell's own OSD uses.
const VOLUME: IconName = IconName::Volume;
const MUTED: IconName = IconName::VolumeOff;
const BRIGHT: IconName = IconName::Sun;

/// Each pill springs under its own key, as every one here is a different
/// quantity.
fn horizontal(
    key: &'static str,
    icon: IconName,
    level: u8,
    fill: Option<gpui::Hsla>,
) -> AnyElement {
    div()
        .w(px(264.))
        .h(px(40.))
        .child({
            let osd = OsdIndicator::new(icon, level).key(key);
            match fill {
                Some(color) => osd.fill(color).icon_color(color),
                None => osd,
            }
        })
        .into_any_element()
}

pub struct OsdStory {
    /// The live example's level.
    level: u8,
}

impl Render for OsdStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = cx.theme().colors().status;
        let weak = cx.entity().downgrade();
        let nudge = move |delta: i16| {
            let weak = weak.clone();
            move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
                weak.update(cx, |this, cx| {
                    this.level = (this.level as i16 + delta).clamp(0, 100) as u8;
                    cx.notify();
                })
                .ok();
            }
        };

        v_flex().gap(Spacing::XLarge.pixels()).child(example_group(
            "OSD",
            vec![
                example(
                    "Live",
                    v_flex()
                        .gap(Spacing::Medium.pixels())
                        .child(horizontal("osd-live", VOLUME, self.level, None))
                        .child(
                            h_flex()
                                .gap(Spacing::Small.pixels())
                                .child(Button::new("osd-down", "-10").on_click(nudge(-10)))
                                .child(Button::new("osd-up", "+10").on_click(nudge(10))),
                        )
                        .into_any_element(),
                )
                .description("Fill and number spring to each new level together."),
                example("Volume", horizontal("osd-volume", VOLUME, 62, None)),
                example("Brightness", horizontal("osd-brightness", BRIGHT, 35, None)),
                example(
                    "Muted",
                    horizontal("osd-muted", MUTED, 0, Some(status.error)),
                )
                .description(
                    "The caller picks the colour - the pill has no idea what muted means.",
                ),
                example(
                    "Overamplified",
                    horizontal("osd-loud", VOLUME, 130, Some(status.warning)),
                )
                .description("Track clamps at 100%, the number does not."),
                example(
                    "Vertical",
                    div()
                        .w(px(40.))
                        .h(px(264.))
                        .child(
                            OsdIndicator::new(VOLUME, 100)
                                .key("osd-vertical")
                                .vertical(true),
                        )
                        .into_any_element(),
                )
                .description("Value on top, fill still grows from the bottom; 100% fits on one line."),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| OsdStory { level: 62 }).into()
}

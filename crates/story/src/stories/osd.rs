//! OSD - the pill that flashes on a volume or brightness change.

use gpui::{AnyElement, div};
use ui::IconName;
use ui::patterns::OsdIndicator;

use crate::prelude::*;

/// Same icons the shell's own OSD uses.
const VOLUME: IconName = IconName::Volume;
const MUTED: IconName = IconName::VolumeOff;
const BRIGHT: IconName = IconName::Sun;

fn horizontal(icon: IconName, level: u8, fill: Option<gpui::Hsla>) -> AnyElement {
    div()
        .w(px(264.))
        .h(px(40.))
        .child({
            let osd = OsdIndicator::new(icon, level);
            match fill {
                Some(color) => osd.fill(color).icon_color(color),
                None => osd,
            }
        })
        .into_any_element()
}

pub struct OsdStory;

impl Render for OsdStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = cx.theme().colors().status;

        v_flex().gap(Spacing::XLarge.pixels()).child(example_group(
            "OSD",
            vec![
                example("Volume", horizontal(VOLUME, 62, None)),
                example("Brightness", horizontal(BRIGHT, 35, None)),
                example("Muted", horizontal(MUTED, 0, Some(status.error))).description(
                    "The caller picks the colour - the pill has no idea what muted means.",
                ),
                example(
                    "Overamplified",
                    horizontal(VOLUME, 130, Some(status.warning)),
                )
                .description("Track clamps at 100%, the number does not."),
                example(
                    "Vertical",
                    div()
                        .w(px(40.))
                        .h(px(264.))
                        .child(OsdIndicator::new(VOLUME, 62).vertical(true))
                        .into_any_element(),
                )
                .description("Value on top, fill still grows from the bottom."),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| OsdStory).into()
}

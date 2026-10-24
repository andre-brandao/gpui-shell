//! The launcher's default row, and icon drawing shared with views that draw
//! their own.

use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

use gpui::{
    AnyElement, App, EntityId, Pixels, RenderImage, Size, Window, div, img, prelude::*, px, size,
};
use ui::{
    ActiveTheme, Color, HighlightedLabel, Icon, IconSize, Label, LabelCommon, ListItem,
    ListItemSpacing, TextSize, ThumbnailCache, ThumbnailKey, Toggleable, thumbnail,
};

use super::item::{Item, ItemIcon};

const ICON_SLOT: Pixels = px(28.);

/// The thumbnail of `path` sized for `slot`, or `None` while it loads.
/// `owner` is notified when it lands.
pub fn thumbnail_image(
    path: &Path,
    mtime: Option<SystemTime>,
    slot: Size<Pixels>,
    owner: EntityId,
    window: &Window,
    cx: &App,
) -> Option<Arc<RenderImage>> {
    let cache = ThumbnailCache::global(cx)?;
    let target = thumbnail::target_size(slot, window.scale_factor());
    let key = ThumbnailKey::new(path.to_path_buf(), mtime, target);
    match cache.get(&key, owner, cx) {
        ui::Thumbnail::Ready(image) => Some(image),
        ui::Thumbnail::Loading | ui::Thumbnail::Failed => None,
    }
}

pub(super) fn default_row(
    item: &Item,
    highlights: &[usize],
    selected: bool,
    owner: EntityId,
    window: &Window,
    cx: &App,
) -> AnyElement {
    let slot_bg = cx.theme().colors.element_background;
    let icon = match &item.icon {
        ItemIcon::None => None,
        ItemIcon::Icon(source) => Some(
            div()
                .size_full()
                .bg(slot_bg)
                .flex()
                .items_center()
                .justify_center()
                .child(Icon::new(source.clone()).size(IconSize::Small))
                .into_any_element(),
        ),
        ItemIcon::Path(path) => Some(img(path.clone()).size_full().into_any_element()),
        ItemIcon::Thumbnail { path, mtime } => Some(
            div()
                .size_full()
                .bg(slot_bg)
                .children(
                    thumbnail_image(path, *mtime, size(ICON_SLOT, ICON_SLOT), owner, window, cx)
                        .map(|image| img(image).size_full()),
                )
                .into_any_element(),
        ),
    };

    ListItem::new(item.id.clone())
        .spacing(ListItemSpacing::Sparse)
        .toggle_state(selected)
        .when_some(icon, |row, icon| {
            row.start_slot(
                div()
                    .size(ICON_SLOT)
                    .flex_none()
                    .rounded(px(6.))
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon),
            )
        })
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(1.))
                .min_w(px(0.))
                .child(
                    HighlightedLabel::new(item.title.clone(), highlights.to_vec())
                        .size(TextSize::Default),
                )
                .when_some(item.subtitle.clone(), |el, subtitle| {
                    el.child(
                        Label::new(subtitle)
                            .size(TextSize::Small)
                            .color(Color::Muted)
                            .truncate(),
                    )
                }),
        )
        .into_any_element()
}

use gpui::{div, prelude::*, px};
use services::Services;

use super::{bluetooth, wifi};

pub fn row(
    services: &Services,
    wifi_expanded: bool,
    bt_expanded: bool,
    cx: &mut gpui::Context<'_, super::ControlCenter>,
) -> impl IntoElement {
    div()
        .flex()
        .items_start()
        .gap(px(6.0))
        .w_full()
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(wifi::wifi_toggle(services, wifi_expanded, cx)),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(bluetooth::bluetooth_toggle(services, bt_expanded, cx)),
        )
}

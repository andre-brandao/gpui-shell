use gpui::{MouseButton, div, prelude::*, px, rems};
use services::{PowerProfile, Services, UPowerCommand};
use ui::prelude::*;

use super::icons;

pub fn power_card(
    services: &Services,
    cx: &mut gpui::Context<'_, super::ControlCenter>,
) -> impl IntoElement {
    let colors = cx.theme().colors();
    let status = cx.theme().status();

    let upower = services.upower.get();
    let profile = upower.power_profile;
    let battery = upower.battery;
    let profiles_available = upower.power_profiles_available;

    let (battery_icon, battery_value, battery_color) = match &battery {
        Some(b) => {
            use services::BatteryState::*;
            let color = if b.is_critical() {
                status.error
            } else if b.is_low() {
                status.warning
            } else if matches!(b.state, Charging | FullyCharged) {
                status.success
            } else {
                colors.text
            };
            (b.icon().to_string(), format!("{}%", b.percentage), color)
        }
        None => (
            icons::BATTERY.to_string(),
            "--".to_string(),
            colors.text_muted,
        ),
    };
    let source_icon = if upower.on_battery { "" } else { "󰚥" };

    let set_profile = |target: PowerProfile| {
        cx.listener({
            let services = services.clone();
            move |_, _event: &gpui::MouseDownEvent, _window, cx| {
                let services = services.clone();
                cx.spawn(async move |_, _| {
                    let _ = services
                        .upower
                        .dispatch(UPowerCommand::SetPowerProfile(target))
                        .await;
                })
                .detach();
            }
        })
    };

    let set_saver = set_profile(PowerProfile::PowerSaver);
    let set_balanced = set_profile(PowerProfile::Balanced);
    let set_perf = set_profile(PowerProfile::Performance);

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .p(px(8.0))
        .bg(colors.surface_background)
        .rounded(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(rems(0.92))
                        .text_color(colors.text_muted)
                        .child(source_icon),
                )
                .child(
                    div()
                        .text_size(rems(0.9))
                        .text_color(battery_color)
                        .child(battery_icon),
                )
                .child(
                    div()
                        .text_size(rems(0.86))
                        .text_color(battery_color)
                        .child(battery_value),
                ),
        )
        .when(profiles_available, |el| {
            el.child(
                div()
                    .id("power-profile-toggle")
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .id("profile-saver")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(28.0))
                            .h(px(24.0))
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .bg(if profile == PowerProfile::PowerSaver {
                                colors.element_selected
                            } else {
                                colors.element_background
                            })
                            .hover(move |s| s.bg(colors.element_hover))
                            .on_mouse_down(MouseButton::Left, set_saver)
                            .child(
                                div()
                                    .text_size(rems(0.86))
                                    .text_color(if profile == PowerProfile::PowerSaver {
                                        colors.text_accent
                                    } else {
                                        colors.text_muted
                                    })
                                    .child(PowerProfile::PowerSaver.icon()),
                            ),
                    )
                    .child(
                        div()
                            .id("profile-balanced")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(28.0))
                            .h(px(24.0))
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .bg(if profile == PowerProfile::Balanced {
                                colors.element_selected
                            } else {
                                colors.element_background
                            })
                            .hover(move |s| s.bg(colors.element_hover))
                            .on_mouse_down(MouseButton::Left, set_balanced)
                            .child(
                                div()
                                    .text_size(rems(0.86))
                                    .text_color(if profile == PowerProfile::Balanced {
                                        colors.text_accent
                                    } else {
                                        colors.text_muted
                                    })
                                    .child(PowerProfile::Balanced.icon()),
                            ),
                    )
                    .child(
                        div()
                            .id("profile-performance")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(28.0))
                            .h(px(24.0))
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .bg(if profile == PowerProfile::Performance {
                                colors.element_selected
                            } else {
                                colors.element_background
                            })
                            .hover(move |s| s.bg(colors.element_hover))
                            .on_mouse_down(MouseButton::Left, set_perf)
                            .child(
                                div()
                                    .text_size(rems(0.86))
                                    .text_color(if profile == PowerProfile::Performance {
                                        colors.text_accent
                                    } else {
                                        colors.text_muted
                                    })
                                    .child(PowerProfile::Performance.icon()),
                            ),
                    ),
            )
        })
}

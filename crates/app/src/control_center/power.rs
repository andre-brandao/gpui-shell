//! Power section for the Control Center.
//!
//! Displays battery status, charging state, and power profile controls.

use gpui::{App, MouseButton, div, prelude::*, px};
use services::{PowerProfile, Services, UPowerCommand};
use ui::{accent, bg, border, font_size, icon_size, interactive, radius, spacing, status, text};

use super::icons;

/// Render the power section (battery and power profile)
pub fn render_power_section(services: &Services) -> impl IntoElement {
    let upower = services.upower.get();

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(spacing::SM))
        .p(px(spacing::SM))
        .bg(bg::secondary())
        .rounded(px(radius::MD))
        .border_1()
        .border_color(border::subtle())
        // Battery status (if available)
        .when_some(upower.battery.as_ref(), |el, battery| {
            el.child(render_battery_status(battery))
        })
        // Power profile controls
        .child(render_power_profiles(services, upower.power_profile))
}

/// Render battery status display
fn render_battery_status(battery: &services::BatteryData) -> impl IntoElement {
    let percentage = battery.percentage;
    let is_charging = battery.is_charging();
    let is_critical = battery.is_critical();
    let time_remaining = format_time_remaining(battery);

    let icon = icons::battery_icon(percentage, is_charging);
    let color = if is_critical {
        status::error()
    } else if is_charging {
        status::success()
    } else if percentage <= 20 {
        status::warning()
    } else {
        text::primary()
    };

    div()
        .flex()
        .items_center()
        .justify_between()
        .w_full()
        .pb(px(spacing::SM))
        .border_b_1()
        .border_color(border::subtle())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::SM))
                // Battery icon
                .child(
                    div()
                        .text_size(px(icon_size::LG))
                        .text_color(color)
                        .child(icon),
                )
                // Battery info
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(spacing::XS))
                                .child(
                                    div()
                                        .text_size(px(font_size::MD))
                                        .text_color(text::primary())
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .child(format!("{}%", percentage)),
                                )
                                .child(
                                    div()
                                        .text_size(px(font_size::SM))
                                        .text_color(text::muted())
                                        .child(if is_charging {
                                            "Charging"
                                        } else {
                                            "On Battery"
                                        }),
                                ),
                        )
                        .when_some(time_remaining, |el, time| {
                            el.child(
                                div()
                                    .text_size(px(font_size::XS))
                                    .text_color(text::muted())
                                    .child(time),
                            )
                        }),
                ),
        )
        // Battery percentage bar
        .child(render_battery_bar(percentage, is_charging, is_critical))
}

/// Render battery percentage bar
fn render_battery_bar(percentage: u8, charging: bool, critical: bool) -> impl IntoElement {
    let fill_color = if critical {
        status::error()
    } else if charging {
        status::success()
    } else if percentage <= 20 {
        status::warning()
    } else {
        accent::primary()
    };

    let width_percent = (percentage as f32 / 100.0).min(1.0);

    div()
        .w(px(60.))
        .h(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .w(px(50.))
                .h(px(20.))
                .bg(bg::tertiary())
                .rounded(px(3.))
                .border_1()
                .border_color(border::default())
                .relative()
                .overflow_hidden()
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .h_full()
                        .w(gpui::relative(width_percent))
                        .bg(fill_color),
                )
                // Battery terminal
                .child(
                    div()
                        .absolute()
                        .right(px(-4.))
                        .top(px(6.))
                        .w(px(3.))
                        .h(px(8.))
                        .bg(border::default())
                        .rounded_r(px(2.)),
                ),
        )
}

/// Render power profile selector
fn render_power_profiles(services: &Services, current_profile: PowerProfile) -> impl IntoElement {
    let services_saver = services.clone();
    let services_balanced = services.clone();
    let services_performance = services.clone();

    div()
        .flex()
        .flex_col()
        .gap(px(spacing::XS))
        .child(
            div()
                .text_size(px(font_size::XS))
                .text_color(text::muted())
                .child("Power Profile"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::XS))
                .child(render_profile_button(
                    "power-saver",
                    icons::POWER_SAVER,
                    "Power Saver",
                    current_profile == PowerProfile::PowerSaver,
                    move |cx| {
                        let s = services_saver.clone();
                        cx.spawn(async move |_| {
                            let _ = s
                                .upower
                                .dispatch(UPowerCommand::SetPowerProfile(PowerProfile::PowerSaver))
                                .await;
                        })
                        .detach();
                    },
                ))
                .child(render_profile_button(
                    "balanced",
                    icons::POWER_BALANCED,
                    "Balanced",
                    current_profile == PowerProfile::Balanced,
                    move |cx| {
                        let s = services_balanced.clone();
                        cx.spawn(async move |_| {
                            let _ = s
                                .upower
                                .dispatch(UPowerCommand::SetPowerProfile(PowerProfile::Balanced))
                                .await;
                        })
                        .detach();
                    },
                ))
                .child(render_profile_button(
                    "performance",
                    icons::POWER_PERFORMANCE,
                    "Performance",
                    current_profile == PowerProfile::Performance,
                    move |cx| {
                        let s = services_performance.clone();
                        cx.spawn(async move |_| {
                            let _ = s
                                .upower
                                .dispatch(UPowerCommand::SetPowerProfile(PowerProfile::Performance))
                                .await;
                        })
                        .detach();
                    },
                )),
        )
}

/// Render a power profile button
fn render_profile_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    active: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(spacing::XS))
        .py(px(spacing::SM))
        .rounded(px(radius::MD))
        .cursor_pointer()
        .when(active, |el| el.bg(accent::primary()))
        .when(!active, |el| el.bg(interactive::default()))
        .hover(|s| s.bg(interactive::hover()))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            on_click(cx);
        })
        .child(
            div()
                .text_size(px(icon_size::MD))
                .text_color(text::primary())
                .child(icon),
        )
        .child(
            div()
                .text_size(px(font_size::XS))
                .text_color(if active {
                    text::primary()
                } else {
                    text::muted()
                })
                .child(label),
        )
}

/// Format time remaining for battery
fn format_time_remaining(battery: &services::BatteryData) -> Option<String> {
    let duration = if battery.is_charging() {
        battery.time_to_full.as_ref()
    } else {
        battery.time_to_empty.as_ref()
    };

    let seconds = duration?.as_secs();
    if seconds == 0 {
        return None;
    }

    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;

    let time_str = if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    };

    Some(if battery.is_charging() {
        format!("{} until full", time_str)
    } else {
        format!("{} remaining", time_str)
    })
}

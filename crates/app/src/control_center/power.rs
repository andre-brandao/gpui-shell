//! Power section for the Control Center.
//!
//! Displays battery status and power profile controls when expanded.

use gpui::{App, Hsla, MouseButton, div, prelude::*, px};
use services::{PowerProfile, Services, UPowerCommand};
use ui::{ActiveTheme, font_size, icon_size, radius, spacing};

use super::icons;

/// Render the power section (expanded view with battery details and profiles)
pub fn render_power_section(services: &Services, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let upower = services.upower.get();
    let services_clone = services.clone();

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(spacing::XS))
        .p(px(spacing::SM))
        .bg(theme.bg.secondary)
        .rounded(px(radius::MD))
        .border_1()
        .border_color(theme.border.subtle)
        .child(
            // Section header
            div()
                .flex()
                .items_center()
                .gap(px(spacing::SM))
                .pb(px(spacing::XS))
                .child(
                    div()
                        .text_size(px(icon_size::SM))
                        .text_color(theme.text.muted)
                        .child(icons::BATTERY_FULL),
                )
                .child(
                    div()
                        .text_size(px(font_size::SM))
                        .text_color(theme.text.secondary)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child("Power"),
                ),
        )
        // Battery status (if available)
        .when_some(upower.battery.as_ref(), |el, battery| {
            let percentage = battery.percentage;
            let is_charging = battery.is_charging();
            let is_critical = battery.is_critical();
            let time_remaining = format_time_remaining(battery);

            let color = if is_critical {
                theme.status.error
            } else if is_charging {
                theme.status.success
            } else if percentage <= 20 {
                theme.status.warning
            } else {
                theme.text.primary
            };

            el.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(spacing::SM))
                    .py(px(spacing::XS))
                    // Battery icon
                    .child(
                        div()
                            .text_size(px(icon_size::LG))
                            .text_color(color)
                            .child(battery.icon()),
                    )
                    // Battery percentage
                    .child(
                        div()
                            .text_size(px(font_size::LG))
                            .text_color(theme.text.primary)
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(format!("{}%", percentage)),
                    )
                    // Status text
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(theme.text.muted)
                            .child(if is_charging {
                                "Charging"
                            } else {
                                "On Battery"
                            }),
                    )
                    // Time remaining
                    .when_some(time_remaining, |el, time| {
                        el.child(
                            div()
                                .text_size(px(font_size::XS))
                                .text_color(theme.text.muted)
                                .child(format!("· {}", time)),
                        )
                    }),
            )
        })
        // Power profiles
        .child(render_power_profiles(&services_clone, upower.power_profile, cx))
}

/// Render power profile selector
fn render_power_profiles(
    services: &Services,
    current_profile: PowerProfile,
    cx: &App,
) -> impl IntoElement {
    let services_saver = services.clone();
    let services_balanced = services.clone();
    let services_performance = services.clone();

    div()
        .flex()
        .items_center()
        .gap(px(spacing::XS))
        .pt(px(spacing::XS))
        .child(render_profile_button(
            "power-saver",
            icons::POWER_SAVER,
            "Saver",
            current_profile == PowerProfile::PowerSaver,
            cx,
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
            cx,
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
            "Perf",
            current_profile == PowerProfile::Performance,
            cx,
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
        ))
}

/// Render a power profile button
fn render_profile_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    active: bool,
    cx: &App,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    let theme = cx.theme();

    // Pre-compute colors for closures
    let accent_primary = theme.accent.primary;
    let interactive_default = theme.interactive.default;
    let interactive_hover = theme.interactive.hover;
    let text_primary = theme.text.primary;
    let text_muted = theme.text.muted;

    let label_color: Hsla = if active { text_primary } else { text_muted };

    div()
        .id(id)
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(spacing::XS))
        .py(px(spacing::SM))
        .rounded(px(radius::SM))
        .cursor_pointer()
        .when(active, move |el| el.bg(accent_primary))
        .when(!active, move |el| el.bg(interactive_default))
        .hover(move |s| s.bg(interactive_hover))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            on_click(cx);
        })
        .child(
            div()
                .text_size(px(icon_size::SM))
                .text_color(text_primary)
                .child(icon),
        )
        .child(
            div()
                .text_size(px(font_size::XS))
                .text_color(label_color)
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

    Some(if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    })
}

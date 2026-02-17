//! Keyboard layout widget for displaying and cycling keyboard layouts.

mod config;
pub use config::KeyboardLayoutConfig;

use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{Context, MouseButton, Window, div, prelude::*, px};
use services::{CompositorCommand, CompositorState};
use ui::{ActiveTheme, radius};

use super::style;
use crate::config::ActiveConfig;
use crate::state::AppState;

const KEYBOARD_ICON: &str = "󰌌";

/// Keyboard layout widget that displays the current keyboard layout.
pub struct KeyboardLayout {
    compositor: services::CompositorSubscriber,
    state: CompositorState,
}

impl KeyboardLayout {
    /// Create a new KeyboardLayout widget.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let compositor = AppState::compositor(cx).clone();
        let state = compositor.get();

        // Subscribe to compositor state changes
        cx.spawn({
            let mut signal = compositor.subscribe().to_stream();
            async move |this, cx| {
                while let Some(new_state) = signal.next().await {
                    let result = this.update(cx, |this, cx| {
                        this.state = new_state;
                        cx.notify();
                    });
                    if result.is_err() {
                        break;
                    }
                }
            }
        })
        .detach();

        Self { compositor, state }
    }

    /// Cycle to the next keyboard layout.
    fn next_layout(&self) {
        if let Err(e) = self
            .compositor
            .dispatch(CompositorCommand::NextKeyboardLayout)
        {
            tracing::error!("Failed to switch keyboard layout: {}", e);
        }
    }

    /// Get a short display name for the keyboard layout.
    /// Converts full layout names like "English (US)" to short codes like "EN".
    fn short_layout_name(&self) -> String {
        let layout = &self.state.keyboard_layout;
        let layout_lower = layout.to_lowercase();

        // Common layout name mappings
        let short = if layout_lower.contains("english") {
            "EN"
        } else if layout_lower.contains("russian") {
            "RU"
        } else if layout_lower.contains("german") {
            "DE"
        } else if layout_lower.contains("french") {
            "FR"
        } else if layout_lower.contains("spanish") {
            "ES"
        } else if layout_lower.contains("italian") {
            "IT"
        } else if layout_lower.contains("portuguese") {
            "PT"
        } else if layout_lower.contains("japanese") {
            "JP"
        } else if layout_lower.contains("chinese") {
            "CN"
        } else if layout_lower.contains("korean") {
            "KR"
        } else if layout_lower.contains("arabic") {
            "AR"
        } else if layout_lower.contains("hebrew") {
            "HE"
        } else if layout_lower.contains("ukrainian") {
            "UA"
        } else if layout_lower.contains("polish") {
            "PL"
        } else if layout_lower.contains("czech") {
            "CZ"
        } else if layout_lower.contains("dutch") {
            "NL"
        } else if layout_lower.contains("swedish") {
            "SE"
        } else if layout_lower.contains("norwegian") {
            "NO"
        } else if layout_lower.contains("danish") {
            "DK"
        } else if layout_lower.contains("finnish") {
            "FI"
        } else if layout_lower.contains("turkish") {
            "TR"
        } else if layout_lower.contains("greek") {
            "GR"
        } else if layout.chars().count() >= 2 {
            // Fallback: take first 2 Unicode chars uppercase.
            return layout.chars().take(2).collect::<String>().to_uppercase();
        } else {
            return layout.to_uppercase();
        };

        short.to_uppercase()
    }

    fn flag_for_layout(&self) -> Option<&'static str> {
        match self.short_layout_name().as_str() {
            "EN" => Some("🇺🇸"),
            "RU" => Some("🇷🇺"),
            "DE" => Some("🇩🇪"),
            "FR" => Some("🇫🇷"),
            "ES" => Some("🇪🇸"),
            "IT" => Some("🇮🇹"),
            "PT" => Some("🇵🇹"),
            "JP" => Some("🇯🇵"),
            "CN" => Some("🇨🇳"),
            "KR" => Some("🇰🇷"),
            "AR" => Some("🇸🇦"),
            "HE" => Some("🇮🇱"),
            "UA" => Some("🇺🇦"),
            "PL" => Some("🇵🇱"),
            "CZ" => Some("🇨🇿"),
            "NL" => Some("🇳🇱"),
            "SE" => Some("🇸🇪"),
            "NO" => Some("🇳🇴"),
            "DK" => Some("🇩🇰"),
            "FI" => Some("🇫🇮"),
            "TR" => Some("🇹🇷"),
            "GR" => Some("🇬🇷"),
            _ => None,
        }
    }
}

impl Render for KeyboardLayout {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().bar.is_vertical();
        let config = &cx.config().bar.modules.keyboard_layout;
        let short_name = self.short_layout_name();
        let icon = if config.show_flag {
            self.flag_for_layout().unwrap_or(KEYBOARD_ICON)
        } else {
            KEYBOARD_ICON
        };

        // Pre-compute colors for closures
        let interactive_default = theme.interactive.default;
        let interactive_hover = theme.interactive.hover;
        let interactive_active = theme.interactive.active;
        let text_secondary = theme.text.secondary;
        let text_primary = theme.text.primary;
        let icon_size = style::icon(is_vertical);
        let text_size = style::label(is_vertical);

        div()
            .id("keyboard-layout")
            .flex()
            .when(is_vertical, |this| this.flex_col())
            .items_center()
            .gap(px(style::CHIP_GAP))
            .px(px(style::chip_padding_x(is_vertical)))
            .py(px(style::CHIP_PADDING_Y))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .bg(interactive_default)
            .hover(move |s| s.bg(interactive_hover))
            .active(move |s| s.bg(interactive_active))
            // Click to cycle layout
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, _window, _cx| {
                    this.next_layout();
                }),
            )
            .child(
                div()
                    .text_size(px(icon_size))
                    .text_color(text_secondary)
                    .child(icon),
            )
            .child(
                div()
                    .text_size(px(text_size))
                    .text_color(text_primary)
                    .child(short_name),
            )
    }
}

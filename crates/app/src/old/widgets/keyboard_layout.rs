//! Keyboard layout widget for displaying and cycling keyboard layouts.

use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{Context, MouseButton, Window, div, prelude::*, px};
use services::{CompositorCommand, CompositorState, CompositorSubscriber};
use ui::{ActiveTheme, radius, spacing};

/// Keyboard layout widget that displays the current keyboard layout.
pub struct KeyboardLayout {
    compositor: CompositorSubscriber,
    state: CompositorState,
}

impl KeyboardLayout {
    /// Create a new KeyboardLayout widget with the given compositor subscriber.
    pub fn new(compositor: CompositorSubscriber, cx: &mut Context<Self>) -> Self {
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

        // Common layout name mappings
        let short = if layout.to_lowercase().contains("english") {
            "EN"
        } else if layout.to_lowercase().contains("russian") {
            "RU"
        } else if layout.to_lowercase().contains("german") {
            "DE"
        } else if layout.to_lowercase().contains("french") {
            "FR"
        } else if layout.to_lowercase().contains("spanish") {
            "ES"
        } else if layout.to_lowercase().contains("italian") {
            "IT"
        } else if layout.to_lowercase().contains("portuguese") {
            "PT"
        } else if layout.to_lowercase().contains("japanese") {
            "JP"
        } else if layout.to_lowercase().contains("chinese") {
            "CN"
        } else if layout.to_lowercase().contains("korean") {
            "KR"
        } else if layout.to_lowercase().contains("arabic") {
            "AR"
        } else if layout.to_lowercase().contains("hebrew") {
            "HE"
        } else if layout.to_lowercase().contains("ukrainian") {
            "UA"
        } else if layout.to_lowercase().contains("polish") {
            "PL"
        } else if layout.to_lowercase().contains("czech") {
            "CZ"
        } else if layout.to_lowercase().contains("dutch") {
            "NL"
        } else if layout.to_lowercase().contains("swedish") {
            "SE"
        } else if layout.to_lowercase().contains("norwegian") {
            "NO"
        } else if layout.to_lowercase().contains("danish") {
            "DK"
        } else if layout.to_lowercase().contains("finnish") {
            "FI"
        } else if layout.to_lowercase().contains("turkish") {
            "TR"
        } else if layout.to_lowercase().contains("greek") {
            "GR"
        } else if layout.len() >= 2 {
            // Fallback: take first 2 characters uppercase
            &layout[..2]
        } else {
            layout
        };

        short.to_uppercase()
    }
}

impl Render for KeyboardLayout {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let short_name = self.short_layout_name();

        // Pre-compute colors for closures
        let interactive_default = theme.interactive.default;
        let interactive_hover = theme.interactive.hover;
        let interactive_active = theme.interactive.active;
        let text_secondary = theme.text.secondary;
        let text_primary = theme.text.primary;

        div()
            .id("keyboard-layout")
            .flex()
            .items_center()
            .gap(px(spacing::XS))
            .px(px(spacing::SM))
            .py(px(2.))
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
                    .text_color(text_secondary)
                    // Keyboard icon
                    .child("󰌌 "),
            )
            .child(div().text_color(text_primary).child(short_name))
    }
}

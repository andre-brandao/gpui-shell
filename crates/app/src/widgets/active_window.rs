//! Active window widget displaying the title of the currently focused window.

use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{Context, Render, Window, div, prelude::*, px};
use services::{CompositorState, CompositorSubscriber};
use ui::{ActiveTheme, font_size, spacing};

/// Maximum characters to display before truncating the title.
const MAX_TITLE_LENGTH: usize = 60;

/// Widget that displays the currently focused window's title.
pub struct ActiveWindow {
    _compositor: CompositorSubscriber,
    state: CompositorState,
}

impl ActiveWindow {
    /// Create a new active window widget.
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

        Self {
            _compositor: compositor,
            state,
        }
    }

    /// Get the display title, truncated if necessary.
    fn display_title(&self) -> String {
        let title = self
            .state
            .active_window
            .as_ref()
            .map(|w| w.title.as_str())
            .unwrap_or("");

        if title.is_empty() {
            return String::new();
        }

        if let Some((cutoff, _)) = title.char_indices().nth(MAX_TITLE_LENGTH) {
            format!("{}…", &title[..cutoff])
        } else {
            title.to_string()
        }
    }
}

impl Render for ActiveWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let title = self.display_title();

        div()
            .id("active-window")
            .flex()
            .items_center()
            .justify_center()
            .px(px(spacing::MD))
            .text_size(px(font_size::SM))
            .text_color(theme.text.secondary)
            .overflow_hidden()
            .text_ellipsis()
            .child(title)
    }
}

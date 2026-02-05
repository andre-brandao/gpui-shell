//! Active window widget displaying the focused window title.

use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{Context, Window, div, prelude::*, px, rems};
use services::{CompositorState, CompositorSubscriber, Services};
use ui::prelude::*;

/// Maximum characters to display before truncating the title.
const MAX_TITLE_LENGTH: usize = 60;

pub struct ActiveWindow {
    compositor: CompositorSubscriber,
    state: CompositorState,
}

impl ActiveWindow {
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
        let compositor = services.compositor.clone();
        let state = compositor.get();

        // Subscribe to compositor updates.
        cx.spawn({
            let mut signal = compositor.subscribe().to_stream();
            async move |this, cx| {
                while let Some(new_state) = signal.next().await {
                    if this
                        .update(cx, |this, cx| {
                            this.state = new_state;
                            cx.notify();
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            }
        })
        .detach();

        Self { compositor, state }
    }

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

        if title.len() > MAX_TITLE_LENGTH {
            format!("{}…", &title[..MAX_TITLE_LENGTH])
        } else {
            title.to_string()
        }
    }
}

impl Render for ActiveWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();
        let title = self.display_title();

        div()
            .id("active-window")
            .flex()
            .items_center()
            .justify_center()
            .px(px(10.0))
            .text_size(rems(0.75))
            .text_color(colors.text_muted)
            .overflow_hidden()
            .text_ellipsis()
            .child(title)
    }
}

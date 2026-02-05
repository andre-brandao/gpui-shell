//! Launcher module providing an application launcher overlay.
//!
//! The launcher provides a keyboard-driven interface for:
//! - Searching and launching applications (@ prefix, or default)
//! - Running shell commands ($ prefix)
//! - Web search with multiple providers (! prefix with shebangs)
//! - Switching workspaces (;ws prefix)
//! - Viewing help and available commands (? prefix)

pub mod view;
mod views;

use futures_signals::signal::SignalExt;
use gpui::{
    App, Bounds, Context, FocusHandle, Focusable, KeyBinding, Point, ScrollHandle, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, actions,
    div, layer_shell::*, prelude::*, px,
};
use services::Services;
use ui::{ActiveTheme, font_size, icon_size, radius, spacing};
use view::{InputResult, LIST_ITEM_HEIGHT, LauncherView, ViewContext, ViewInput, is_special_char};
use views::{HelpView, all_views};

actions!(launcher, [Escape, Enter]);

const LAUNCHER_WIDTH: f32 = 600.0;
const LAUNCHER_HEIGHT: f32 = 450.0;

/// Visible height of the content area (approximate).
const VISIBLE_HEIGHT: f32 = 350.0;
/// Number of items to jump when using Page Up/Down.
const ITEMS_PER_PAGE: usize = 7;

/// The main launcher struct.
pub struct Launcher {
    services: Services,
    search_query: String,
    selected_index: usize,
    item_count: usize,
    focus_handle: FocusHandle,
    scroll_handle: ScrollHandle,
    views: Vec<Box<dyn LauncherView>>,
    help_view: HelpView,
}

impl Launcher {
    /// Create a new launcher with the given services and optional initial input.
    pub fn new(services: Services, initial_input: Option<String>, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let scroll_handle = ScrollHandle::new();

        // Subscribe to service updates for reactive rendering
        cx.spawn({
            let mut compositor_signal = services.compositor.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while compositor_signal.next().await.is_some() {
                    let should_continue = this.update(cx, |_, cx| cx.notify()).is_ok();
                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

        cx.spawn({
            let mut sysinfo_signal = services.sysinfo.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while sysinfo_signal.next().await.is_some() {
                    let should_continue = this.update(cx, |_, cx| cx.notify()).is_ok();
                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

        cx.spawn({
            let mut upower_signal = services.upower.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while upower_signal.next().await.is_some() {
                    let should_continue = this.update(cx, |_, cx| cx.notify()).is_ok();
                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

        let views = all_views();
        let help_view = HelpView::new(&views);

        Launcher {
            services,
            search_query: initial_input.unwrap_or_default(),
            selected_index: 0,
            item_count: 0,
            focus_handle,
            scroll_handle,
            views,
            help_view,
        }
    }

    /// Set the search query (used for IPC input).
    pub fn set_input(&mut self, input: String) {
        self.search_query = input;
        self.selected_index = 0;
        self.reset_scroll();
    }

    /// Reset scroll to top.
    fn reset_scroll(&self) {
        self.scroll_handle.set_offset(gpui::point(px(0.), px(0.)));
    }

    /// Scroll to keep the selected item visible.
    fn scroll_to_selected(&self) {
        let selected_top = px(self.selected_index as f32 * LIST_ITEM_HEIGHT);
        let selected_bottom = selected_top + px(LIST_ITEM_HEIGHT);

        let current_offset = self.scroll_handle.offset();
        let scroll_top = -current_offset.y;
        let scroll_bottom = scroll_top + px(VISIBLE_HEIGHT);

        let new_offset_y = if selected_top < scroll_top {
            // Item is above visible area, scroll up
            -selected_top
        } else if selected_bottom > scroll_bottom {
            // Item is below visible area, scroll down
            -(selected_bottom - px(VISIBLE_HEIGHT))
        } else {
            // Item is already visible
            return;
        };

        self.scroll_handle
            .set_offset(gpui::point(px(0.), new_offset_y));
    }

    /// Parse the search query to find which view should handle it.
    /// Returns (matched_view_or_none, search_term_for_view).
    fn parse_query(&self) -> (Option<&dyn LauncherView>, &str) {
        let query = self.search_query.trim();

        if query.is_empty() {
            return (self.default_view(), "");
        }

        // Check if query starts with any view's prefix
        // We need to find the longest matching prefix first
        let mut best_match: Option<(&dyn LauncherView, usize)> = None;

        for view in &self.views {
            let prefix = view.prefix();
            if query.starts_with(prefix) {
                // Check if this is a better (longer) match
                if best_match.is_none() || prefix.len() > best_match.unwrap().1 {
                    best_match = Some((view.as_ref(), prefix.len()));
                }
            }
        }

        // Also check help view
        if query.starts_with(self.help_view.prefix()) {
            let prefix_len = self.help_view.prefix().len();
            if best_match.is_none() || prefix_len > best_match.unwrap().1 {
                best_match = Some((&self.help_view, prefix_len));
            }
        }

        if let Some((view, prefix_len)) = best_match {
            let rest = query[prefix_len..].trim_start();
            return (Some(view), rest);
        }

        // Check if starts with a special char but no matching prefix
        // In this case, show help view
        if let Some(first_char) = query.chars().next() {
            if is_special_char(first_char) {
                // Unknown special prefix - show help
                return (Some(&self.help_view), query);
            }
        }

        // No prefix, use default view with full query as search
        (self.default_view(), query)
    }

    /// Get the default view.
    fn default_view(&self) -> Option<&dyn LauncherView> {
        self.views
            .iter()
            .find(|v| v.is_default())
            .map(|v| v.as_ref())
    }

    /// Get the current active view.
    fn current_view(&self) -> &dyn LauncherView {
        let (view, _) = self.parse_query();
        view.unwrap_or(&self.help_view)
    }

    /// Get the current view name for display.
    fn current_view_name(&self) -> &str {
        self.current_view().name()
    }

    /// Create view context for rendering.
    fn view_context(&self) -> ViewContext<'_> {
        let (_, search) = self.parse_query();
        ViewContext {
            services: &self.services,
            query: search,
            selected_index: self.selected_index,
        }
    }

    fn handle_input(&mut self, input: ViewInput, cx: &mut App) -> bool {
        let vx = self.view_context();
        let view = self.current_view();

        match view.handle_input(&input, &vx, cx) {
            InputResult::Handled { query, close } => {
                // Update search query based on current view prefix
                let (matched_view, _) = self.parse_query();
                if let Some(v) = matched_view {
                    let prefix = v.prefix();
                    if query.is_empty() {
                        self.search_query = prefix.to_string();
                    } else {
                        self.search_query = format!("{} {}", prefix, query);
                    }
                } else {
                    self.search_query = query;
                }
                self.selected_index = 0;
                close
            }
            InputResult::Unhandled => {
                // Default handling
                match input {
                    ViewInput::Char(c) => {
                        self.search_query.push_str(&c);
                        self.selected_index = 0;
                        self.reset_scroll();
                    }
                    ViewInput::Backspace => {
                        self.search_query.pop();
                        self.selected_index = 0;
                        self.reset_scroll();
                    }
                    ViewInput::Up => {
                        if self.item_count > 0 {
                            self.selected_index = if self.selected_index == 0 {
                                self.item_count - 1
                            } else {
                                self.selected_index - 1
                            };
                            self.scroll_to_selected();
                        }
                    }
                    ViewInput::Down => {
                        if self.item_count > 0 {
                            self.selected_index = (self.selected_index + 1) % self.item_count;
                            self.scroll_to_selected();
                        }
                    }
                    ViewInput::PageUp => {
                        if self.item_count > 0 {
                            self.selected_index =
                                self.selected_index.saturating_sub(ITEMS_PER_PAGE);
                            self.scroll_to_selected();
                        }
                    }
                    ViewInput::PageDown => {
                        if self.item_count > 0 {
                            self.selected_index = (self.selected_index + ITEMS_PER_PAGE)
                                .min(self.item_count.saturating_sub(1));
                            self.scroll_to_selected();
                        }
                    }
                    ViewInput::Enter => {
                        return self.execute_selected(cx);
                    }
                }
                false
            }
        }
    }

    fn execute_selected(&mut self, cx: &mut App) -> bool {
        let view = self.current_view();

        // Check if we're in help view and selected a command
        if std::ptr::eq(view, &self.help_view as &dyn LauncherView) {
            // In help view, switch to selected view
            if let Some(target_view) = self.views.get(self.selected_index) {
                let prefix = target_view.prefix();
                self.search_query = format!("{} ", prefix);
                self.selected_index = 0;
                return false;
            }
        }

        let vx = self.view_context();
        view.on_select(self.selected_index, &vx, cx)
    }

    fn placeholder(&self) -> String {
        "Search apps or type @, $, !, ? for commands...".to_string()
    }
}

impl Focusable for Launcher {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Launcher {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Always keep the launcher focused
        if !self.focus_handle.is_focused(window) {
            self.focus_handle.focus(window, cx);
        }

        let theme = cx.theme();

        let query = self.search_query.clone();
        let view_name = self.current_view_name().to_string();
        let placeholder = self.placeholder();
        let is_empty = self.search_query.is_empty();

        // Render current view
        let vx = self.view_context();
        let current_view = self.current_view();
        let (view_content, item_count) = current_view.render(&vx, cx);
        let footer_actions = current_view.footer_actions(&vx);
        self.item_count = item_count;

        // Clamp selected index
        if self.selected_index >= item_count && item_count > 0 {
            self.selected_index = item_count - 1;
        }

        // Pre-compute colors for closures
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let text_secondary = theme.text.secondary;
        let text_disabled = theme.text.disabled;
        let text_placeholder = theme.text.placeholder;
        let bg_primary = theme.bg.primary;
        let border_default = theme.border.default;
        let interactive_default = theme.interactive.default;

        div()
            .id("launcher")
            .track_focus(&self.focus_handle)
            .key_context("Launcher")
            .on_action(cx.listener(|_, _: &Escape, window, _cx| {
                // Clear the static handle before removing window
                *LAUNCHER_WINDOW.lock().unwrap() = None;
                window.remove_window();
            }))
            .on_action(cx.listener(|this, _: &Enter, window, cx| {
                if this.handle_input(ViewInput::Enter, cx) {
                    // Clear the static handle before removing window
                    *LAUNCHER_WINDOW.lock().unwrap() = None;
                    window.remove_window();
                }
                cx.notify();
            }))
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    let should_close = match event.keystroke.key.as_str() {
                        "up" => this.handle_input(ViewInput::Up, cx),
                        "down" => this.handle_input(ViewInput::Down, cx),
                        "pageup" => this.handle_input(ViewInput::PageUp, cx),
                        "pagedown" => this.handle_input(ViewInput::PageDown, cx),
                        "backspace" => this.handle_input(ViewInput::Backspace, cx),
                        _ => {
                            if let Some(key_char) = &event.keystroke.key_char {
                                this.handle_input(ViewInput::Char(key_char.clone()), cx)
                            } else if event.keystroke.key.len() == 1
                                && !event.keystroke.modifiers.control
                                && !event.keystroke.modifiers.alt
                            {
                                this.handle_input(ViewInput::Char(event.keystroke.key.clone()), cx)
                            } else {
                                false
                            }
                        }
                    };
                    if should_close {
                        // Clear the static handle before removing window
                        *LAUNCHER_WINDOW.lock().unwrap() = None;
                        window.remove_window();
                    }
                    cx.notify();
                }),
            )
            .size_full()
            .bg(bg_primary)
            .border_1()
            .border_color(border_default)
            .rounded(px(radius::LG))
            .text_color(text_primary)
            .flex()
            .flex_col()
            .overflow_hidden()
            // Search input area
            .child(
                div()
                    .w_full()
                    .px(px(spacing::LG))
                    .py(px(spacing::MD))
                    .flex()
                    .items_center()
                    .gap(px(spacing::MD))
                    // Search icon
                    .child(
                        div()
                            .text_size(px(icon_size::LG))
                            .text_color(text_muted)
                            .child(""),
                    )
                    // Search text
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(font_size::MD))
                            .child(if query.is_empty() { placeholder } else { query })
                            .text_color(if is_empty {
                                text_placeholder
                            } else {
                                text_primary
                            }),
                    )
                    // View badge (right side)
                    .child(
                        div()
                            .px(px(spacing::SM))
                            .py(px(3.))
                            .rounded(px(radius::SM))
                            .bg(interactive_default)
                            .text_size(px(font_size::SM))
                            .text_color(text_secondary)
                            .child(view_name),
                    ),
            )
            // Divider line
            .child(div().w_full().h(px(1.)).bg(border_default))
            // View content with scroll support
            .child(
                div()
                    .id("view-content")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .py(px(spacing::XS))
                    .child(view_content),
            )
            // Bottom footer bar
            .child(div().w_full().h(px(1.)).bg(border_default))
            .child(
                div()
                    .w_full()
                    .px(px(spacing::LG))
                    .py(px(spacing::SM))
                    .flex()
                    .items_center()
                    .justify_between()
                    // Left side - prefix hints
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::SM))
                            .text_size(px(font_size::XS))
                            .text_color(text_disabled)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .child("@apps")
                                    .child("·")
                                    .child("$shell")
                                    .child("·")
                                    .child("!web")
                                    .child("·")
                                    .child("?help"),
                            ),
                    )
                    // Right side - action hints from view
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::LG))
                            .text_size(px(font_size::SM))
                            .text_color(text_muted)
                            .children(footer_actions.into_iter().map(|(action, key)| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(spacing::SM - 2.0))
                                    .child(action)
                                    .child(
                                        div()
                                            .px(px(spacing::SM - 2.0))
                                            .py(px(2.))
                                            .rounded(px(radius::SM - 1.0))
                                            .bg(interactive_default)
                                            .text_size(px(font_size::XS))
                                            .child(key),
                                    )
                            })),
                    ),
            )
    }
}

/// Global state to track the launcher window.
static LAUNCHER_WINDOW: std::sync::Mutex<Option<WindowHandle<Launcher>>> =
    std::sync::Mutex::new(None);

/// Register keybindings for the launcher.
pub fn register_keybindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Escape, Some("Launcher")),
        KeyBinding::new("enter", Enter, Some("Launcher")),
    ]);
}

/// Toggle the launcher window.
pub fn toggle(services: Services, cx: &mut App) {
    toggle_with_input(services, None, cx);
}

pub fn toggle_from_ipc(services: Services, input: Option<String>, cx: &mut App) {
    let start = std::time::Instant::now();
    tracing::debug!("toggle_from_ipc: start");

    let mut guard = LAUNCHER_WINDOW.lock().unwrap();
    tracing::debug!("toggle_from_ipc: acquired lock {:?}", start.elapsed());

    if let Some(handle) = guard.take() {
        let _ = handle.update(cx, |_, window, _| {
            window.remove_window();
        });
        tracing::debug!("toggle_from_ipc: closed window {:?}", start.elapsed());
    } else {
        tracing::debug!("toggle_from_ipc: opening new window {:?}", start.elapsed());
        drop(guard);
        toggle_with_input(services, input, cx);
    }
    tracing::debug!("toggle_from_ipc: done {:?}", start.elapsed());
}

/// Toggle the launcher window with optional prefilled input.
pub fn toggle_with_input(services: Services, input: Option<String>, cx: &mut App) {
    let start = std::time::Instant::now();
    tracing::debug!("toggle_with_input: start");

    let mut guard = LAUNCHER_WINDOW.lock().unwrap();
    tracing::debug!("toggle_with_input: acquired lock {:?}", start.elapsed());

    if let Some(handle) = guard.take() {
        // If input is provided, update existing launcher instead of closing
        if let Some(input_text) = input {
            let update_result = handle.update(cx, |launcher, _, cx| {
                launcher.set_input(input_text);
                cx.notify();
            });
            if update_result.is_ok() {
                *guard = Some(handle);
                return;
            }
        }
        // No input or update failed, close the window
        let _ = handle.update(cx, |_, window, _| {
            window.remove_window();
        });
        tracing::debug!("toggle_with_input: closed window {:?}", start.elapsed());
    } else {
        tracing::debug!(
            "toggle_with_input: opening new window {:?}",
            start.elapsed()
        );
        if let Ok(handle) = cx.open_window(
            WindowOptions {
                titlebar: None,
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::new(px(0.), px(0.)),
                    size: Size::new(px(LAUNCHER_WIDTH), px(LAUNCHER_HEIGHT)),
                })),
                app_id: Some("gpuishell-launcher".to_string()),
                window_background: WindowBackgroundAppearance::Transparent,
                kind: WindowKind::LayerShell(LayerShellOptions {
                    namespace: "launcher".to_string(),
                    layer: Layer::Overlay,
                    anchor: Anchor::TOP,
                    exclusive_zone: None,
                    margin: Some((px(100.), px(0.), px(0.), px(0.))),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    ..Default::default()
                }),
                focus: true,
                ..Default::default()
            },
            move |_, cx| {
                cx.new(|cx| {
                    let new_start = std::time::Instant::now();
                    let launcher = Launcher::new(services.clone(), input.clone(), cx);
                    tracing::debug!(
                        "toggle_with_input: Launcher::new took {:?}",
                        new_start.elapsed()
                    );
                    launcher
                })
            },
        ) {
            *guard = Some(handle);
            tracing::debug!("toggle_with_input: window opened {:?}", start.elapsed());
        }
    }
    tracing::debug!("toggle_with_input: done {:?}", start.elapsed());
}

/// Open the launcher (alias for toggle).
pub fn open(services: Services, cx: &mut App) {
    toggle(services, cx);
}

/// Open the launcher with prefilled input.
/// If already open, updates the input. If closed, opens with the input.
pub fn open_with_input(services: Services, input: Option<String>, cx: &mut App) {
    let start = std::time::Instant::now();
    tracing::debug!("open_with_input: start");

    let guard = LAUNCHER_WINDOW.lock().unwrap();
    tracing::debug!("open_with_input: acquired lock {:?}", start.elapsed());

    if let Some(handle) = &*guard {
        // Launcher already open, update input if provided
        if let Some(input_text) = input {
            let _ = handle.update(cx, |launcher, _, cx| {
                launcher.set_input(input_text);
                cx.notify();
            });
        }
    } else {
        // Launcher not open, open it with input
        tracing::debug!(
            "open_with_input: delegating to toggle_with_input {:?}",
            start.elapsed()
        );
        drop(guard); // Release lock before calling toggle_with_input
        toggle_with_input(services, input, cx);
    }
    tracing::debug!("open_with_input: done {:?}", start.elapsed());
}

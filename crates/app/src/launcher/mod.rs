//! Launcher module — keyboard-driven command palette overlay.
//!
//! Provides a prefix-routed search interface for:
//! - Searching and launching applications (`@` prefix, or default)
//! - Running shell commands (`$` prefix)
//! - Web search with multiple providers (`!` prefix)
//! - Viewing help and available commands (`?` prefix)

pub mod view;
mod views;

use gpui::{
    App, Bounds, Context, FocusHandle, Focusable, KeyBinding, Point, ScrollHandle, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, actions,
    div, layer_shell::*, prelude::*, px,
};
use services::Services;
use ui::{ActiveTheme, Color, Label, LabelSize, prelude::*};
use view::{ViewEvent, ViewHandle};
use views::register_views;

actions!(launcher, [Escape, Enter]);

const LAUNCHER_WIDTH: f32 = 600.0;
const LAUNCHER_HEIGHT: f32 = 450.0;
const ITEMS_PER_PAGE: usize = 7;

const SPECIAL_CHARS: &[char] = &['@', '$', '!', '?', ';', '~', '#', ':'];

pub struct Launcher {
    #[allow(dead_code)]
    services: Services,
    query: String,
    selected_index: usize,
    focus_handle: FocusHandle,
    scroll_handle: ScrollHandle,
    views: Vec<ViewHandle>,
    active_view_index: Option<usize>,
    pending_close: bool,
}

impl Launcher {
    pub fn new(services: Services, initial_input: Option<String>, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let scroll_handle = ScrollHandle::new();

        let views = register_views(&services, cx);

        let query = initial_input.unwrap_or_default();

        let mut launcher = Launcher {
            services,
            query: String::new(),
            selected_index: 0,
            focus_handle,
            scroll_handle,
            views,
            active_view_index: None,
            pending_close: false,
        };

        launcher.set_query(query, cx);
        launcher
    }

    fn set_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.query = query;
        let (view_idx, view_query) = self.route_query();
        let changed_view = self.active_view_index != Some(view_idx);
        self.active_view_index = Some(view_idx);

        self.views[view_idx].set_query(&view_query, cx);

        if changed_view {
            self.selected_index = 0;
            self.reset_scroll();
        }

        cx.notify();
    }

    fn route_query(&self) -> (usize, String) {
        let query = self.query.trim();

        if query.is_empty() {
            if let Some(idx) = self.default_view_index() {
                return (idx, String::new());
            }
            return (self.help_view_index(), String::new());
        }

        // Find longest matching prefix
        let mut best: Option<(usize, usize)> = None;
        for (i, view_handle) in self.views.iter().enumerate() {
            let prefix = view_handle.meta.prefix;
            if query.starts_with(prefix) && (best.is_none() || prefix.len() > best.unwrap().1) {
                best = Some((i, prefix.len()));
            }
        }

        if let Some((idx, prefix_len)) = best {
            let rest = query[prefix_len..].trim_start().to_string();
            return (idx, rest);
        }

        // Starts with a special char but no match -> help
        if query.starts_with(SPECIAL_CHARS) {
            return (self.help_view_index(), query.to_string());
        }

        // No prefix -> default view with full query
        let idx = self.default_view_index().unwrap_or(self.help_view_index());
        (idx, query.to_string())
    }

    fn default_view_index(&self) -> Option<usize> {
        self.views.iter().position(|v| v.meta.is_default)
    }

    fn help_view_index(&self) -> usize {
        self.views
            .iter()
            .position(|v| v.meta.id == "help")
            .unwrap_or(0)
    }

    fn active_view(&self) -> &ViewHandle {
        &self.views[self.active_view_index.unwrap_or(0)]
    }

    fn reset_scroll(&self) {
        self.scroll_handle.set_offset(gpui::point(px(0.), px(0.)));
    }

    // ── Input handling ──────────────────────────────────────────────

    fn on_char(&mut self, ch: &str, cx: &mut Context<Self>) {
        self.query.push_str(ch);
        self.selected_index = 0;
        self.reset_scroll();
        let q = self.query.clone();
        self.set_query(q, cx);
    }

    fn on_backspace(&mut self, cx: &mut Context<Self>) {
        self.query.pop();
        self.selected_index = 0;
        self.reset_scroll();
        let q = self.query.clone();
        self.set_query(q, cx);
    }

    fn on_move_up(&mut self, cx: &App) {
        let count = self.active_view().match_count(cx);
        if count > 0 {
            self.selected_index = if self.selected_index == 0 {
                count - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    fn on_move_down(&mut self, cx: &App) {
        let count = self.active_view().match_count(cx);
        if count > 0 {
            self.selected_index = (self.selected_index + 1) % count;
        }
    }

    fn on_page_up(&mut self, cx: &App) {
        let count = self.active_view().match_count(cx);
        if count > 0 {
            self.selected_index = self.selected_index.saturating_sub(ITEMS_PER_PAGE);
        }
    }

    fn on_page_down(&mut self, cx: &App) {
        let count = self.active_view().match_count(cx);
        if count > 0 {
            self.selected_index =
                (self.selected_index + ITEMS_PER_PAGE).min(count.saturating_sub(1));
        }
    }

    fn on_confirm(&mut self, cx: &mut Context<Self>) {
        let idx = self.active_view_index.unwrap_or(0);
        self.views[idx].confirm(self.selected_index, cx);
    }

    fn handle_view_event(&mut self, event: &ViewEvent, cx: &mut Context<Self>) {
        match event {
            ViewEvent::Close => {
                self.pending_close = true;
                cx.notify();
            }
            ViewEvent::SwitchTo(prefix) => {
                let new_query = if prefix.ends_with(' ') {
                    prefix.clone()
                } else {
                    format!("{} ", prefix)
                };
                self.selected_index = 0;
                self.reset_scroll();
                self.set_query(new_query, cx);
            }
            ViewEvent::MatchesUpdated => {
                let count = self.active_view().match_count(cx);
                if self.selected_index >= count && count > 0 {
                    self.selected_index = count - 1;
                }
                cx.notify();
            }
        }
    }

    // ── Render helpers ──────────────────────────────────────────────

    fn render_search_bar(&self, cx: &App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let view = self.active_view();
        let view_name = view.meta.name;
        let is_empty = self.query.is_empty();

        div()
            .w_full()
            .px(px(16.))
            .py(px(12.))
            .flex()
            .items_center()
            .gap(px(12.))
            .child(
                Label::new("\u{f002}")
                    .size(LabelSize::Large)
                    .color(Color::Muted),
            )
            .child(div().flex_1().text_size(px(15.)).child(if is_empty {
                Label::new("Search apps or type @, $, !, ? for commands...")
                    .color(Color::Placeholder)
            } else {
                Label::new(self.query.clone()).color(Color::Default)
            }))
            .child(
                div()
                    .px(px(8.))
                    .py(px(3.))
                    .rounded(px(6.))
                    .bg(colors.element_background)
                    .child(
                        Label::new(view_name)
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    ),
            )
    }

    fn render_view_content(&self, cx: &App) -> impl IntoElement {
        let view = self.active_view();
        let count = view.match_count(cx);

        let selected = if count > 0 {
            self.selected_index.min(count - 1)
        } else {
            0
        };

        div()
            .id("view-content")
            .flex_1()
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .py(px(4.))
            .child(div().flex().flex_col().map(|el| {
                if let Some(content) = view.render_content(cx) {
                    // Content view: header + full content + footer
                    el.when_some(view.render_header(cx), |el, header| el.child(header))
                        .child(content)
                        .when_some(view.render_footer(cx), |el, footer| el.child(footer))
                } else {
                    // List view: header + items + footer
                    el.when_some(view.render_header(cx), |el, header| el.child(header))
                        .children((0..count).map(|i| view.render_item(i, i == selected, cx)))
                        .when_some(view.render_footer(cx), |el, footer| el.child(footer))
                }
            }))
    }

    fn render_footer_bar(&self, cx: &App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let actions = self.active_view().footer_actions(cx);

        div()
            .w_full()
            .px(px(16.))
            .py(px(8.))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div().flex().items_center().gap(px(8.)).children(
                    ["@apps", "$shell", "!web", "?help"]
                        .iter()
                        .enumerate()
                        .flat_map(|(i, hint)| {
                            let mut items: Vec<AnyElement> = Vec::new();
                            if i > 0 {
                                items.push(
                                    Label::new("\u{00b7}")
                                        .size(LabelSize::XSmall)
                                        .color(Color::Disabled)
                                        .into_any_element(),
                                );
                            }
                            items.push(
                                Label::new(*hint)
                                    .size(LabelSize::XSmall)
                                    .color(Color::Disabled)
                                    .into_any_element(),
                            );
                            items
                        }),
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .children(actions.into_iter().map(|action| {
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                Label::new(action.label)
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            )
                            .child(
                                div()
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(px(5.))
                                    .bg(colors.element_background)
                                    .child(
                                        Label::new(action.key)
                                            .size(LabelSize::XSmall)
                                            .color(Color::Muted),
                                    ),
                            )
                    })),
            )
    }
}

impl Focusable for Launcher {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Launcher {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.pending_close {
            self.pending_close = false;
            *LAUNCHER_WINDOW.lock().unwrap() = None;
            window.remove_window();
        }

        if !self.focus_handle.is_focused(window) {
            self.focus_handle.focus(window, cx);
        }

        let colors = cx.theme().colors();

        div()
            .id("launcher")
            .track_focus(&self.focus_handle)
            .key_context("Launcher")
            .on_action(cx.listener(|_, _: &Escape, window, _cx| {
                *LAUNCHER_WINDOW.lock().unwrap() = None;
                window.remove_window();
            }))
            .on_action(cx.listener(|this, _: &Enter, _window, cx| {
                this.on_confirm(cx);
            }))
            .on_key_down(
                cx.listener(|this, event: &gpui::KeyDownEvent, _window, cx| {
                    match event.keystroke.key.as_str() {
                        "up" => this.on_move_up(cx),
                        "down" => this.on_move_down(cx),
                        "pageup" => this.on_page_up(cx),
                        "pagedown" => this.on_page_down(cx),
                        "backspace" => {
                            this.on_backspace(cx);
                            return;
                        }
                        _ => {
                            if let Some(key_char) = &event.keystroke.key_char {
                                this.on_char(key_char, cx);
                                return;
                            } else if event.keystroke.key.len() == 1
                                && !event.keystroke.modifiers.control
                                && !event.keystroke.modifiers.alt
                            {
                                this.on_char(&event.keystroke.key, cx);
                                return;
                            }
                            return;
                        }
                    }
                    cx.notify();
                }),
            )
            .size_full()
            .bg(colors.background)
            .border_1()
            .border_color(colors.border)
            .rounded(px(12.))
            .text_color(colors.text)
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(self.render_search_bar(cx))
            .child(div().w_full().h(px(1.)).bg(colors.border))
            .child(self.render_view_content(cx))
            .child(div().w_full().h(px(1.)).bg(colors.border))
            .child(self.render_footer_bar(cx))
    }
}

// ── Window management ───────────────────────────────────────────────

static LAUNCHER_WINDOW: std::sync::Mutex<Option<WindowHandle<Launcher>>> =
    std::sync::Mutex::new(None);

pub fn register_keybindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Escape, Some("Launcher")),
        KeyBinding::new("enter", Enter, Some("Launcher")),
    ]);
}

pub fn toggle(services: Services, cx: &mut App) {
    toggle_with_input(services, None, cx);
}

pub fn toggle_from_ipc(services: Services, input: Option<String>, cx: &mut App) {
    let mut guard = LAUNCHER_WINDOW.lock().unwrap();
    if let Some(handle) = guard.take() {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    } else {
        drop(guard);
        toggle_with_input(services, input, cx);
    }
}

pub fn toggle_with_input(services: Services, input: Option<String>, cx: &mut App) {
    let mut guard = LAUNCHER_WINDOW.lock().unwrap();

    if let Some(handle) = guard.take() {
        if let Some(input_text) = input {
            let update_result = handle.update(cx, |launcher, _, cx| {
                let q = input_text.clone();
                launcher.set_query(q, cx);
            });
            if update_result.is_ok() {
                *guard = Some(handle);
                return;
            }
        }
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    } else {
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
            move |_, cx| cx.new(|cx| Launcher::new(services.clone(), input.clone(), cx)),
        ) {
            *guard = Some(handle);
        }
    }
}

pub fn open(services: Services, cx: &mut App) {
    toggle(services, cx);
}

pub fn open_with_input(services: Services, input: Option<String>, cx: &mut App) {
    let guard = LAUNCHER_WINDOW.lock().unwrap();
    if let Some(handle) = &*guard {
        if let Some(input_text) = input {
            let _ = handle.update(cx, |launcher, _, cx| {
                let q = input_text.clone();
                launcher.set_query(q, cx);
            });
        }
    } else {
        drop(guard);
        toggle_with_input(services, input, cx);
    }
}

mod view;
mod views;

use crate::services::Services;
use gpui::{
    App, AppContext, Bounds, Context, FocusHandle, Focusable, KeyBinding, Point, ScrollHandle,
    Size, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions, actions, div, layer_shell::*, prelude::*, px, rgba,
};
use view::{InputResult, LIST_ITEM_HEIGHT, LauncherView, ViewContext, ViewInput};
use views::{HelpView, all_views};

actions!(launcher, [Escape, Enter]);

const LAUNCHER_WIDTH: f32 = 600.0;
const LAUNCHER_HEIGHT: f32 = 450.0;

/// Launcher configuration.
#[derive(Clone)]
pub struct LauncherConfig {
    /// The character used to prefix commands (default: ';').
    pub prefix_char: char,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        LauncherConfig { prefix_char: ';' }
    }
}

/// Visible height of the content area (approximate)
const VISIBLE_HEIGHT: f32 = 350.0;
/// Number of items to jump when using Page Up/Down
const ITEMS_PER_PAGE: usize = 7;

pub struct Launcher {
    services: Services,
    config: LauncherConfig,
    search_query: String,
    selected_index: usize,
    item_count: usize,
    focus_handle: FocusHandle,
    scroll_handle: ScrollHandle,
    views: Vec<Box<dyn LauncherView>>,
    help_view: HelpView,
}

impl Launcher {
    pub fn new(services: Services, config: LauncherConfig, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Observe all services for updates
        cx.observe(&services.applications, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.compositor, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.audio, |_, _, cx| cx.notify()).detach();
        cx.observe(&services.network, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.upower, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.sysinfo, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.bluetooth, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.brightness, |_, _, cx| cx.notify())
            .detach();

        let views = all_views();
        let help_view = HelpView::new(config.prefix_char, &views);
        let scroll_handle = ScrollHandle::new();

        Launcher {
            services,
            config,
            search_query: String::new(),
            selected_index: 0,
            item_count: 0,
            focus_handle,
            scroll_handle,
            views,
            help_view,
        }
    }

    /// Reset scroll to top
    fn reset_scroll(&self) {
        self.scroll_handle.set_offset(gpui::point(px(0.), px(0.)));
    }

    /// Scroll to keep the selected item visible
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

    /// Parse the search query to extract prefix and search term.
    fn parse_query(&self) -> (Option<&str>, &str) {
        let query = self.search_query.trim();

        if query.starts_with(self.config.prefix_char) {
            let rest = &query[self.config.prefix_char.len_utf8()..];

            if let Some(space_idx) = rest.find(' ') {
                let prefix = &rest[..space_idx];
                let search = rest[space_idx..].trim();
                (Some(prefix), search)
            } else {
                (Some(rest), "")
            }
        } else {
            (None, query)
        }
    }

    /// Find the view matching the given prefix.
    fn find_view(&self, prefix: &str) -> Option<&dyn LauncherView> {
        self.views
            .iter()
            .find(|v| v.prefix() == prefix)
            .map(|v| v.as_ref())
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
        let (prefix, _) = self.parse_query();

        match prefix {
            Some(p) => {
                if let Some(view) = self.find_view(p) {
                    return view;
                }
                &self.help_view
            }
            None => self.default_view().unwrap_or(&self.help_view),
        }
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
            prefix_char: self.config.prefix_char,
        }
    }

    fn handle_input(&mut self, input: ViewInput, cx: &mut App) -> bool {
        let vx = self.view_context();
        let view = self.current_view();

        match view.handle_input(&input, &vx, cx) {
            InputResult::Handled { query, close } => {
                // Update search query based on prefix
                let (prefix, _) = self.parse_query();
                if let Some(p) = prefix {
                    self.search_query = format!("{}{} {}", self.config.prefix_char, p, query);
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
        let vx = self.view_context();
        let (prefix, _) = self.parse_query();

        // Check if we're in help view and selected a command
        if prefix.is_some() && self.find_view(prefix.unwrap()).is_none() {
            // In help view, switch to selected view
            let entries: Vec<_> = self.views.iter().collect();
            if let Some(view) = entries.get(self.selected_index) {
                self.search_query = format!("{}{} ", self.config.prefix_char, view.prefix());
                self.selected_index = 0;
                return false;
            }
        }

        let view = self.current_view();
        view.on_select(self.selected_index, &vx, cx)
    }

    fn placeholder(&self) -> String {
        format!("Search or type {}command...", self.config.prefix_char)
    }
}

impl Focusable for Launcher {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Launcher {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search_query.clone();
        let view_name = self.current_view_name().to_string();
        let placeholder = self.placeholder();
        let is_empty = self.search_query.is_empty();

        // Render current view
        let vx = self.view_context();
        let (view_content, item_count) = self.current_view().render(&vx, cx);
        self.item_count = item_count;

        // Clamp selected index
        if self.selected_index >= item_count && item_count > 0 {
            self.selected_index = item_count - 1;
        }

        div()
            .id("launcher")
            .track_focus(&self.focus_handle)
            .key_context("Launcher")
            .on_action(cx.listener(|_, _: &Escape, window, _cx| {
                window.remove_window();
            }))
            .on_action(cx.listener(|this, _: &Enter, window, cx| {
                if this.handle_input(ViewInput::Enter, cx) {
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
                        window.remove_window();
                    }
                    cx.notify();
                }),
            )
            .size_full()
            .bg(rgba(0x1a1a1aee))
            .border_1()
            .border_color(rgba(0x333333ff))
            .rounded(px(12.))
            .p(px(16.))
            .text_color(rgba(0xffffffff))
            .flex()
            .flex_col()
            .gap(px(12.))
            // Search input with view indicator
            .child(
                div()
                    .w_full()
                    .px(px(12.))
                    .py(px(10.))
                    .bg(rgba(0x333333ff))
                    .rounded(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            // Search text
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(14.))
                                    .child(if query.is_empty() { placeholder } else { query })
                                    .text_color(if is_empty {
                                        rgba(0x888888ff)
                                    } else {
                                        rgba(0xffffffff)
                                    }),
                            )
                            // View badge (right side)
                            .child(
                                div()
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .bg(rgba(0x3b82f6ff))
                                    .text_size(px(10.))
                                    .child(view_name),
                            ),
                    ),
            )
            // View content with scroll support
            .child(
                div()
                    .id("view-content")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .child(view_content),
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
    toggle_with_config(services, LauncherConfig::default(), cx);
}

/// Toggle the launcher window with custom configuration.
pub fn toggle_with_config(services: Services, config: LauncherConfig, cx: &mut App) {
    let mut guard = LAUNCHER_WINDOW.lock().unwrap();

    if let Some(handle) = guard.take() {
        let _ = handle.update(cx, |_, window, _| {
            window.remove_window();
        });
    } else {
        if let Ok(handle) = cx.open_window(
            WindowOptions {
                titlebar: None,
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::new(px(0.), px(0.)),
                    size: Size::new(px(LAUNCHER_WIDTH), px(LAUNCHER_HEIGHT)),
                })),
                app_id: Some("gpui-launcher".to_string()),
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
            move |_, cx| cx.new(|cx| Launcher::new(services.clone(), config.clone(), cx)),
        ) {
            *guard = Some(handle);
        }
    }
}

/// Open the launcher.
pub fn open(services: Services, cx: &mut App) {
    toggle(services, cx);
}

mod view;
mod views;

use crate::services::Services;
use gpui::{
    App, AppContext, Bounds, Context, FocusHandle, Focusable, KeyBinding, MouseButton, Point, Size,
    Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
    actions, div, layer_shell::*, prelude::*, px, rgba,
};
use view::{LauncherView, ViewAction, ViewItem, execute_action};
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

pub struct Launcher {
    services: Services,
    config: LauncherConfig,
    search_query: String,
    selected_index: usize,
    focus_handle: FocusHandle,
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

        let views = all_views();
        let help_view = HelpView::new(config.prefix_char, &views);

        Launcher {
            services,
            config,
            search_query: String::new(),
            selected_index: 0,
            focus_handle,
            views,
            help_view,
        }
    }

    /// Parse the search query to extract prefix and search term.
    fn parse_query(&self) -> (Option<&str>, &str) {
        let query = self.search_query.trim();

        if query.starts_with(self.config.prefix_char) {
            // Has prefix char, extract the command
            let rest = &query[self.config.prefix_char.len_utf8()..];

            // Find the first space to separate prefix from search
            if let Some(space_idx) = rest.find(' ') {
                let prefix = &rest[..space_idx];
                let search = rest[space_idx..].trim();
                (Some(prefix), search)
            } else {
                // No space yet, the whole thing is the prefix (partial)
                (Some(rest), "")
            }
        } else {
            // No prefix, use default view
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

    /// Get items for the current query.
    fn get_items(&self, cx: &App) -> Vec<ViewItem> {
        let (prefix, search) = self.parse_query();

        match prefix {
            Some(p) => {
                // Check if prefix matches a view exactly
                if let Some(view) = self.find_view(p) {
                    return view.items(search, &self.services, cx);
                }

                // Check if prefix is partial match (user still typing)
                let partial_matches: Vec<_> = self
                    .views
                    .iter()
                    .filter(|v| v.prefix().starts_with(p))
                    .collect();

                if partial_matches.len() == 1 && partial_matches[0].prefix() == p {
                    // Exact match
                    return partial_matches[0].items(search, &self.services, cx);
                }

                // Show help with matching prefixes
                self.help_view.items(p, &self.services, cx)
            }
            None => {
                // Use default view (apps)
                if let Some(view) = self.default_view() {
                    view.items(search, &self.services, cx)
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// Get the current view name for display.
    fn current_view_name(&self) -> &str {
        let (prefix, _) = self.parse_query();

        match prefix {
            Some(p) => {
                if let Some(view) = self.find_view(p) {
                    return view.name();
                }
                "Help"
            }
            None => self.default_view().map(|v| v.name()).unwrap_or("Search"),
        }
    }

    fn execute_selected(&mut self, cx: &mut Context<Self>, window: &mut Window) {
        let items = self.get_items(cx);
        if let Some(item) = items.get(self.selected_index) {
            match &item.action {
                ViewAction::SwitchView(prefix) => {
                    // Switch to the new view
                    self.search_query = format!("{}{} ", self.config.prefix_char, prefix);
                    self.selected_index = 0;
                    cx.notify();
                }
                action => {
                    execute_action(action, &self.services, cx);
                    window.remove_window();
                }
            }
        }
    }

    fn execute_item(&mut self, item: &ViewItem, cx: &mut Context<Self>, window: &mut Window) {
        match &item.action {
            ViewAction::SwitchView(prefix) => {
                self.search_query = format!("{}{} ", self.config.prefix_char, prefix);
                self.selected_index = 0;
                cx.notify();
            }
            action => {
                execute_action(action, &self.services, cx);
                window.remove_window();
            }
        }
    }

    fn move_selection(&mut self, delta: i32, cx: &App) {
        let items = self.get_items(cx);
        if items.is_empty() {
            self.selected_index = 0;
            return;
        }

        let len = items.len() as i32;
        let new_index = (self.selected_index as i32 + delta).rem_euclid(len);
        self.selected_index = new_index as usize;
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
        let items = self.get_items(cx);
        let selected_index = self.selected_index;
        let query = self.search_query.clone();
        let view_name = self.current_view_name().to_string();
        let placeholder = self.placeholder();

        div()
            .id("launcher")
            .track_focus(&self.focus_handle)
            .key_context("Launcher")
            .on_action(cx.listener(|_, _: &Escape, window, _cx| {
                window.remove_window();
            }))
            .on_action(cx.listener(|this, _: &Enter, window, cx| {
                this.execute_selected(cx, window);
            }))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                match event.keystroke.key.as_str() {
                    "up" => {
                        this.move_selection(-1, cx);
                        cx.notify();
                    }
                    "down" => {
                        this.move_selection(1, cx);
                        cx.notify();
                    }
                    "backspace" => {
                        this.search_query.pop();
                        this.selected_index = 0;
                        cx.notify();
                    }
                    _ => {
                        if let Some(key_char) = &event.keystroke.key_char {
                            this.search_query.push_str(key_char);
                            this.selected_index = 0;
                            cx.notify();
                        } else if event.keystroke.key.len() == 1
                            && !event.keystroke.modifiers.control
                            && !event.keystroke.modifiers.alt
                        {
                            this.search_query.push_str(&event.keystroke.key);
                            this.selected_index = 0;
                            cx.notify();
                        }
                    }
                }
            }))
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
                                    .text_color(if self.search_query.is_empty() {
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
            // Items list
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .children(items.into_iter().enumerate().map(|(i, item)| {
                        let is_selected = i == selected_index;
                        let item_for_click = item.clone();

                        div()
                            .id(item.id.clone())
                            .w_full()
                            .px(px(12.))
                            .py(px(8.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .when(is_selected, |el| el.bg(rgba(0x3b82f6ff)))
                            .when(!is_selected, |el| el.hover(|s| s.bg(rgba(0x333333ff))))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, window, cx| {
                                    this.execute_item(&item_for_click, cx, window);
                                }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.))
                                    // Icon
                                    .child(
                                        div()
                                            .w(px(32.))
                                            .h(px(32.))
                                            .rounded(px(6.))
                                            .bg(rgba(0x444444ff))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_size(px(16.))
                                            .child(item.icon.clone()),
                                    )
                                    // Title and subtitle
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.))
                                            .child(
                                                div()
                                                    .text_size(px(14.))
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .child(item.title.clone()),
                                            )
                                            .when_some(item.subtitle.clone(), |el, subtitle| {
                                                el.child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .text_color(rgba(0x888888ff))
                                                        .child(subtitle),
                                                )
                                            }),
                                    ),
                            )
                    })),
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

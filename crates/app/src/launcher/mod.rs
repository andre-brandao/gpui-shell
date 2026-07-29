//! Launcher module providing an application launcher overlay.
//!
//! The launcher provides a keyboard-driven interface for:
//! - Searching and launching applications (@ prefix, or default)
//! - Running shell commands ($ prefix)
//! - Web search with multiple providers (! prefix with shebangs)
//! - Switching workspaces (;ws prefix)
//! - Viewing help and available commands (? prefix)

pub mod config;
pub mod modules;
pub mod view;

use gpui::{
    App, Bounds, Context, FocusHandle, Focusable, Point, ScrollHandle, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, div,
    layer_shell::*, prelude::*, px,
};
use modules::{HelpView, all_views};
use ui::patterns::LauncherFrame;
use ui::{ActiveTheme, InputBuffer, VariableList, VariableListScrollHandle, render_input_line};
use view::{InputResult, LauncherView, ViewContext, ViewInput, is_prefix};

use crate::config::Config;
use crate::keybinds::{
    Backspace, Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp, DeleteWordBack,
    PageDown, PageUp, SelectAll, SelectLeft, SelectRight, SelectWordLeft, SelectWordRight,
    WordLeft, WordRight,
};
use crate::state::{AppState, watch};

/// Number of items to jump when using Page Up/Down.
const ITEMS_PER_PAGE: usize = 7;

/// Which view owns the current query.
///
/// Resolved as a slot rather than a `&dyn LauncherView` so the launcher can
/// still take `&mut` on the view to refresh its matches - a borrow that a
/// returned reference into `self` would rule out.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewSlot {
    Help,
    Index(usize),
}

/// The main launcher struct.
pub struct Launcher {
    input: InputBuffer,
    selected_index: usize,
    focus_handle: FocusHandle,
    /// Scroll state for content views (shell, web), which render one
    /// element rather than a row list.
    scroll_handle: ScrollHandle,
    /// Scroll and row-measurement state for list views. Persisted across
    /// frames so `gpui::list` can cache row heights.
    list_state: VariableListScrollHandle,
    /// Item count `list_state` was last sized for.
    list_count: usize,
    /// Set when the query or view changes, so the next frame rewinds the
    /// list to the top.
    needs_list_reset: bool,
    views: Vec<Box<dyn LauncherView>>,
    help_view: HelpView,
}

impl Launcher {
    /// Create a new launcher with optional initial input.
    pub fn new(initial_input: Option<String>, cx: &mut Context<Self>) -> Self {
        let compositor = AppState::compositor(cx).clone();
        let sysinfo = AppState::sysinfo(cx).clone();
        let upower = AppState::upower(cx).clone();
        let focus_handle = cx.focus_handle();
        let scroll_handle = ScrollHandle::new();

        // Subscribe to service updates for reactive rendering
        watch(cx, compositor.subscribe(), |_, _, cx| {
            cx.notify();
        });

        watch(cx, sysinfo.subscribe(), |_, _, cx| {
            cx.notify();
        });

        watch(cx, upower.subscribe(), |_, _, cx| {
            cx.notify();
        });

        let launcher_config = Config::global(cx).launcher.clone();
        let views = all_views(&launcher_config);
        let help_view = HelpView::new(&launcher_config.modules.help, &views);

        Launcher {
            input: InputBuffer::new(initial_input.unwrap_or_default()),
            selected_index: 0,
            focus_handle,
            scroll_handle,
            // ponytail: measure_all builds every row once per query change
            // so keyboard selection can scroll anywhere. Fine at ~50 apps;
            // switch to uniform rows + VirtualList if the list gets large.
            list_state: VariableListScrollHandle::new(0).measure_all(),
            list_count: 0,
            needs_list_reset: true,
            views,
            help_view,
        }
    }

    /// Set the search query (used for IPC input).
    pub fn set_input(&mut self, input: String) {
        self.input.set_text(input);
        self.selected_index = 0;
        self.reset_scroll();
    }

    /// Reset scroll to top.
    fn reset_scroll(&mut self) {
        self.scroll_handle.set_offset(gpui::point(px(0.), px(0.)));
        self.needs_list_reset = true;
    }

    /// Ensure the selected item is scrolled into view.
    fn scroll_to_selected(&self) {
        self.list_state.scroll_to_item(self.selected_index);
    }

    /// Parse the search query to find which view should handle it.
    ///
    /// Returns the slot of the matched view plus that view's search term.
    /// The term is owned so callers can hold it across a `&mut self`
    /// borrow of the view itself.
    fn parse_query(&self) -> (ViewSlot, String) {
        let query = self.input.text().trim();

        if query.is_empty() {
            return (self.default_slot(), String::new());
        }

        // Check if query starts with any view's prefix
        // We need to find the longest matching prefix first
        let mut best_match: Option<(ViewSlot, usize)> = None;

        for (ix, view) in self.views.iter().enumerate() {
            let prefix = view.prefix();
            if is_prefix(query, prefix) {
                // Check if this is a better (longer) match
                if best_match.is_none() || prefix.len() > best_match.unwrap().1 {
                    best_match = Some((ViewSlot::Index(ix), prefix.len()));
                }
            }
        }

        // Also check help view
        if is_prefix(query, self.help_view.prefix()) {
            let prefix_len = self.help_view.prefix().len();
            if best_match.is_none() || prefix_len > best_match.unwrap().1 {
                best_match = Some((ViewSlot::Help, prefix_len));
            }
        }

        if let Some((slot, prefix_len)) = best_match {
            return (slot, query[prefix_len..].trim_start().to_string());
        }

        // Check if starts with a known prefix character but no matching prefix
        // In this case, show help view
        if let Some(first_char) = query.chars().next()
            && (self
                .views
                .iter()
                .any(|v| v.prefix().starts_with(first_char))
                || self.help_view.prefix().starts_with(first_char))
        {
            // Unknown special prefix - show help
            return (ViewSlot::Help, query.to_string());
        }

        // No prefix, use default view with full query as search
        (self.default_slot(), query.to_string())
    }

    /// Slot of the default view, falling back to help when none is marked.
    fn default_slot(&self) -> ViewSlot {
        self.views
            .iter()
            .position(|v| v.is_default())
            .map_or(ViewSlot::Help, ViewSlot::Index)
    }

    fn view(&self, slot: ViewSlot) -> &dyn LauncherView {
        match slot {
            ViewSlot::Help => &self.help_view,
            ViewSlot::Index(ix) => self.views[ix].as_ref(),
        }
    }

    fn view_mut(&mut self, slot: ViewSlot) -> &mut dyn LauncherView {
        match slot {
            ViewSlot::Help => &mut self.help_view,
            ViewSlot::Index(ix) => self.views[ix].as_mut(),
        }
    }

    /// Get the current active view.
    fn current_view(&self) -> &dyn LauncherView {
        self.view(self.parse_query().0)
    }

    /// Get the current view name for display.
    fn current_view_name(&self) -> &str {
        self.current_view().name()
    }

    /// Refresh the current view's matches and return its slot, search term
    /// and match count.
    ///
    /// Every read of `match_count` / `render_item` downstream reuses this
    /// one filter pass.
    fn refresh_matches(&mut self, cx: &App) -> (ViewSlot, String, usize) {
        let (slot, query) = self.parse_query();
        let vx = ViewContext {
            query: &query,
            selected_index: self.selected_index,
        };
        let view = self.view_mut(slot);
        view.update_matches(&vx, cx);
        let count = view.match_count(&vx, cx);
        (slot, query, count)
    }

    fn handle_input(&mut self, input: ViewInput, cx: &mut App) -> bool {
        let (slot, query, item_count) = self.refresh_matches(cx);
        let vx = ViewContext {
            query: &query,
            selected_index: self.selected_index,
        };
        let view = self.view(slot);

        match view.handle_input(&input, &vx, cx) {
            InputResult::Handled { query, close } => {
                // Update search query based on current view prefix
                let prefix = self.current_view().prefix().to_string();
                if query.is_empty() {
                    self.input.set_text(prefix);
                } else {
                    self.input.set_text(format!("{} {}", prefix, query));
                }
                self.selected_index = 0;
                self.reset_scroll();
                close
            }
            InputResult::Unhandled => {
                // Default handling
                match input {
                    ViewInput::Char(c) => {
                        self.input.insert_str(&c);
                        self.selected_index = 0;
                        self.reset_scroll();
                    }
                    ViewInput::Backspace => {
                        self.input.backspace();
                        self.selected_index = 0;
                        self.reset_scroll();
                    }
                    ViewInput::Up => {
                        if item_count > 0 {
                            self.selected_index = if self.selected_index == 0 {
                                item_count - 1
                            } else {
                                self.selected_index - 1
                            };
                            self.scroll_to_selected();
                        }
                    }
                    ViewInput::Down => {
                        if item_count > 0 {
                            self.selected_index = (self.selected_index + 1) % item_count;
                            self.scroll_to_selected();
                        }
                    }
                    ViewInput::PageUp => {
                        if item_count > 0 {
                            self.selected_index =
                                self.selected_index.saturating_sub(ITEMS_PER_PAGE);
                            self.scroll_to_selected();
                        }
                    }
                    ViewInput::PageDown => {
                        if item_count > 0 {
                            self.selected_index = (self.selected_index + ITEMS_PER_PAGE)
                                .min(item_count.saturating_sub(1));
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

    fn delete_word_back(&mut self) {
        self.input.delete_word_back();
        self.selected_index = 0;
        self.reset_scroll();
    }

    fn execute_selected(&mut self, cx: &mut App) -> bool {
        let (slot, query) = self.parse_query();

        // Check if we're in help view and selected a command
        if slot == ViewSlot::Help
            && let Some(prefix) = self.help_view.selected_prefix(self.selected_index)
        {
            let target = self
                .views
                .iter()
                .find(|v| v.prefix() == prefix)
                .map(|v| v.prefix().to_string());
            if let Some(prefix) = target {
                self.input.set_text(format!("{} ", prefix));
                self.selected_index = 0;
                self.reset_scroll();
                return false;
            }
        }

        let vx = ViewContext {
            query: &query,
            selected_index: self.selected_index,
        };
        self.view(slot).on_select(self.selected_index, &vx, cx)
    }

    fn placeholder(&self) -> String {
        "Search apps or type @, $, !, ? for commands...".to_string()
    }

    fn prefix_hint_label(name: &str) -> String {
        match name {
            "Applications" => "apps".to_string(),
            "Web Search" => "web".to_string(),
            _ => name.to_lowercase(),
        }
    }

    fn format_prefix_hint(prefix: &str, name: &str) -> Option<String> {
        let prefix = prefix.trim();
        if prefix.is_empty() {
            return None;
        }

        let label = Self::prefix_hint_label(name);
        let spacer = if prefix.chars().count() > 1 { " " } else { "" };
        Some(format!("{prefix}{spacer}{label}"))
    }

    fn footer_prefix_hints(&self) -> String {
        let mut hints: Vec<String> = self
            .views
            .iter()
            .filter_map(|view| Self::format_prefix_hint(view.prefix(), view.name()))
            .collect();

        if let Some(help_hint) =
            Self::format_prefix_hint(self.help_view.prefix(), self.help_view.name())
        {
            hints.push(help_hint);
        }

        hints.join(" · ")
    }
}

impl Focusable for Launcher {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Launcher {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(cx.theme().font_size);

        // Always keep the launcher focused
        if !self.focus_handle.is_focused(window) {
            self.focus_handle.focus(window, cx);
        }

        let view_name = self.current_view_name().to_string();
        let prefix_hints = self.footer_prefix_hints();
        let placeholder = self.placeholder();

        // One filter pass for the whole frame. Everything below reads its
        // result instead of re-filtering per row.
        let (slot, query, item_count) = self.refresh_matches(cx);
        if self.selected_index >= item_count && item_count > 0 {
            self.selected_index = item_count - 1;
        }

        // `gpui::list` needs to be told when the row set changes; it caches
        // measurements per index otherwise.
        if self.needs_list_reset || item_count != self.list_count {
            self.list_state.reset(item_count);
            self.list_count = item_count;
            self.needs_list_reset = false;
        }

        let vx = ViewContext {
            query: &query,
            selected_index: self.selected_index,
        };
        let current_view = self.view(slot);
        let footer_bar = current_view.render_footer_bar(&vx, cx);
        let header = current_view.render_header(&vx, cx);
        let footer = current_view.render_footer(&vx, cx);
        let content = current_view.render_content(&vx, cx);

        div()
            .id("launcher")
            .track_focus(&self.focus_handle)
            .key_context("Launcher")
            .on_action(cx.listener(|this, _: &Cancel, window, cx| {
                if this.input.is_empty() {
                    // Clear the static handle before removing window
                    *LAUNCHER_WINDOW.lock().unwrap() = None;
                    window.remove_window();
                } else {
                    // First Esc clears input; second Esc closes.
                    this.input.clear();
                    this.selected_index = 0;
                    this.reset_scroll();
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &Confirm, window, cx| {
                if this.handle_input(ViewInput::Enter, cx) {
                    // Clear the static handle before removing window
                    *LAUNCHER_WINDOW.lock().unwrap() = None;
                    window.remove_window();
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CursorUp, _window, cx| {
                this.handle_input(ViewInput::Up, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CursorDown, _window, cx| {
                this.handle_input(ViewInput::Down, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &PageUp, _window, cx| {
                this.handle_input(ViewInput::PageUp, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &PageDown, _window, cx| {
                this.handle_input(ViewInput::PageDown, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Backspace, _window, cx| {
                this.handle_input(ViewInput::Backspace, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &DeleteWordBack, _window, cx| {
                this.delete_word_back();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CursorLeft, _window, cx| {
                this.input.move_left(false);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CursorRight, _window, cx| {
                this.input.move_right(false);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &WordLeft, _window, cx| {
                this.input.move_word_left(false);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &WordRight, _window, cx| {
                this.input.move_word_right(false);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectWordLeft, _window, cx| {
                this.input.move_word_left(true);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectWordRight, _window, cx| {
                this.input.move_word_right(true);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectLeft, _window, cx| {
                this.input.move_left(true);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectRight, _window, cx| {
                this.input.move_right(true);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectAll, _window, cx| {
                this.input.select_all();
                cx.notify();
            }))
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, _window, cx| {
                    if event.keystroke.modifiers.control || event.keystroke.modifiers.alt {
                        return;
                    }

                    let input_str = event.keystroke.key_char.as_ref().cloned().or_else(|| {
                        let key = event.keystroke.key.as_str();
                        if key.chars().count() == 1 {
                            Some(key.to_string())
                        } else {
                            None
                        }
                    });

                    let Some(s) = input_str else {
                        return;
                    };
                    if s.chars().any(|c| c.is_control()) {
                        return;
                    }

                    this.handle_input(ViewInput::Char(s), cx);
                    cx.notify();
                }),
            )
            .size_full()
            .child(
                // Chrome lives in `ui::patterns`; the body is ours. List
                // views go through `VariableList`, which only builds the
                // rows gpui actually needs - layout cost in gpui is per
                // element and independent of visibility, so building all N
                // rows to show ~8 is the whole cost. Header and footer sit
                // outside the list because it owns its own scroll.
                LauncherFrame::new(render_input_line(&self.input, &placeholder, cx))
                    .badge(view_name)
                    .hints(prefix_hints)
                    .actions(footer_bar)
                    .children(header)
                    .child(if let Some(content) = content {
                        div()
                            .id("view-scroll")
                            .flex_1()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll_handle)
                            .child(content)
                            .into_any_element()
                    } else {
                        // `gpui::list` sizes itself from its own style, not
                        // from its rows, so it needs a parent with a
                        // definite height to measure against - `flex_1` on
                        // the list itself resolves to zero and it renders
                        // no rows at all.
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(
                                VariableList::new(
                                    self.list_state.clone(),
                                    cx.processor(|this, ix: usize, _window, cx| {
                                        let (slot, query) = this.parse_query();
                                        let vx = ViewContext {
                                            query: &query,
                                            selected_index: this.selected_index,
                                        };
                                        this.view(slot).render_item(
                                            ix,
                                            ix == this.selected_index,
                                            &vx,
                                            cx,
                                        )
                                    }),
                                )
                                .h_full(),
                            )
                            .into_any_element()
                    })
                    .children(footer),
            )
    }
}

/// Global state to track the launcher window.
static LAUNCHER_WINDOW: std::sync::Mutex<Option<WindowHandle<Launcher>>> =
    std::sync::Mutex::new(None);

pub fn init(_cx: &mut App) {}

/// Toggle the launcher window with optional prefilled input.
///
/// Behavior:
/// - If launcher is closed: opens it (with optional input).
/// - If launcher is open and `input` is `Some`: updates the input.
/// - If launcher is open and `input` is `None`: closes it.
pub fn toggle(input: Option<String>, cx: &mut App) {
    let start = std::time::Instant::now();
    tracing::debug!("launcher::toggle: start");

    let mut guard = LAUNCHER_WINDOW.lock().unwrap();
    tracing::debug!("launcher::toggle: acquired lock {:?}", start.elapsed());

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
        tracing::debug!("launcher::toggle: closed window {:?}", start.elapsed());
    } else {
        tracing::debug!("launcher::toggle: opening new window {:?}", start.elapsed());
        let cfg = Config::global(cx).launcher.clone();
        if let Ok(handle) = cx.open_window(
            WindowOptions {
                titlebar: None,
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::new(px(0.), px(0.)),
                    size: Size::new(px(cfg.width), px(cfg.height)),
                })),
                app_id: Some("gpuishell-launcher".to_string()),
                window_background: WindowBackgroundAppearance::Transparent,
                kind: WindowKind::LayerShell(LayerShellOptions {
                    namespace: "launcher".to_string(),
                    layer: Layer::Overlay,
                    anchor: Anchor::TOP,
                    exclusive_zone: None,
                    margin: Some((
                        px(cfg.margin_top),
                        px(cfg.margin_right),
                        px(cfg.margin_bottom),
                        px(cfg.margin_left),
                    )),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    ..Default::default()
                }),
                focus: true,
                ..Default::default()
            },
            move |_, cx| {
                cx.new(|cx| {
                    let new_start = std::time::Instant::now();
                    let launcher = Launcher::new(input.clone(), cx);
                    tracing::debug!(
                        "launcher::toggle: Launcher::new took {:?}",
                        new_start.elapsed()
                    );
                    launcher
                })
            },
        ) {
            *guard = Some(handle);
            tracing::debug!("launcher::toggle: window opened {:?}", start.elapsed());
        }
    }
    tracing::debug!("launcher::toggle: done {:?}", start.elapsed());
}

//! Keyboard-driven launcher. The launcher owns the window, the input, the
//! selection and the rows; views (`views/`) supply items or a custom body.

pub mod config;
pub mod item;
mod matcher;
mod route;
pub mod row;
pub mod view;
pub mod views;

use std::ops::Range;

use gpui::{
    App, Bounds, ClickEvent, Context, EntityId, FocusHandle, Focusable, Point, ScrollHandle, Size,
    Subscription, Task, Window, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, div, layer_shell::*, prelude::*, px,
};
use ui::patterns::{LauncherFrame, footer_hints};
use ui::{
    ActiveTheme, Color, InputBuffer, Label, LabelCommon, ScrollStrategy, Spacing, TextSize,
    ThumbnailCache, VirtualList, VirtualListScrollHandle, render_input_line,
};

use self::item::Item;
use self::matcher::{Ranked, Ranker};
use self::route::Route;
use self::view::{
    Body, Hint, Items, Matching, NavKey, Outcome, ViewDescriptor, ViewHandle, ViewMeta,
};
use crate::config::Config;
use crate::keybinds::{
    Backspace, Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp, DeleteWordBack,
    PageDown, PageUp, SecondaryConfirm, SelectAll, SelectLeft, SelectRight, SelectWordLeft,
    SelectWordRight, WordLeft, WordRight,
};

/// Rows to jump on Page Up/Down.
const ROWS_PER_PAGE: usize = 7;

pub struct Launcher {
    input: InputBuffer,
    focus_handle: FocusHandle,
    /// Scroll state for custom bodies.
    body_scroll: ScrollHandle,
    list_scroll: VirtualListScrollHandle,
    registry: Vec<ViewDescriptor>,
    metas: Vec<ViewMeta>,
    /// Built on first route, so views nobody opens never start their services.
    views: Vec<Option<Box<dyn ViewHandle>>>,
    subscriptions: Vec<Subscription>,
    /// Input text `route` was computed from, so it is parsed once per edit.
    routed_input: String,
    route: Route,
    items: Vec<Item>,
    matches: Vec<Ranked>,
    selected: usize,
    /// Columns of the body drawn last, for grid navigation.
    columns: usize,
    ranker: Ranker,
    /// Drops item replies that a newer query overtook.
    generation: u64,
    pending: Option<Task<()>>,
    owner: EntityId,
}

impl Launcher {
    pub fn new(initial_input: Option<String>, cx: &mut Context<Self>) -> Self {
        let config = Config::global(cx).launcher.clone();
        let registry = views::registry(&config.views);
        let metas: Vec<ViewMeta> = registry.iter().map(|d| d.meta.clone()).collect();
        let input = InputBuffer::new(initial_input.unwrap_or_default());
        let routed_input = input.text().to_string();
        let route = route::route(&routed_input, &metas);

        let mut launcher = Launcher {
            input,
            focus_handle: cx.focus_handle(),
            body_scroll: ScrollHandle::new(),
            list_scroll: VirtualListScrollHandle::new(),
            views: registry.iter().map(|_| None).collect(),
            registry,
            metas,
            subscriptions: Vec::new(),
            routed_input,
            route,
            items: Vec::new(),
            matches: Vec::new(),
            selected: 0,
            columns: 1,
            ranker: Ranker::default(),
            generation: 0,
            pending: None,
            owner: cx.entity_id(),
        };
        launcher.requery(false, cx);
        launcher
    }

    /// Replace the input (IPC `--input`).
    pub fn set_input(&mut self, input: String, cx: &mut Context<Self>) {
        self.input.set_text(input);
        self.sync(cx);
    }

    /// Re-route after an edit. Cheap when the text did not change.
    fn sync(&mut self, cx: &mut Context<Self>) {
        if self.input.text() == self.routed_input {
            return;
        }
        self.routed_input = self.input.text().to_string();
        cx.notify();
        let route = route::route(&self.routed_input, &self.metas);
        if route == self.route {
            return;
        }
        if route.view != self.route.view {
            self.items.clear();
            self.matches.clear();
        }
        self.route = route;
        self.requery(false, cx);
    }

    fn current(&self) -> Option<&dyn ViewHandle> {
        self.views.get(self.route.view)?.as_deref()
    }

    /// Ask the current view for items. `keep_selection` holds the cursor on
    /// the same item when the data changed under an unchanged query.
    pub(crate) fn requery(&mut self, keep_selection: bool, cx: &mut Context<Self>) {
        let ix = self.route.view;
        if self.views.get(ix).is_some_and(Option::is_none) {
            let view = self.registry[ix].build(cx);
            self.subscriptions.extend(view.connect(cx));
            self.views[ix] = Some(view);
        }
        if !keep_selection {
            self.selected = 0;
            self.list_scroll.scroll_to_item(0, ScrollStrategy::Top);
            self.body_scroll.set_offset(gpui::point(px(0.), px(0.)));
        }

        self.generation += 1;
        let generation = self.generation;
        let Some(view) = self.current() else { return };
        match view.items(&self.route.query, cx) {
            Items::Ready(items) => self.apply(items, keep_selection, cx),
            Items::Pending(task) => {
                self.pending = Some(cx.spawn(async move |this, cx| {
                    let items = task.await;
                    this.update(cx, |this, cx| {
                        if this.generation == generation {
                            this.apply(items, keep_selection, cx);
                        }
                    })
                    .ok();
                }));
            }
        }
        cx.notify();
    }

    fn apply(&mut self, items: Vec<Item>, keep_selection: bool, cx: &mut Context<Self>) {
        let previous = keep_selection
            .then(|| self.selected_item().map(|item| item.id.clone()))
            .flatten();
        let matching = self
            .current()
            .map_or(Matching::Shell, |view| view.matching(cx));
        self.matches = match matching {
            Matching::Shell => self.ranker.rank(&self.route.query, &items),
            Matching::View => (0..items.len())
                .map(|index| Ranked {
                    index,
                    highlights: Vec::new(),
                })
                .collect(),
        };
        self.items = items;
        if let Some(id) = previous
            && let Some(pos) = self
                .matches
                .iter()
                .position(|m| self.items[m.index].id == id)
        {
            self.selected = pos;
        }
        self.selected = self.selected.min(self.matches.len().saturating_sub(1));
        self.pending = None;
        cx.notify();
    }

    fn selected_item(&self) -> Option<&Item> {
        let ranked = self.matches.get(self.selected)?;
        self.items.get(ranked.index)
    }

    fn activate(&mut self, action: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.selected_item().cloned() else {
            return;
        };
        let Some(action) = item.actions.get(action) else {
            return;
        };
        let Some(view) = self.current() else { return };
        let outcome = view.activate(&item, &action.id, cx);
        self.apply_outcome(outcome, window, cx);
    }

    fn apply_outcome(&mut self, outcome: Outcome, window: &mut Window, cx: &mut Context<Self>) {
        match outcome {
            Outcome::Close => close(window, cx),
            Outcome::Stay => cx.notify(),
            Outcome::SetQuery(text) => {
                self.input.set_text(text);
                self.sync(cx);
            }
        }
    }

    fn navigate(&mut self, key: NavKey, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(view) = self.current()
            && let Some(outcome) = view.on_key(key, cx)
        {
            self.apply_outcome(outcome, window, cx);
            return;
        }

        let count = self.matches.len();
        let columns = self.columns.max(1);
        match key {
            NavKey::Enter => return self.activate(0, window, cx),
            NavKey::Left if columns > 1 => self.selected = self.selected.saturating_sub(1),
            NavKey::Right if columns > 1 => {
                self.selected = (self.selected + 1).min(count.saturating_sub(1));
            }
            NavKey::Left => self.input.move_left(false),
            NavKey::Right => self.input.move_right(false),
            _ if count == 0 => {}
            // A single column wraps; a grid stops at its edges.
            NavKey::Up if columns == 1 => self.selected = (self.selected + count - 1) % count,
            NavKey::Down if columns == 1 => self.selected = (self.selected + 1) % count,
            NavKey::Up => self.selected = self.selected.saturating_sub(columns),
            NavKey::Down => self.selected = (self.selected + columns).min(count - 1),
        }
        self.scroll_to_selected();
        cx.notify();
    }

    fn page(&mut self, down: bool, cx: &mut Context<Self>) {
        let count = self.matches.len();
        if count == 0 {
            return;
        }
        let step = ROWS_PER_PAGE * self.columns.max(1);
        self.selected = if down {
            (self.selected + step).min(count - 1)
        } else {
            self.selected.saturating_sub(step)
        };
        self.scroll_to_selected();
        cx.notify();
    }

    fn scroll_to_selected(&self) {
        self.list_scroll
            .scroll_to_item(self.selected / self.columns.max(1), ScrollStrategy::Nearest);
    }

    fn edit(&mut self, cx: &mut Context<Self>, edit: impl FnOnce(&mut InputBuffer)) {
        edit(&mut self.input);
        self.sync(cx);
        cx.notify();
    }

    fn placeholder(&self) -> String {
        let default = self.metas.iter().find(|meta| meta.default);
        let help = self.metas.iter().find(|meta| meta.id == "help");
        match (default, help) {
            (Some(default), Some(help)) => format!(
                "Search {} or type {} for commands...",
                default.name.to_lowercase(),
                help.prefix
            ),
            _ => "Search...".to_string(),
        }
    }

    /// Bare prefixes for the footer; full names live in help.
    fn prefix_hints(&self) -> String {
        self.metas
            .iter()
            .map(|meta| meta.prefix.trim())
            .filter(|prefix| !prefix.is_empty())
            .collect::<Vec<_>>()
            .join("  ")
    }

    fn render_list_row(
        &self,
        row: usize,
        columns: usize,
        height: gpui::Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let start = row * columns;
        let end = (start + columns).min(self.matches.len());
        let view = self.current();
        let cells = (start..end).map(|ix| {
            let ranked = &self.matches[ix];
            let item = &self.items[ranked.index];
            let selected = ix == self.selected;
            let content = view
                .and_then(|view| view.render_row(item, selected, window, cx))
                .unwrap_or_else(|| {
                    row::default_row(item, &ranked.highlights, selected, self.owner, window, cx)
                });
            div()
                .id(("launcher-row", ix))
                .flex_1()
                .min_w(px(0.))
                .h_full()
                .flex()
                .flex_col()
                .justify_center()
                .overflow_hidden()
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.selected = ix;
                    this.activate(0, window, cx);
                }))
                .child(content)
        });
        let fillers = (end - start..columns).map(|_| div().flex_1());

        div()
            .h(height)
            .w_full()
            .flex()
            .when(columns > 1, |el| {
                el.gap(Spacing::Medium.pixels())
                    .px(Spacing::Medium.pixels())
            })
            .children(cells)
            .children(fillers)
            .into_any_element()
    }
}

fn close(window: &mut Window, cx: &mut App) {
    crate::windows::ShellWindows::set_launcher(cx, None);
    window.remove_window();
}

impl Focusable for Launcher {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Launcher {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(cx.theme().font_size);
        if !self.focus_handle.is_focused(window) {
            self.focus_handle.focus(window, cx);
        }

        let view_name = self.metas[self.route.view].name;
        let (body, header, row_height, empty_text, hints) = match self.current() {
            Some(view) => (
                view.body(cx),
                view.render_header(window, cx),
                view.row_height(cx),
                view.empty_text(cx),
                view.footer_hints(self.selected_item(), cx),
            ),
            None => (Body::List, None, px(0.), None, None),
        };
        let hints = hints.unwrap_or_else(|| {
            let mut hints: Vec<Hint> = self
                .selected_item()
                .map(|item| {
                    item.actions
                        .iter()
                        .zip(["Enter", "Shift Enter"])
                        .map(|(action, key)| (action.label.clone(), key.into()))
                        .collect()
                })
                .unwrap_or_default();
            hints.push(("Close".into(), "Esc".into()));
            hints
        });

        let body = match body {
            Body::Custom(view) => {
                self.columns = 1;
                div()
                    .id("view-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.body_scroll)
                    .child(view)
                    .into_any_element()
            }
            Body::List | Body::Grid { .. } if self.matches.is_empty() => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .px(Spacing::XLarge.pixels())
                .children(
                    empty_text
                        .map(|text| Label::new(text).size(TextSize::Small).color(Color::Muted)),
                )
                .into_any_element(),
            Body::List | Body::Grid { .. } => {
                let columns = match body {
                    Body::Grid { columns } => columns.max(1),
                    _ => 1,
                };
                self.columns = columns;
                let rows = self.matches.len().div_ceil(columns);
                // The list sizes itself from its own style, so it needs a
                // parent with a definite height.
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        VirtualList::new(
                            "launcher-list",
                            rows,
                            cx.processor(move |this, range: Range<usize>, window, cx| {
                                range
                                    .map(|row| {
                                        this.render_list_row(row, columns, row_height, window, cx)
                                    })
                                    .collect::<Vec<_>>()
                            }),
                        )
                        .track_scroll(self.list_scroll.clone())
                        .size_full(),
                    )
                    .into_any_element()
            }
        };

        div()
            .id("launcher")
            .track_focus(&self.focus_handle)
            .key_context("Launcher")
            .on_action(cx.listener(|this, _: &Cancel, window, cx| {
                // First Esc clears the input; the second closes.
                if this.input.is_empty() {
                    close(window, cx);
                } else {
                    this.edit(cx, InputBuffer::clear);
                }
            }))
            .on_action(cx.listener(|this, _: &Confirm, window, cx| {
                this.navigate(NavKey::Enter, window, cx);
            }))
            .on_action(cx.listener(|this, _: &SecondaryConfirm, window, cx| {
                this.activate(1, window, cx);
            }))
            .on_action(cx.listener(|this, _: &CursorUp, window, cx| {
                this.navigate(NavKey::Up, window, cx);
            }))
            .on_action(cx.listener(|this, _: &CursorDown, window, cx| {
                this.navigate(NavKey::Down, window, cx);
            }))
            .on_action(cx.listener(|this, _: &CursorLeft, window, cx| {
                this.navigate(NavKey::Left, window, cx);
            }))
            .on_action(cx.listener(|this, _: &CursorRight, window, cx| {
                this.navigate(NavKey::Right, window, cx);
            }))
            .on_action(cx.listener(|this, _: &PageUp, _, cx| this.page(false, cx)))
            .on_action(cx.listener(|this, _: &PageDown, _, cx| this.page(true, cx)))
            .on_action(cx.listener(|this, _: &Backspace, _, cx| {
                this.edit(cx, InputBuffer::backspace);
            }))
            .on_action(cx.listener(|this, _: &DeleteWordBack, _, cx| {
                this.edit(cx, InputBuffer::delete_word_back);
            }))
            .on_action(cx.listener(|this, _: &WordLeft, _, cx| {
                this.edit(cx, |input| input.move_word_left(false));
            }))
            .on_action(cx.listener(|this, _: &WordRight, _, cx| {
                this.edit(cx, |input| input.move_word_right(false));
            }))
            .on_action(cx.listener(|this, _: &SelectWordLeft, _, cx| {
                this.edit(cx, |input| input.move_word_left(true));
            }))
            .on_action(cx.listener(|this, _: &SelectWordRight, _, cx| {
                this.edit(cx, |input| input.move_word_right(true));
            }))
            .on_action(cx.listener(|this, _: &SelectLeft, _, cx| {
                this.edit(cx, |input| input.move_left(true));
            }))
            .on_action(cx.listener(|this, _: &SelectRight, _, cx| {
                this.edit(cx, |input| input.move_right(true));
            }))
            .on_action(cx.listener(|this, _: &SelectAll, _, cx| {
                this.edit(cx, InputBuffer::select_all);
            }))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                let keystroke = &event.keystroke;
                if keystroke.modifiers.control || keystroke.modifiers.alt {
                    return;
                }
                let typed = keystroke.key_char.clone().or_else(|| {
                    (keystroke.key.chars().count() == 1).then(|| keystroke.key.clone())
                });
                if let Some(text) = typed
                    && !text.chars().any(char::is_control)
                {
                    this.edit(cx, |input| input.insert_str(&text));
                }
            }))
            .size_full()
            .child(
                LauncherFrame::new(render_input_line(&self.input, &self.placeholder(), cx))
                    .badge(view_name)
                    .hints(self.prefix_hints())
                    .actions(footer_hints(hints, cx))
                    .children(header)
                    .child(body),
            )
    }
}

pub fn init(cx: &mut App) {
    ThumbnailCache::init(cx);
}

/// Toggle the launcher window with optional prefilled input.
///
/// - Closed: opens it, with `input` if given.
/// - Open with `input`: replaces the input.
/// - Open without `input`: closes it.
pub fn toggle(input: Option<String>, cx: &mut App) {
    if let Some(handle) = crate::windows::ShellWindows::take_launcher(cx) {
        if let Some(input_text) = input {
            let update_result = handle.update(cx, |launcher, _, cx| {
                launcher.set_input(input_text, cx);
            });
            if update_result.is_ok() {
                crate::windows::ShellWindows::set_launcher(cx, Some(handle));
                return;
            }
        }
        let _ = handle.update(cx, |_, window, _| {
            window.remove_window();
        });
        return;
    }

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
        move |_, cx| cx.new(|cx| Launcher::new(input.clone(), cx)),
    ) {
        crate::windows::ShellWindows::set_launcher(cx, Some(handle));
    }
}

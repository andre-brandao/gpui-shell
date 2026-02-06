//! Launcher view trait and type-erased handle.
//!
//! Each launcher view is a GPUI entity that can hold state, subscribe to
//! service signals, and emit events. The [`ViewHandle`] wrapper provides
//! type erasure so the launcher can store heterogeneous views uniformly.

use gpui::{AnyElement, App, Context, Entity, EventEmitter};
use ui::IconName;

// ── Events ──────────────────────────────────────────────────────────

/// Events a view can emit to communicate with the launcher container.
#[derive(Clone, Debug)]
pub enum ViewEvent {
    /// Request the launcher to close.
    Close,
    /// Switch the launcher to a different view by prefix (e.g. "@ ").
    SwitchTo(String),
    /// The view's match list changed; the launcher should re-clamp selection.
    MatchesUpdated,
}

// ── Footer ──────────────────────────────────────────────────────────

/// A single action hint displayed in the launcher footer.
pub struct FooterAction {
    pub label: &'static str,
    pub key: &'static str,
}

// ── View metadata ───────────────────────────────────────────────────

/// Static metadata extracted from a view, used by the launcher for routing
/// and by the help view for listing available commands.
#[derive(Clone, Debug)]
pub struct ViewMeta {
    pub id: &'static str,
    pub prefix: &'static str,
    pub name: &'static str,
    pub icon: IconName,
    pub description: &'static str,
    pub is_default: bool,
    pub show_in_help: bool,
}

// ── Trait ────────────────────────────────────────────────────────────

/// A pluggable launcher view.
///
/// Implementors are GPUI entities (`Entity<Self>`) that manage their own
/// state and rendering. The launcher container calls these methods through
/// a type-erased [`ViewHandle`].
///
/// Views come in two flavours:
///
/// - **List views** (e.g. apps, help) — implement `render_item()` to render
///   individual selectable rows. The launcher iterates `match_count()` times.
///
/// - **Content views** (e.g. shell, web) — override `render_content()` to
///   return a single element for their entire body. When this returns `Some`,
///   the launcher skips the item loop.
pub trait LauncherView: EventEmitter<ViewEvent> + Sized + 'static {
    // ── Identity ──

    fn id(&self) -> &'static str;
    fn prefix(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn icon(&self) -> IconName;
    fn description(&self) -> &'static str;

    // ── Configuration ──

    fn is_default(&self) -> bool {
        false
    }

    fn show_in_help(&self) -> bool {
        true
    }

    // ── Items ──

    /// How many selectable items the view currently has.
    fn match_count(&self) -> usize;

    /// Update the view's internal filter/state for a new query string.
    /// The query has already had the prefix stripped.
    fn set_query(&mut self, query: &str, cx: &mut Context<Self>);

    // ── Rendering ──

    /// Render a single list item at `index`. `selected` is true if the
    /// launcher's selection cursor is on this item.
    fn render_item(&self, index: usize, selected: bool, cx: &App) -> AnyElement;

    /// Optional header rendered above the item list.
    fn render_header(&self, _cx: &App) -> Option<AnyElement> {
        None
    }

    /// Optional section rendered below the item list.
    fn render_footer(&self, _cx: &App) -> Option<AnyElement> {
        None
    }

    /// Full-content rendering for non-list views. When this returns `Some`,
    /// `render_item()` is not called and the returned element is used as
    /// the view body instead.
    fn render_content(&self, _cx: &App) -> Option<AnyElement> {
        None
    }

    // ── Actions ──

    /// Called when the user confirms the selection at `index` (Enter / click).
    fn confirm(&mut self, index: usize, cx: &mut Context<Self>);

    /// Action hints for the launcher footer bar.
    fn footer_actions(&self) -> Vec<FooterAction> {
        vec![
            FooterAction {
                label: "Open",
                key: "Enter",
            },
            FooterAction {
                label: "Close",
                key: "Esc",
            },
        ]
    }

    /// Extract static metadata from this view.
    fn meta(&self) -> ViewMeta {
        ViewMeta {
            id: self.id(),
            prefix: self.prefix(),
            name: self.name(),
            icon: self.icon(),
            description: self.description(),
            is_default: self.is_default(),
            show_in_help: self.show_in_help(),
        }
    }
}

// ── Type-erased handle ──────────────────────────────────────────────

/// A type-erased wrapper around `Entity<V>` where `V: LauncherView`.
///
/// Uses closure-based dispatch to call trait methods without requiring
/// `dyn LauncherView` (which GPUI entities don't support).
#[allow(clippy::type_complexity)]
pub struct ViewHandle {
    pub meta: ViewMeta,

    match_count: Box<dyn Fn(&App) -> usize>,
    set_query: Box<dyn Fn(&str, &mut App)>,
    render_item: Box<dyn Fn(usize, bool, &App) -> AnyElement>,
    render_header: Box<dyn Fn(&App) -> Option<AnyElement>>,
    render_footer: Box<dyn Fn(&App) -> Option<AnyElement>>,
    render_content: Box<dyn Fn(&App) -> Option<AnyElement>>,
    confirm: Box<dyn Fn(usize, &mut App)>,
    footer_actions: Box<dyn Fn(&App) -> Vec<FooterAction>>,
}

impl ViewHandle {
    /// Wrap a concrete `Entity<V>` into a type-erased handle.
    pub fn new<V: LauncherView>(entity: Entity<V>, cx: &App) -> Self {
        let meta = entity.read(cx).meta();

        let e = entity.clone();
        let match_count = Box::new(move |cx: &App| e.read(cx).match_count());

        let e = entity.clone();
        let set_query = Box::new(move |query: &str, cx: &mut App| {
            let q = query.to_string();
            e.update(cx, |view, cx| view.set_query(&q, cx));
        });

        let e = entity.clone();
        let render_item = Box::new(move |index: usize, selected: bool, cx: &App| {
            e.read(cx).render_item(index, selected, cx)
        });

        let e = entity.clone();
        let render_header = Box::new(move |cx: &App| e.read(cx).render_header(cx));

        let e = entity.clone();
        let render_footer = Box::new(move |cx: &App| e.read(cx).render_footer(cx));

        let e = entity.clone();
        let render_content = Box::new(move |cx: &App| e.read(cx).render_content(cx));

        let e = entity.clone();
        let confirm = Box::new(move |index: usize, cx: &mut App| {
            e.update(cx, |view, cx| view.confirm(index, cx));
        });

        let e = entity.clone();
        let footer_actions = Box::new(move |cx: &App| e.read(cx).footer_actions());

        Self {
            meta,
            match_count,
            set_query,
            render_item,
            render_header,
            render_footer,
            render_content,
            confirm,
            footer_actions,
        }
    }

    pub fn match_count(&self, cx: &App) -> usize {
        (self.match_count)(cx)
    }

    pub fn set_query(&self, query: &str, cx: &mut App) {
        (self.set_query)(query, cx)
    }

    pub fn render_item(&self, index: usize, selected: bool, cx: &App) -> AnyElement {
        (self.render_item)(index, selected, cx)
    }

    pub fn render_header(&self, cx: &App) -> Option<AnyElement> {
        (self.render_header)(cx)
    }

    pub fn render_footer(&self, cx: &App) -> Option<AnyElement> {
        (self.render_footer)(cx)
    }

    pub fn render_content(&self, cx: &App) -> Option<AnyElement> {
        (self.render_content)(cx)
    }

    pub fn confirm(&self, index: usize, cx: &mut App) {
        (self.confirm)(index, cx)
    }

    pub fn footer_actions(&self, cx: &App) -> Vec<FooterAction> {
        (self.footer_actions)(cx)
    }
}

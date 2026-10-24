//! The launcher view API.
//!
//! A view is a GPUI entity. Most views hand the launcher [`Item`]s and let it
//! rank, draw and activate them; richer ones draw their own rows or cards, or
//! take over the whole body with [`Body::Custom`].

use gpui::{
    AnyElement, AnyView, App, AppContext, Context, Entity, EventEmitter, Pixels, SharedString,
    Subscription, Task, Window, px,
};
use ui::{ActiveTheme, IconName, Spacing, TextSize};

use super::Launcher;
use super::item::Item;

/// gpui's default line height, as a multiple of the font size.
const LINE_HEIGHT: f32 = 1.618;

/// How the launcher knows a view: everything help, routing and the footer
/// need before the view is ever built.
#[derive(Clone)]
pub struct ViewMeta {
    pub id: &'static str,
    pub prefix: SharedString,
    pub name: &'static str,
    pub icon: IconName,
    pub description: &'static str,
    /// Owns queries without a prefix.
    pub default: bool,
    pub in_help: bool,
}

/// Who filters and orders the items.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Matching {
    /// The launcher fuzzy-ranks items against the query.
    Shell,
    /// The view already did; keep its order.
    View,
}

pub enum Items {
    Ready(Vec<Item>),
    /// Slow sources resolve off the frame; the old items stay until then.
    Pending(Task<Vec<Item>>),
}

pub enum Body {
    List,
    Grid {
        columns: usize,
    },
    /// The view draws the whole body itself; it gets no items.
    Custom(AnyView),
}

/// What happens after an item is activated.
pub enum Outcome {
    Close,
    Stay,
    /// Replace the whole input, prefix included.
    SetQuery(String),
}

/// Navigation keys a view may claim before the launcher moves the selection.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NavKey {
    Up,
    Down,
    Left,
    Right,
    Enter,
}

pub enum ViewEvent {
    /// The view's data changed under the same query; ask for items again.
    ItemsChanged,
}

pub type Hint = (SharedString, SharedString);

pub trait LauncherView: EventEmitter<ViewEvent> + Sized + 'static {
    fn matching(&self) -> Matching {
        Matching::Shell
    }

    /// Items for `query`. Called when the query changes and after
    /// [`ViewEvent::ItemsChanged`], never per frame.
    fn items(&mut self, query: &str, cx: &mut Context<Self>) -> Items;

    /// Run `action` on `item`. Clicks, Enter and Shift+Enter all land here.
    fn activate(&mut self, item: &Item, action: &str, cx: &mut Context<Self>) -> Outcome;

    fn body(&mut self, _cx: &mut Context<Self>) -> Body {
        Body::List
    }

    /// Height of every row (or grid row), so only visible rows are built.
    fn row_height(&self, cx: &App) -> Pixels {
        list_row_height(cx, &[TextSize::Default, TextSize::Small], px(28.))
    }

    /// Draw a row or card instead of the launcher's default. The launcher
    /// still owns its click.
    fn render_row(
        &self,
        _item: &Item,
        _selected: bool,
        _window: &Window,
        _cx: &App,
    ) -> Option<AnyElement> {
        None
    }

    fn render_header(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        None
    }

    /// Shown in place of the list when there are no items.
    fn empty_text(&self) -> Option<SharedString> {
        None
    }

    /// Claim a navigation key. `None` leaves it to the launcher.
    fn on_key(&mut self, _key: NavKey, _cx: &mut Context<Self>) -> Option<Outcome> {
        None
    }

    /// Footer hints. `None` derives them from the selected item's actions.
    fn footer_hints(&self, _selected: Option<&Item>) -> Option<Vec<Hint>> {
        None
    }
}

/// Height of a sparse `ListItem` stacking `lines` of text beside a slot at
/// least `min_content` tall.
pub fn list_row_height(cx: &App, lines: &[TextSize], min_content: Pixels) -> Pixels {
    let rem = cx.theme().font_size;
    let text: Pixels = lines
        .iter()
        .map(|size| size.rems().to_pixels(rem) * LINE_HEIGHT)
        .fold(px(0.), |a, b| a + b)
        + px(lines.len().saturating_sub(1) as f32);
    // Two px of slack so rounding never clips the last line.
    text.max(min_content) + Spacing::XSmall.pixels() * 2. + px(2.)
}

/// A built view behind a trait object, so the launcher can hold views of
/// different types.
pub(super) trait ViewHandle {
    fn matching(&self, cx: &App) -> Matching;
    fn items(&self, query: &str, cx: &mut App) -> Items;
    fn activate(&self, item: &Item, action: &str, cx: &mut App) -> Outcome;
    fn body(&self, cx: &mut App) -> Body;
    fn row_height(&self, cx: &App) -> Pixels;
    fn render_row(
        &self,
        item: &Item,
        selected: bool,
        window: &Window,
        cx: &App,
    ) -> Option<AnyElement>;
    fn render_header(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement>;
    fn empty_text(&self, cx: &App) -> Option<SharedString>;
    fn on_key(&self, key: NavKey, cx: &mut App) -> Option<Outcome>;
    fn footer_hints(&self, selected: Option<&Item>, cx: &App) -> Option<Vec<Hint>>;
    /// Redraw the launcher when the view notifies; requery on `ItemsChanged`.
    fn connect(&self, cx: &mut Context<Launcher>) -> [Subscription; 2];
}

impl<V: LauncherView> ViewHandle for Entity<V> {
    fn matching(&self, cx: &App) -> Matching {
        self.read(cx).matching()
    }

    fn items(&self, query: &str, cx: &mut App) -> Items {
        self.update(cx, |view, cx| view.items(query, cx))
    }

    fn activate(&self, item: &Item, action: &str, cx: &mut App) -> Outcome {
        self.update(cx, |view, cx| view.activate(item, action, cx))
    }

    fn body(&self, cx: &mut App) -> Body {
        self.update(cx, |view, cx| view.body(cx))
    }

    fn row_height(&self, cx: &App) -> Pixels {
        self.read(cx).row_height(cx)
    }

    fn render_row(
        &self,
        item: &Item,
        selected: bool,
        window: &Window,
        cx: &App,
    ) -> Option<AnyElement> {
        self.read(cx).render_row(item, selected, window, cx)
    }

    fn render_header(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        self.update(cx, |view, cx| view.render_header(window, cx))
    }

    fn empty_text(&self, cx: &App) -> Option<SharedString> {
        self.read(cx).empty_text()
    }

    fn on_key(&self, key: NavKey, cx: &mut App) -> Option<Outcome> {
        self.update(cx, |view, cx| view.on_key(key, cx))
    }

    fn footer_hints(&self, selected: Option<&Item>, cx: &App) -> Option<Vec<Hint>> {
        self.read(cx).footer_hints(selected)
    }

    fn connect(&self, cx: &mut Context<Launcher>) -> [Subscription; 2] {
        [
            cx.observe(self, |_, _, cx| cx.notify()),
            cx.subscribe(self, |launcher, _, event, cx| match event {
                ViewEvent::ItemsChanged => launcher.requery(true, cx),
            }),
        ]
    }
}

type BuildView = dyn Fn(&mut App) -> Box<dyn ViewHandle>;

/// A view the launcher can build on first use.
pub struct ViewDescriptor {
    pub meta: ViewMeta,
    build: Box<BuildView>,
}

impl ViewDescriptor {
    pub fn new<V: LauncherView>(
        meta: ViewMeta,
        build: impl Fn(&mut Context<V>) -> V + 'static,
    ) -> Self {
        Self {
            meta,
            build: Box::new(move |cx| Box::new(cx.new(&build))),
        }
    }

    pub(super) fn build(&self, cx: &mut App) -> Box<dyn ViewHandle> {
        (self.build)(cx)
    }
}

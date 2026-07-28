//! TextField - single-line text input with IME, caret, selection and undo.
//!
//! Derived from `crates/gpui/examples/input.rs` in
//! [zed-industries/zed](https://github.com/zed-industries/zed), licensed under
//! Apache-2.0. This file has been modified: it has been adapted to engram's
//! theming, component shape, handler aliases, and action namespace, and the
//! example wiring has been removed in favour of an embeddable
//! `TextField` entity. See `LICENSE-APACHE` at the workspace root.
//!
//! The heavy lifting is done by:
//!
//! - A [`TextField`] entity that owns the content, selection range, marked
//!   (IME-composing) range, and focus handle.
//! - A private [`TextElement`] that implements [`gpui::Element`] directly -
//!   shaping the text line with `window.text_system().shape_line(...)`,
//!   painting the caret and selection rects, and registering the field as
//!   an input target via `window.handle_input(...)`.
//! - An [`EntityInputHandler`] impl on `TextField` that handles all the
//!   UTF-16 <-> UTF-8 conversions the platform needs for IME, and that the
//!   OS calls into when composing text, querying bounds, or performing
//!   replacements.
//! - A set of [`gpui::actions!`]-style actions bound by [`crate::init`]:
//!   left/right/home/end navigation, select variants, backspace/delete,
//!   select-all, copy/cut/paste, and submit (Enter).
//!
//! Word-by-word navigation is bound to Ctrl/Alt+Left/Right (and the shift
//! variants for selection, and the backspace/delete variants for word
//! deletion). Undo/redo is bound to Cmd/Ctrl+Z (undo) and
//! Cmd/Ctrl+Shift+Z / Ctrl+Y (redo), with consecutive typing and
//! consecutive deletions grouped into a single undo step.
//!
//! Multi-line mode is opt-in via [`TextField::multi_line`]: in that mode
//! Shift+Enter inserts a newline (Enter still submits), Up/Down navigate
//! between lines with a preserved goal column, Home/End act line-aware,
//! paste preserves newlines, and the field auto-grows in height down to
//! [`TextField::min_lines`]. Only hard wraps (explicit `\n`) create new
//! visual rows - soft-wrap on width overflow is not implemented.
//! Word-double-click selection is still TODO.
//!
//! ## Usage
//!
//! ```ignore
//! let field = cx.new(|cx| TextField::with_value(cx, "initial"));
//! field.update(cx, |field, cx| {
//!     field.set_placeholder("Type here...");
//! });
//! // Render as a child:
//! div().child(field.clone())
//! ```

use std::collections::VecDeque;
use std::ops::Range;
use std::rc::Rc;

use crate::theme::{ActiveTheme, Radius, Spacing};
use gpui::{
    App, Bounds, ClipboardItem, Context, Element, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PaintQuad, Pixels, Point, Render, ShapedLine, SharedString, Style, Styled,
    TextRun, UTF16Selection, UnderlineStyle, Window, actions, div, fill, point, prelude::*, px,
    relative, size,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::components::stack::h_flex;
use crate::traits::StringHandler;

// -----------------------------------------------------------------------
// Actions
// -----------------------------------------------------------------------
//
// These are declared at module scope so they're reachable both from our
// `Render` impl (which calls `.on_action(cx.listener(...))`) and from
// `crate::init`, which binds keys to them. The action namespace is
// `engram_text_field` to avoid colliding with any action names a host app
// may have already bound (e.g. Zed's own `text_input::Backspace`).

actions!(
    engram_text_field,
    [
        Backspace,
        Delete,
        DeleteWordLeft,
        DeleteWordRight,
        Left,
        Right,
        Up,
        Down,
        WordLeft,
        WordRight,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectWordLeft,
        SelectWordRight,
        SelectAll,
        Newline,
        Home,
        End,
        Copy,
        Cut,
        Paste,
        Undo,
        Redo,
        Submit,
    ]
);

/// Classification used to group consecutive edits into a single undo step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditKind {
    Insert,
    DeleteBack,
    DeleteForward,
    Paste,
    Cut,
}

#[derive(Clone)]
struct UndoSnapshot {
    content: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
}

const UNDO_CAP: usize = 256;

/// Emitted when the user presses Enter. The payload is the field value
/// at the moment of submission.
pub struct TextFieldSubmitEvent(pub String);

// -----------------------------------------------------------------------
// TextField
// -----------------------------------------------------------------------

/// One logical line's shaped geometry, populated by the `TextElement` on
/// paint. In single-line mode there is always exactly one entry.
#[derive(Clone)]
struct ShapedRow {
    /// UTF-8 byte offset of the start of this line in `content`.
    byte_start: usize,
    /// Line contents as passed to the text system (excluding trailing '\n').
    line: ShapedLine,
}

/// Single-line text field by default; call [`TextField::multi_line`] to
/// enable newline insertion, auto-growing height, and up/down navigation.
///
/// See the module docs for the supported feature set.
pub struct TextField {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    /// UTF-8 byte offsets into `content`.
    selected_range: Range<usize>,
    selection_reversed: bool,
    /// UTF-8 byte offsets into `content` for the currently-composing IME
    /// range, if any.
    marked_range: Option<Range<usize>>,
    /// Populated during paint by the private `TextElement`. One entry per
    /// logical line - always one entry in single-line mode.
    last_rows: Vec<ShapedRow>,
    last_bounds: Option<Bounds<Pixels>>,
    last_line_height: Option<Pixels>,
    is_selecting: bool,
    multi_line: bool,
    min_lines: usize,
    width: Pixels,
    /// Preserved column for up/down navigation, in pixels from line start.
    /// Cleared by any non-up/down movement.
    goal_x: Option<Pixels>,
    on_change: Option<StringHandler>,
    on_submit: Option<StringHandler>,
    undo_stack: VecDeque<UndoSnapshot>,
    redo_stack: VecDeque<UndoSnapshot>,
    /// Kind of the edit currently being grouped. A caret movement, mouse
    /// click, or undo/redo clears it, starting a fresh group.
    pending_edit_kind: Option<EditKind>,
    /// Cached result of [`Self::line_ranges`]. Invalidated whenever
    /// `content` changes. For long multi-line buffers, recomputing on
    /// every Home/End/Up/Down keystroke was a measurable cost.
    line_ranges_cache: Option<Vec<Range<usize>>>,
}

impl TextField {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: SharedString::default(),
            placeholder: SharedString::default(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_rows: Vec::new(),
            last_bounds: None,
            last_line_height: None,
            is_selecting: false,
            multi_line: false,
            min_lines: 3,
            width: px(220.0),
            goal_x: None,
            on_change: None,
            on_submit: None,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            pending_edit_kind: None,
            line_ranges_cache: None,
        }
    }

    pub fn with_value(cx: &mut Context<Self>, value: impl Into<String>) -> Self {
        let mut this = Self::new(cx);
        let content: SharedString = value.into().into();
        this.selected_range = content.len()..content.len();
        this.content = content;
        this.line_ranges_cache = None;
        this
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    /// Enable multi-line mode. Enter inserts a newline instead of
    /// submitting; paste preserves newlines; Up/Down and line-aware
    /// Home/End become available; the field auto-grows in height.
    /// Use [`Self::min_lines`] to reserve vertical space when empty.
    pub fn multi_line(mut self) -> Self {
        self.multi_line = true;
        self
    }

    /// In multi-line mode, the minimum number of lines of height the
    /// field reserves even when the content is shorter. Ignored in
    /// single-line mode.
    pub fn min_lines(mut self, lines: usize) -> Self {
        self.min_lines = lines.max(1);
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    pub fn on_submit(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_submit = Some(Rc::new(handler));
        self
    }

    /// Read the current value.
    pub fn value(&self) -> &str {
        &self.content
    }

    /// Replace the value programmatically. Selection is clamped to the
    /// new content length. Undo history is cleared - programmatic sets
    /// are treated as a fresh document rather than a keystroke.
    pub fn set_value(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        let new: SharedString = value.into().into();
        let end = new.len();
        self.content = new;
        self.selected_range = end..end;
        self.selection_reversed = false;
        self.marked_range = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.pending_edit_kind = None;
        self.line_ranges_cache = None;
        cx.notify();
    }

    // ---------- undo / redo ----------

    fn snapshot(&self) -> UndoSnapshot {
        UndoSnapshot {
            content: self.content.clone(),
            selected_range: self.selected_range.clone(),
            selection_reversed: self.selection_reversed,
        }
    }

    fn restore(&mut self, snap: UndoSnapshot) {
        self.content = snap.content;
        self.selected_range = snap.selected_range;
        self.selection_reversed = snap.selection_reversed;
        self.marked_range = None;
        self.pending_edit_kind = None;
        self.line_ranges_cache = None;
    }

    /// Called before a mutation. Pushes a snapshot of the *pre-edit*
    /// state onto the undo stack, unless the previous edit matches the
    /// same grouping kind - in which case consecutive typing or deletion
    /// collapses into a single undo step.
    fn push_undo(&mut self, kind: EditKind) {
        let groupable = matches!(
            kind,
            EditKind::Insert | EditKind::DeleteBack | EditKind::DeleteForward
        );
        if groupable && self.pending_edit_kind == Some(kind) && !self.undo_stack.is_empty() {
            self.pending_edit_kind = Some(kind);
            return;
        }
        if self.undo_stack.len() >= UNDO_CAP {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(self.snapshot());
        self.redo_stack.clear();
        self.pending_edit_kind = Some(kind);
    }

    /// Call when the user performs a non-editing action (caret movement,
    /// selection change, click). The next edit starts a new undo group.
    fn undo_boundary(&mut self) {
        self.pending_edit_kind = None;
    }

    // ---------- internal helpers ----------

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    fn previous_word_boundary(&self, offset: usize) -> usize {
        previous_word_boundary(&self.content, offset)
    }

    fn next_word_boundary(&self, offset: usize) -> usize {
        next_word_boundary(&self.content, offset)
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        let Some(bounds) = self.last_bounds.as_ref() else {
            return 0;
        };
        if self.last_rows.is_empty() {
            return 0;
        }
        if position.y < bounds.top() {
            return 0;
        }
        let line_height = self.last_line_height.unwrap_or(px(20.0));
        let dy = (position.y - bounds.top()).max(px(0.0));
        let row_idx = ((dy / line_height) as usize).min(self.last_rows.len() - 1);
        let row = &self.last_rows[row_idx];
        let local_x = (position.x - bounds.left()).max(px(0.0));
        row.byte_start + row.line.closest_index_for_x(local_x)
    }

    // ---------- multi-line geometry ----------

    /// Byte ranges of each logical line (i.e. the span between consecutive
    /// '\n' characters), always at least one entry. The range covers the
    /// line text only - the terminating '\n' is not included.
    ///
    /// The result is cached until `content` next changes; the cache is
    /// invalidated by every site that reassigns `self.content`.
    fn line_ranges(&mut self) -> &[Range<usize>] {
        if self.line_ranges_cache.is_none() {
            let content = &self.content;
            let mut ranges = Vec::new();
            let mut start = 0;
            for (i, b) in content.as_bytes().iter().enumerate() {
                if *b == b'\n' {
                    ranges.push(start..i);
                    start = i + 1;
                }
            }
            ranges.push(start..content.len());
            self.line_ranges_cache = Some(ranges);
        }
        self.line_ranges_cache.as_deref().unwrap()
    }

    fn line_index_for_offset(&self, offset: usize) -> usize {
        let mut idx = 0;
        let mut remaining = offset;
        for b in self.content.as_bytes() {
            if remaining == 0 {
                break;
            }
            if *b == b'\n' {
                idx += 1;
            }
            remaining -= 1;
        }
        idx
    }

    fn line_range_for_offset(&mut self, offset: usize) -> Range<usize> {
        let line_idx = self.line_index_for_offset(offset);
        let ranges = self.line_ranges();
        let idx = line_idx.min(ranges.len() - 1);
        ranges[idx].clone()
    }

    /// X coordinate of `offset` within its own logical line, using the
    /// last shaped layout. Returns 0 if we haven't painted yet or the
    /// offset falls outside the shaped rows.
    fn x_for_offset(&self, offset: usize) -> Pixels {
        let Some(row) = self
            .last_rows
            .iter()
            .find(|r| offset >= r.byte_start && offset <= r.byte_start + r.line.len)
        else {
            return px(0.0);
        };
        row.line.x_for_index(offset - row.byte_start)
    }

    /// Byte offset closest to `x` on line `line_idx`, using the last
    /// shaped layout. Clamps to the first/last line if out of range.
    fn offset_for_line_x(&mut self, line_idx: usize, x: Pixels) -> usize {
        if self.last_rows.is_empty() {
            let ranges = self.line_ranges();
            let idx = line_idx.min(ranges.len() - 1);
            return ranges[idx].end;
        }
        let idx = line_idx.min(self.last_rows.len() - 1);
        let row = &self.last_rows[idx];
        row.byte_start + row.line.closest_index_for_x(x)
    }

    fn notify_change(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(handler) = self.on_change.clone() {
            let value = self.content.to_string();
            handler(&value, window, cx);
        }
        cx.notify();
    }

    // ---------- UTF-16 <-> UTF-8 conversion (for IME) ----------

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8 = 0;
        let mut utf16 = 0;
        for ch in self.content.chars() {
            if utf16 >= offset {
                break;
            }
            utf16 += ch.len_utf16();
            utf8 += ch.len_utf8();
        }
        utf8
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16 = 0;
        let mut utf8 = 0;
        for ch in self.content.chars() {
            if utf8 >= offset {
                break;
            }
            utf8 += ch.len_utf8();
            utf16 += ch.len_utf16();
        }
        utf16
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range.start)..self.offset_from_utf16(range.end)
    }

    // ---------- action handlers ----------

    fn on_left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_boundary();
        self.goal_x = None;
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    fn on_right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_boundary();
        self.goal_x = None;
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    fn on_select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_boundary();
        self.goal_x = None;
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn on_select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_boundary();
        self.goal_x = None;
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn on_word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_boundary();
        self.goal_x = None;
        let offset = if self.selected_range.is_empty() {
            self.previous_word_boundary(self.cursor_offset())
        } else {
            self.selected_range.start
        };
        self.move_to(offset, cx);
    }

    fn on_word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_boundary();
        self.goal_x = None;
        let offset = if self.selected_range.is_empty() {
            self.next_word_boundary(self.cursor_offset())
        } else {
            self.selected_range.end
        };
        self.move_to(offset, cx);
    }

    fn on_select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_boundary();
        self.goal_x = None;
        self.select_to(self.previous_word_boundary(self.cursor_offset()), cx);
    }

    fn on_select_word_right(
        &mut self,
        _: &SelectWordRight,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.undo_boundary();
        self.goal_x = None;
        self.select_to(self.next_word_boundary(self.cursor_offset()), cx);
    }

    fn on_delete_word_left(
        &mut self,
        _: &DeleteWordLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            let prev = self.previous_word_boundary(self.cursor_offset());
            if self.cursor_offset() == prev {
                return;
            }
            self.select_to(prev, cx);
        }
        self.push_undo(EditKind::DeleteBack);
        let range = self.selected_range.clone();
        self.perform_edit(range, "", window, cx);
    }

    fn on_delete_word_right(
        &mut self,
        _: &DeleteWordRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            let next = self.next_word_boundary(self.cursor_offset());
            if self.cursor_offset() == next {
                return;
            }
            self.select_to(next, cx);
        }
        self.push_undo(EditKind::DeleteForward);
        let range = self.selected_range.clone();
        self.perform_edit(range, "", window, cx);
    }

    fn on_select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_boundary();
        self.goal_x = None;
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx);
    }

    fn on_home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_boundary();
        self.goal_x = None;
        let offset = if self.multi_line {
            self.line_range_for_offset(self.cursor_offset()).start
        } else {
            0
        };
        self.move_to(offset, cx);
    }

    fn on_end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_boundary();
        self.goal_x = None;
        let offset = if self.multi_line {
            self.line_range_for_offset(self.cursor_offset()).end
        } else {
            self.content.len()
        };
        self.move_to(offset, cx);
    }

    fn on_up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_boundary();
        let cursor = self.cursor_offset();
        let line_idx = self.line_index_for_offset(cursor);
        if line_idx == 0 {
            self.goal_x = None;
            self.move_to(0, cx);
            return;
        }
        let x = self.goal_x.unwrap_or_else(|| self.x_for_offset(cursor));
        self.goal_x = Some(x);
        let target = self.offset_for_line_x(line_idx - 1, x);
        self.move_to(target, cx);
    }

    fn on_down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_boundary();
        let cursor = self.cursor_offset();
        let line_idx = self.line_index_for_offset(cursor);
        let total_lines = self.line_ranges().len();
        if line_idx + 1 >= total_lines {
            self.goal_x = None;
            let end = self.content.len();
            self.move_to(end, cx);
            return;
        }
        let x = self.goal_x.unwrap_or_else(|| self.x_for_offset(cursor));
        self.goal_x = Some(x);
        let target = self.offset_for_line_x(line_idx + 1, x);
        self.move_to(target, cx);
    }

    fn on_select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_boundary();
        let cursor = self.cursor_offset();
        let line_idx = self.line_index_for_offset(cursor);
        if line_idx == 0 {
            self.goal_x = None;
            self.select_to(0, cx);
            return;
        }
        let x = self.goal_x.unwrap_or_else(|| self.x_for_offset(cursor));
        self.goal_x = Some(x);
        let target = self.offset_for_line_x(line_idx - 1, x);
        self.select_to(target, cx);
    }

    fn on_select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_boundary();
        let cursor = self.cursor_offset();
        let line_idx = self.line_index_for_offset(cursor);
        let total_lines = self.line_ranges().len();
        if line_idx + 1 >= total_lines {
            self.goal_x = None;
            let end = self.content.len();
            self.select_to(end, cx);
            return;
        }
        let x = self.goal_x.unwrap_or_else(|| self.x_for_offset(cursor));
        self.goal_x = Some(x);
        let target = self.offset_for_line_x(line_idx + 1, x);
        self.select_to(target, cx);
    }

    fn on_newline(&mut self, _: &Newline, window: &mut Window, cx: &mut Context<Self>) {
        if !self.multi_line {
            return;
        }
        self.push_undo(EditKind::Insert);
        let range = self.selected_range.clone();
        self.perform_edit(range, "\n", window, cx);
    }

    fn on_backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let prev = self.previous_boundary(self.cursor_offset());
            if self.cursor_offset() == prev {
                return;
            }
            self.select_to(prev, cx);
        }
        self.push_undo(EditKind::DeleteBack);
        let range = self.selected_range.clone();
        self.perform_edit(range, "", window, cx);
    }

    fn on_delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let next = self.next_boundary(self.cursor_offset());
            if self.cursor_offset() == next {
                return;
            }
            self.select_to(next, cx);
        }
        self.push_undo(EditKind::DeleteForward);
        let range = self.selected_range.clone();
        self.perform_edit(range, "", window, cx);
    }

    fn on_copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn on_cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.push_undo(EditKind::Cut);
            let range = self.selected_range.clone();
            self.perform_edit(range, "", window, cx);
        }
    }

    fn on_paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            // Single-line field: flatten newlines to spaces so a multi-line
            // clipboard payload can't silently split the content.
            let sanitized = if self.multi_line {
                text
            } else {
                text.replace('\n', " ")
            };
            self.push_undo(EditKind::Paste);
            let range = self.selected_range.clone();
            self.perform_edit(range, &sanitized, window, cx);
        }
    }

    fn handle_submit(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.content.to_string();
        if let Some(handler) = self.on_submit.clone() {
            handler(&value, window, cx);
        }
        cx.emit(TextFieldSubmitEvent(value));
    }

    // ---------- mouse ----------

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.undo_boundary();
        self.is_selecting = true;
        let idx = self.index_for_mouse_position(event.position);
        if event.modifiers.shift {
            self.select_to(idx, cx);
        } else {
            self.move_to(idx, cx);
        }
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            let idx = self.index_for_mouse_position(event.position);
            self.select_to(idx, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    /// Core mutation: replace `range` with `new_text`, collapse selection
    /// to the end of the inserted text, notify listeners. Does not push
    /// undo - callers are responsible for calling [`push_undo`] first.
    fn perform_edit(
        &mut self,
        range: Range<usize>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut out = String::with_capacity(self.content.len() + new_text.len());
        out.push_str(&self.content[..range.start]);
        out.push_str(new_text);
        out.push_str(&self.content[range.end..]);
        self.content = out.into();
        self.line_ranges_cache = None;
        let cursor = range.start + new_text.len();
        self.selected_range = cursor..cursor;
        self.marked_range.take();
        self.notify_change(window, cx);
    }

    fn on_undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        let Some(prev) = self.undo_stack.pop_back() else {
            return;
        };
        let current = self.snapshot();
        self.restore(prev);
        self.redo_stack.push_back(current);
        cx.notify();
    }

    fn on_redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        let Some(next) = self.redo_stack.pop_back() else {
            return;
        };
        let current = self.snapshot();
        self.restore(next);
        self.undo_stack.push_back(current);
        cx.notify();
    }
}

impl EventEmitter<TextFieldSubmitEvent> for TextField {}

impl Focusable for TextField {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// -----------------------------------------------------------------------
// EntityInputHandler - platform-facing IME + edit interface
// -----------------------------------------------------------------------

impl EntityInputHandler for TextField {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
        self.undo_boundary();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        let kind = if new_text.is_empty() {
            EditKind::DeleteBack
        } else {
            EditKind::Insert
        };
        self.push_undo(kind);
        self.perform_edit(range, new_text, window, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        // IME composition groups with typing; an unmark (see `unmark_text`)
        // ends the group so the next real edit pushes a fresh snapshot.
        self.push_undo(EditKind::Insert);

        let mut out = String::with_capacity(self.content.len() + new_text.len());
        out.push_str(&self.content[..range.start]);
        out.push_str(new_text);
        out.push_str(&self.content[range.end..]);
        self.content = out.into();
        self.line_ranges_cache = None;

        self.marked_range = if new_text.is_empty() {
            None
        } else {
            Some(range.start..range.start + new_text.len())
        };
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .map(|r| r.start + range.start..r.end + range.end)
            .unwrap_or_else(|| {
                let cursor = range.start + new_text.len();
                cursor..cursor
            });
        self.notify_change(window, cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        if self.last_rows.is_empty() {
            return None;
        }
        let line_height = self.last_line_height.unwrap_or(px(20.0));
        let range = self.range_from_utf16(&range_utf16);
        let start_line = self.line_index_for_offset(range.start);
        let start_x = self.x_for_offset(range.start);
        let end_x = self.x_for_offset(range.end);
        Some(Bounds::from_corners(
            point(
                bounds.left() + start_x,
                bounds.top() + line_height * start_line as f32,
            ),
            point(
                bounds.left() + end_x,
                bounds.top() + line_height * (start_line + 1) as f32,
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        p: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds?;
        let _line_point = bounds.localize(&p)?;
        if self.last_rows.is_empty() {
            return None;
        }
        let utf8 = self.index_for_mouse_position(p);
        Some(self.offset_to_utf16(utf8))
    }
}

// -----------------------------------------------------------------------
// TextElement - the custom Element that shapes & paints the line
// -----------------------------------------------------------------------

struct TextElement {
    input: Entity<TextField>,
}

struct PreparedRow {
    byte_start: usize,
    line: ShapedLine,
}

struct TextPrepaintState {
    rows: Vec<PreparedRow>,
    cursor: Option<PaintQuad>,
    selections: Vec<PaintQuad>,
    line_height: Pixels,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = TextPrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let input = self.input.read(cx);
        let lines = if input.multi_line {
            #[allow(clippy::naive_bytecount)]
            let n = input
                .content
                .as_bytes()
                .iter()
                .filter(|b| **b == b'\n')
                .count();
            n + 1
        } else {
            1
        };
        let min_lines = if input.multi_line { input.min_lines } else { 1 };
        let visible = lines.max(min_lines);
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = (window.line_height() * visible as f32).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let colors = cx.theme().colors();
        let text_color = colors.text;
        let placeholder_color = colors.text_placeholder;
        let caret_color = colors.text;
        let selection_color = colors.element_selected;

        let input = self.input.read(cx);
        let content = input.content.clone();
        let selected_range = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let marked_range = input.marked_range.clone();
        let multi_line = input.multi_line;
        let placeholder = input.placeholder.clone();
        let style = window.text_style();

        let line_height = window.line_height();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let font = style.font();

        // Choose between placeholder and real content as the display payload.
        // Placeholder is rendered as a single line regardless of mode.
        let show_placeholder = content.is_empty();
        let display_source: SharedString = if show_placeholder {
            placeholder
        } else {
            content.clone()
        };
        let display_color = if show_placeholder {
            placeholder_color
        } else {
            text_color
        };

        // Split into logical lines preserving byte offsets for caret/selection
        // math. A placeholder is treated as one opaque line.
        let line_specs: Vec<(usize, SharedString)> = if show_placeholder || !multi_line {
            vec![(0, display_source)]
        } else {
            let mut specs = Vec::new();
            let mut start = 0;
            let text = display_source.as_ref();
            for (i, b) in text.as_bytes().iter().enumerate() {
                if *b == b'\n' {
                    specs.push((start, SharedString::from(text[start..i].to_string())));
                    start = i + 1;
                }
            }
            specs.push((start, SharedString::from(text[start..].to_string())));
            specs
        };

        let mut rows: Vec<PreparedRow> = Vec::with_capacity(line_specs.len());
        for (byte_start, line_text) in &line_specs {
            let len = line_text.len();
            let base = TextRun {
                len,
                font: font.clone(),
                color: display_color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let runs = if let Some(mr) = marked_range
                .as_ref()
                .filter(|mr| mr.start < byte_start + len && mr.end > *byte_start)
            {
                // Intersect the marked range with this row.
                let local_start = mr.start.saturating_sub(*byte_start);
                let local_end = (mr.end - *byte_start).min(len);
                let head = TextRun {
                    len: local_start,
                    ..base.clone()
                };
                let mid = TextRun {
                    len: local_end - local_start,
                    underline: Some(UnderlineStyle {
                        color: Some(base.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..base.clone()
                };
                let tail = TextRun {
                    len: len - local_end,
                    ..base.clone()
                };
                [head, mid, tail]
                    .into_iter()
                    .filter(|r| r.len > 0)
                    .collect::<Vec<_>>()
            } else {
                vec![base]
            };

            let shaped = window
                .text_system()
                .shape_line(line_text.clone(), font_size, &runs, None);
            rows.push(PreparedRow {
                byte_start: *byte_start,
                line: shaped,
            });
        }

        // Caret or selection quads.
        let (cursor_quad, selections) = if show_placeholder || selected_range.is_empty() {
            let (line_idx, local) = if show_placeholder {
                (0, 0)
            } else {
                let li = offset_line_index(&content, cursor);
                (li, cursor - rows[li].byte_start)
            };
            let cursor_x = rows[line_idx].line.x_for_index(local);
            let caret_y = bounds.top() + line_height * line_idx as f32;
            (
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_x, caret_y),
                        size(px(1.5), line_height),
                    ),
                    caret_color,
                )),
                Vec::new(),
            )
        } else {
            // Build one rect per covered row.
            let mut quads = Vec::new();
            for (row_idx, row) in rows.iter().enumerate() {
                let row_end = row.byte_start + row.line.len;
                if selected_range.end <= row.byte_start || selected_range.start > row_end {
                    continue;
                }
                let local_start = selected_range.start.saturating_sub(row.byte_start);
                let local_end = (selected_range.end - row.byte_start).min(row.line.len);
                let x0 = row.line.x_for_index(local_start);
                let x1 = row.line.x_for_index(local_end);
                // Extend to end-of-line when the selection continues onto a
                // following row, so the highlight visually covers the newline.
                let extend_eol = selected_range.end > row_end && row_idx + 1 < rows.len();
                let right = if extend_eol { x1 + px(6.0) } else { x1 };
                let top = bounds.top() + line_height * row_idx as f32;
                quads.push(fill(
                    Bounds::from_corners(
                        point(bounds.left() + x0, top),
                        point(bounds.left() + right, top + line_height),
                    ),
                    selection_color,
                ));
            }
            (None, quads)
        };

        TextPrepaintState {
            rows,
            cursor: cursor_quad,
            selections,
            line_height,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        for selection in prepaint.selections.drain(..) {
            window.paint_quad(selection);
        }
        let line_height = prepaint.line_height;
        let rows = std::mem::take(&mut prepaint.rows);
        for (idx, row) in rows.iter().enumerate() {
            let origin = point(bounds.left(), bounds.top() + line_height * idx as f32);
            row.line
                .paint(origin, line_height, gpui::TextAlign::Left, None, window, cx)
                .ok();
        }

        if focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        let rows_for_field = rows
            .into_iter()
            .map(|r| ShapedRow {
                byte_start: r.byte_start,
                line: r.line,
            })
            .collect::<Vec<_>>();
        self.input.update(cx, |input, _cx| {
            input.last_rows = rows_for_field;
            input.last_bounds = Some(bounds);
            input.last_line_height = Some(line_height);
        });
    }
}

fn offset_line_index(content: &str, offset: usize) -> usize {
    let mut idx = 0;
    let mut remaining = offset;
    for b in content.as_bytes() {
        if remaining == 0 {
            break;
        }
        if *b == b'\n' {
            idx += 1;
        }
        remaining -= 1;
    }
    idx
}

// -----------------------------------------------------------------------
// Render
// -----------------------------------------------------------------------

impl Render for TextField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();
        let is_focused = self.focus_handle.is_focused(window);
        let border = if is_focused {
            colors.border_focused
        } else {
            colors.border
        };

        h_flex()
            .id("engram-text-field")
            .key_context("TextField")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_left))
            .on_action(cx.listener(Self::on_right))
            .on_action(cx.listener(Self::on_up))
            .on_action(cx.listener(Self::on_down))
            .on_action(cx.listener(Self::on_select_left))
            .on_action(cx.listener(Self::on_select_right))
            .on_action(cx.listener(Self::on_select_up))
            .on_action(cx.listener(Self::on_select_down))
            .on_action(cx.listener(Self::on_word_left))
            .on_action(cx.listener(Self::on_word_right))
            .on_action(cx.listener(Self::on_select_word_left))
            .on_action(cx.listener(Self::on_select_word_right))
            .on_action(cx.listener(Self::on_delete_word_left))
            .on_action(cx.listener(Self::on_delete_word_right))
            .on_action(cx.listener(Self::on_select_all))
            .on_action(cx.listener(Self::on_home))
            .on_action(cx.listener(Self::on_end))
            .on_action(cx.listener(Self::on_newline))
            .on_action(cx.listener(Self::on_backspace))
            .on_action(cx.listener(Self::on_delete))
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_undo))
            .on_action(cx.listener(Self::on_redo))
            .on_action(cx.listener(Self::handle_submit))
            // The mouse-down handler both grabs focus (so subsequent
            // key events route here) and positions the caret. Splitting
            // those two concerns across two handlers worked but added
            // an unnecessary closure layer; one listener does both.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus_handle, cx);
                    this.on_mouse_down(event, window, cx);
                    cx.stop_propagation();
                }),
            )
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .w(self.width)
            .map(|el| {
                if self.multi_line {
                    // Auto-grow: the inner TextElement reports a height of
                    // (lines * line_height), and the outer box follows.
                    el.py(Spacing::XSmall.pixels())
                } else {
                    el.h(px(24.0)).items_center()
                }
            })
            .px(Spacing::Small.pixels())
            .rounded(Radius::XSmall.pixels())
            .border_1()
            .border_color(border)
            .bg(colors.element_background)
            .cursor_text()
            .line_height(px(20.0))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .map(|el| {
                        if self.multi_line {
                            el.items_start()
                        } else {
                            el.h_full().items_center()
                        }
                    })
                    .child(TextElement { input: cx.entity() }),
            )
    }
}

fn is_word(s: &str) -> bool {
    s.chars().any(|c| c.is_alphanumeric() || c == '_')
}

fn previous_word_boundary(content: &str, offset: usize) -> usize {
    content
        .split_word_bound_indices()
        .rev()
        .find_map(|(idx, word)| (is_word(word) && idx < offset).then_some(idx))
        .unwrap_or(0)
}

fn next_word_boundary(content: &str, offset: usize) -> usize {
    content
        .split_word_bound_indices()
        .find_map(|(idx, word)| {
            let end = idx + word.len();
            (is_word(word) && end > offset).then_some(end)
        })
        .unwrap_or(content.len())
}

/// Helper for callers that just want a focused field with an initial value.
pub fn text_field(cx: &mut App, initial: impl Into<String>) -> Entity<TextField> {
    let initial = initial.into();
    cx.new(|cx| TextField::with_value(cx, initial))
}

// -----------------------------------------------------------------------
// Keybinding registration
// -----------------------------------------------------------------------

/// Register the default keybindings for [`TextField`]. Called by
/// [`crate::init`], but also exposed separately in case an app wants to
/// initialize engram without binding our keys (e.g. to remap them).
pub fn bind_text_field_keys(cx: &mut App) {
    use gpui::KeyBinding;
    cx.bind_keys([
        KeyBinding::new("left", Left, Some("TextField")),
        KeyBinding::new("right", Right, Some("TextField")),
        KeyBinding::new("up", Up, Some("TextField")),
        KeyBinding::new("down", Down, Some("TextField")),
        KeyBinding::new("shift-left", SelectLeft, Some("TextField")),
        KeyBinding::new("shift-right", SelectRight, Some("TextField")),
        KeyBinding::new("shift-up", SelectUp, Some("TextField")),
        KeyBinding::new("shift-down", SelectDown, Some("TextField")),
        // In multi-line mode, Shift+Enter inserts a newline and Enter
        // still submits; the Newline action is a no-op when single-line
        // is active (see `on_newline`).
        KeyBinding::new("shift-enter", Newline, Some("TextField")),
        // Word-by-word navigation: alt on macOS, ctrl on Linux/Windows.
        KeyBinding::new("alt-left", WordLeft, Some("TextField")),
        KeyBinding::new("alt-right", WordRight, Some("TextField")),
        KeyBinding::new("ctrl-left", WordLeft, Some("TextField")),
        KeyBinding::new("ctrl-right", WordRight, Some("TextField")),
        KeyBinding::new("alt-shift-left", SelectWordLeft, Some("TextField")),
        KeyBinding::new("alt-shift-right", SelectWordRight, Some("TextField")),
        KeyBinding::new("ctrl-shift-left", SelectWordLeft, Some("TextField")),
        KeyBinding::new("ctrl-shift-right", SelectWordRight, Some("TextField")),
        KeyBinding::new("alt-backspace", DeleteWordLeft, Some("TextField")),
        KeyBinding::new("alt-delete", DeleteWordRight, Some("TextField")),
        KeyBinding::new("ctrl-backspace", DeleteWordLeft, Some("TextField")),
        KeyBinding::new("ctrl-delete", DeleteWordRight, Some("TextField")),
        KeyBinding::new("home", Home, Some("TextField")),
        KeyBinding::new("end", End, Some("TextField")),
        KeyBinding::new("backspace", Backspace, Some("TextField")),
        KeyBinding::new("delete", Delete, Some("TextField")),
        KeyBinding::new("enter", Submit, Some("TextField")),
        // Cmd bindings for macOS; ctrl for Linux/Windows. Bind both so
        // the field just works regardless of the host platform's
        // convention.
        KeyBinding::new("cmd-a", SelectAll, Some("TextField")),
        KeyBinding::new("ctrl-a", SelectAll, Some("TextField")),
        KeyBinding::new("cmd-c", Copy, Some("TextField")),
        KeyBinding::new("ctrl-c", Copy, Some("TextField")),
        KeyBinding::new("cmd-x", Cut, Some("TextField")),
        KeyBinding::new("ctrl-x", Cut, Some("TextField")),
        KeyBinding::new("cmd-v", Paste, Some("TextField")),
        KeyBinding::new("ctrl-v", Paste, Some("TextField")),
        KeyBinding::new("cmd-z", Undo, Some("TextField")),
        KeyBinding::new("ctrl-z", Undo, Some("TextField")),
        KeyBinding::new("cmd-shift-z", Redo, Some("TextField")),
        KeyBinding::new("ctrl-shift-z", Redo, Some("TextField")),
        KeyBinding::new("ctrl-y", Redo, Some("TextField")),
    ]);
}

#[cfg(test)]
mod tests {
    use super::{next_word_boundary, previous_word_boundary};

    #[test]
    fn next_word_boundary_jumps_to_word_end() {
        assert_eq!(next_word_boundary("hello world", 0), 5);
        assert_eq!(next_word_boundary("hello world", 2), 5);
        assert_eq!(next_word_boundary("hello world", 5), 11);
        assert_eq!(next_word_boundary("hello world", 11), 11);
    }

    #[test]
    fn previous_word_boundary_jumps_to_word_start() {
        assert_eq!(previous_word_boundary("hello world", 11), 6);
        assert_eq!(previous_word_boundary("hello world", 7), 6);
        assert_eq!(previous_word_boundary("hello world", 6), 0);
        assert_eq!(previous_word_boundary("hello world", 0), 0);
    }

    #[test]
    fn word_boundary_skips_punctuation() {
        assert_eq!(next_word_boundary("foo, bar; baz", 3), 8);
        assert_eq!(previous_word_boundary("foo, bar; baz", 8), 5);
    }

    #[test]
    fn word_boundary_handles_underscore_as_word() {
        assert_eq!(next_word_boundary("foo_bar baz", 0), 7);
        assert_eq!(previous_word_boundary("foo_bar baz", 7), 0);
    }
}

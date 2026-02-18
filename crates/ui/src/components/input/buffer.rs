//! Lightweight input buffer for keyboard-first UIs.
//!
//! GPUI doesn't ship a full text-input widget for every use case. This buffer
//! provides basic editing primitives (cursor, selection, word deletion) that
//! can be reused across views.

#[derive(Debug, Clone, Default)]
pub struct InputBuffer {
    text: String,
    /// Cursor position as a byte index into `text`.
    cursor: usize,
    /// Selection anchor as a byte index into `text`.
    ///
    /// When present, the selection range is `min(anchor, cursor)..max(anchor, cursor)`.
    anchor: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorPlacement {
    /// Cursor is between `before` and `after` (no selection).
    Between,
    /// Cursor is at the start of the selected span.
    BeforeSelection,
    /// Cursor is at the end of the selected span.
    AfterSelection,
}

pub struct PlainRenderParts<'a> {
    pub before: &'a str,
    pub selected: Option<&'a str>,
    pub after: &'a str,
    pub cursor: CursorPlacement,
}

pub struct MaskedRenderParts {
    pub before: String,
    pub selected: Option<String>,
    pub after: String,
    pub cursor: CursorPlacement,
}

impl InputBuffer {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let cursor = text.len();
        Self {
            text,
            cursor,
            anchor: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.anchor = None;
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor = self.text.len();
        self.anchor = None;
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn selection_range_bytes(&self) -> Option<(usize, usize)> {
        let a = self.anchor?;
        if a == self.cursor {
            return None;
        }
        Some((a.min(self.cursor), a.max(self.cursor)))
    }

    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }

    pub fn select_all(&mut self) {
        if self.text.is_empty() {
            self.anchor = None;
            self.cursor = 0;
            return;
        }
        self.anchor = Some(0);
        self.cursor = self.text.len();
    }

    pub fn move_left(&mut self, select: bool) {
        if !select {
            if let Some((start, _end)) = self.selection_range_bytes() {
                self.cursor = start;
                self.anchor = None;
                return;
            }
        }
        self.prepare_selection(select);
        if let Some((new_cursor, _ch)) = prev_char(&self.text, self.cursor) {
            self.cursor = new_cursor;
        }
        self.cleanup_selection();
    }

    pub fn move_right(&mut self, select: bool) {
        if !select {
            if let Some((_start, end)) = self.selection_range_bytes() {
                self.cursor = end;
                self.anchor = None;
                return;
            }
        }
        self.prepare_selection(select);
        if let Some(new_cursor) = next_char_boundary(&self.text, self.cursor) {
            self.cursor = new_cursor;
        }
        self.cleanup_selection();
    }

    pub fn move_word_left(&mut self, select: bool) {
        if !select {
            if let Some((start, _end)) = self.selection_range_bytes() {
                self.cursor = start;
                self.anchor = None;
                return;
            }
        }
        self.prepare_selection(select);

        let mut i = self.cursor;
        while let Some((prev_idx, ch)) = prev_char(&self.text, i) {
            if !ch.is_whitespace() {
                break;
            }
            i = prev_idx;
        }
        while let Some((prev_idx, ch)) = prev_char(&self.text, i) {
            if ch.is_whitespace() {
                break;
            }
            i = prev_idx;
        }
        self.cursor = i;
        self.cleanup_selection();
    }

    pub fn move_word_right(&mut self, select: bool) {
        if !select {
            if let Some((_start, end)) = self.selection_range_bytes() {
                self.cursor = end;
                self.anchor = None;
                return;
            }
        }
        self.prepare_selection(select);

        let mut i = self.cursor;
        while let Some((ch, next_idx)) = next_char(&self.text, i) {
            if !ch.is_whitespace() {
                break;
            }
            i = next_idx;
        }
        while let Some((ch, next_idx)) = next_char(&self.text, i) {
            if ch.is_whitespace() {
                break;
            }
            i = next_idx;
        }
        self.cursor = i;
        self.cleanup_selection();
    }

    pub fn insert_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.delete_selection_if_any();
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
        self.anchor = None;
    }

    pub fn backspace(&mut self) {
        if self.delete_selection_if_any() {
            return;
        }
        let Some((start, _ch)) = prev_char(&self.text, self.cursor) else {
            return;
        };
        self.text.replace_range(start..self.cursor, "");
        self.cursor = start;
        self.anchor = None;
    }

    pub fn delete_word_back(&mut self) {
        if self.delete_selection_if_any() {
            return;
        }
        if self.cursor == 0 {
            return;
        }

        let mut start = self.cursor;
        while let Some((prev_idx, ch)) = prev_char(&self.text, start) {
            if !ch.is_whitespace() {
                break;
            }
            start = prev_idx;
        }

        while let Some((prev_idx, ch)) = prev_char(&self.text, start) {
            if ch.is_whitespace() {
                break;
            }
            start = prev_idx;
        }

        if start != self.cursor {
            self.text.replace_range(start..self.cursor, "");
            self.cursor = start;
        }
        self.anchor = None;
    }

    pub fn plain_render_parts(&self) -> PlainRenderParts<'_> {
        if let Some((sel_start, sel_end)) = self.selection_range_bytes() {
            let cursor = if self.cursor == sel_start {
                CursorPlacement::BeforeSelection
            } else {
                CursorPlacement::AfterSelection
            };
            PlainRenderParts {
                before: &self.text[..sel_start],
                selected: Some(&self.text[sel_start..sel_end]),
                after: &self.text[sel_end..],
                cursor,
            }
        } else {
            PlainRenderParts {
                before: &self.text[..self.cursor],
                selected: None,
                after: &self.text[self.cursor..],
                cursor: CursorPlacement::Between,
            }
        }
    }

    pub fn masked_render_parts(&self, mask: char) -> MaskedRenderParts {
        let cursor_chars = char_offset_at_byte(&self.text, self.cursor);
        let sel_chars = self.selection_range_bytes().map(|(s, e)| {
            (
                char_offset_at_byte(&self.text, s),
                char_offset_at_byte(&self.text, e),
            )
        });

        if let Some((sel_start, sel_end)) = sel_chars {
            let cursor = if cursor_chars == sel_start {
                CursorPlacement::BeforeSelection
            } else {
                CursorPlacement::AfterSelection
            };
            MaskedRenderParts {
                before: mask.to_string().repeat(sel_start),
                selected: Some(mask.to_string().repeat(sel_end.saturating_sub(sel_start))),
                after: mask
                    .to_string()
                    .repeat(self.text.chars().count().saturating_sub(sel_end)),
                cursor,
            }
        } else {
            let total = self.text.chars().count();
            MaskedRenderParts {
                before: mask.to_string().repeat(cursor_chars),
                selected: None,
                after: mask.to_string().repeat(total.saturating_sub(cursor_chars)),
                cursor: CursorPlacement::Between,
            }
        }
    }

    fn prepare_selection(&mut self, select: bool) {
        if select {
            if self.anchor.is_none() {
                self.anchor = Some(self.cursor);
            }
        } else {
            self.anchor = None;
        }
    }

    fn cleanup_selection(&mut self) {
        if let Some(a) = self.anchor
            && a == self.cursor
        {
            self.anchor = None;
        }
    }

    fn delete_selection_if_any(&mut self) -> bool {
        let Some((start, end)) = self.selection_range_bytes() else {
            return false;
        };
        self.text.replace_range(start..end, "");
        self.cursor = start;
        self.anchor = None;
        true
    }
}

fn next_char_boundary(text: &str, cursor: usize) -> Option<usize> {
    if cursor >= text.len() {
        return None;
    }
    let ch = text[cursor..].chars().next()?;
    Some(cursor + ch.len_utf8())
}

fn next_char(text: &str, cursor: usize) -> Option<(char, usize)> {
    if cursor >= text.len() {
        return None;
    }
    let ch = text[cursor..].chars().next()?;
    Some((ch, cursor + ch.len_utf8()))
}

fn prev_char(text: &str, cursor: usize) -> Option<(usize, char)> {
    if cursor == 0 {
        return None;
    }
    let (idx, ch) = text[..cursor].char_indices().last()?;
    Some((idx, ch))
}

fn char_offset_at_byte(text: &str, byte: usize) -> usize {
    debug_assert!(byte <= text.len());
    debug_assert!(text.is_char_boundary(byte));
    text[..byte].chars().count()
}

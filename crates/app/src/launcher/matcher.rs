//! Fuzzy ranking shared by every view that leaves matching to the launcher.

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use super::item::Item;

/// An item that matched, with the byte offsets of its matched title chars.
pub struct Ranked {
    pub index: usize,
    pub highlights: Vec<usize>,
}

pub struct Ranker {
    matcher: Matcher,
    buf: Vec<char>,
}

impl Default for Ranker {
    fn default() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT),
            buf: Vec::new(),
        }
    }
}

impl Ranker {
    /// Matching items, best first. A title match outranks one found only in
    /// the subtitle or keywords; ties keep the view's order.
    pub fn rank(&mut self, query: &str, items: &[Item]) -> Vec<Ranked> {
        let query = query.trim();
        if query.is_empty() {
            return (0..items.len())
                .map(|index| Ranked {
                    index,
                    highlights: Vec::new(),
                })
                .collect();
        }

        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let mut indices = Vec::new();
        let mut scored: Vec<(u32, Ranked)> = Vec::new();

        for (index, item) in items.iter().enumerate() {
            indices.clear();
            let title = Utf32Str::new(&item.title, &mut self.buf);
            if let Some(score) = pattern.indices(title, &mut self.matcher, &mut indices) {
                let highlights = char_to_byte_offsets(&item.title, &mut indices);
                scored.push((score.saturating_mul(2), Ranked { index, highlights }));
                continue;
            }

            let rest = [item.subtitle.as_deref(), item.keywords.as_deref()];
            let haystack = rest.iter().flatten().copied().collect::<Vec<_>>().join(" ");
            if haystack.is_empty() {
                continue;
            }
            let haystack = Utf32Str::new(&haystack, &mut self.buf);
            if let Some(score) = pattern.score(haystack, &mut self.matcher) {
                scored.push((
                    score,
                    Ranked {
                        index,
                        highlights: Vec::new(),
                    },
                ));
            }
        }

        scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
        scored.into_iter().map(|(_, ranked)| ranked).collect()
    }
}

fn char_to_byte_offsets(text: &str, chars: &mut [u32]) -> Vec<usize> {
    chars.sort_unstable();
    let mut wanted = chars.iter().copied().peekable();
    let mut offsets = Vec::with_capacity(chars.len());
    for (ix, (byte, _)) in text.char_indices().enumerate() {
        if wanted.peek() == Some(&(ix as u32)) {
            offsets.push(byte);
            wanted.next();
        }
    }
    offsets
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles(ranker: &mut Ranker, query: &str, items: &[Item]) -> Vec<String> {
        ranker
            .rank(query, items)
            .into_iter()
            .map(|r| items[r.index].title.to_string())
            .collect()
    }

    fn items() -> Vec<Item> {
        vec![
            Item::new("gimp", "GIMP").subtitle("Image editor"),
            Item::new("ff", "Firefox Browser").subtitle("Web browser"),
            Item::new("kitty", "Kitty").keywords("terminal"),
        ]
    }

    #[test]
    fn fuzzy_query_finds_scattered_letters() {
        assert_eq!(
            titles(&mut Ranker::default(), "fxbr", &items()),
            ["Firefox Browser"]
        );
    }

    #[test]
    fn subtitle_and_keywords_match_without_highlights() {
        let mut ranker = Ranker::default();
        let items = items();
        let ranked = ranker.rank("terminal", &items);
        assert_eq!(ranked.len(), 1);
        assert_eq!(items[ranked[0].index].title.as_ref(), "Kitty");
        assert!(ranked[0].highlights.is_empty());
    }

    #[test]
    fn title_matches_rank_above_subtitle_matches() {
        let items = vec![
            Item::new("a", "Files").subtitle("browser"),
            Item::new("b", "Browser"),
        ];
        assert_eq!(
            titles(&mut Ranker::default(), "browser", &items),
            ["Browser", "Files"]
        );
    }

    #[test]
    fn empty_query_keeps_every_item_in_order() {
        assert_eq!(
            titles(&mut Ranker::default(), "  ", &items()),
            ["GIMP", "Firefox Browser", "Kitty"]
        );
    }

    #[test]
    fn highlights_are_byte_offsets() {
        let items = vec![Item::new("a", "Ärger")];
        let ranked = Ranker::default().rank("rg", &items);
        // `Ä` is two bytes, so `r` starts at byte 2.
        assert_eq!(ranked[0].highlights, [2, 3]);
    }
}

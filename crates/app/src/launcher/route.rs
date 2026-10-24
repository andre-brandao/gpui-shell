//! Which view owns the input, and what it searches for.

use super::view::ViewMeta;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    /// Index into the registry.
    pub view: usize,
    /// The input with the view's prefix removed.
    pub query: String,
}

/// The longest matching prefix wins; a stray prefix character goes to help,
/// anything else to the default view.
pub fn route(input: &str, metas: &[ViewMeta]) -> Route {
    let input = input.trim();
    let fallback = metas
        .iter()
        .position(|meta| meta.default)
        .or_else(|| help_index(metas))
        .unwrap_or(0);

    if input.is_empty() {
        return Route {
            view: fallback,
            query: String::new(),
        };
    }

    let best = metas
        .iter()
        .enumerate()
        .filter(|(_, meta)| !meta.prefix.is_empty() && input.starts_with(meta.prefix.as_ref()))
        .max_by_key(|(_, meta)| meta.prefix.len());
    if let Some((view, meta)) = best {
        return Route {
            view,
            query: input[meta.prefix.len()..].trim_start().to_string(),
        };
    }

    let first = input.chars().next();
    let looks_like_prefix = metas.iter().any(|meta| meta.prefix.chars().next() == first);
    if looks_like_prefix && let Some(help) = help_index(metas) {
        return Route {
            view: help,
            query: input.to_string(),
        };
    }

    Route {
        view: fallback,
        query: input.to_string(),
    }
}

fn help_index(metas: &[ViewMeta]) -> Option<usize> {
    metas.iter().position(|meta| meta.id == "help")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui::IconName;

    fn meta(id: &'static str, prefix: &'static str, default: bool) -> ViewMeta {
        ViewMeta {
            id,
            prefix: prefix.into(),
            name: id,
            icon: IconName::Layers,
            description: "",
            default,
            in_help: true,
        }
    }

    fn metas() -> Vec<ViewMeta> {
        vec![
            meta("apps", "@", true),
            meta("services", ";s", false),
            meta("wallpaper", ";wp", false),
            meta("help", "?", false),
        ]
    }

    #[test]
    fn plain_text_goes_to_the_default_view() {
        assert_eq!(
            route("fire", &metas()),
            Route {
                view: 0,
                query: "fire".into()
            }
        );
    }

    #[test]
    fn longest_prefix_wins() {
        assert_eq!(
            route(";wp  forest", &metas()),
            Route {
                view: 2,
                query: "forest".into()
            }
        );
        assert_eq!(route(";s", &metas()).view, 1);
    }

    #[test]
    fn an_unknown_prefix_opens_help_with_the_whole_input() {
        assert_eq!(
            route(";x", &metas()),
            Route {
                view: 3,
                query: ";x".into()
            }
        );
    }

    #[test]
    fn empty_input_without_a_default_falls_back_to_help() {
        let metas: Vec<_> = metas().into_iter().filter(|m| !m.default).collect();
        assert_eq!(route("", &metas).view, 2);
    }
}

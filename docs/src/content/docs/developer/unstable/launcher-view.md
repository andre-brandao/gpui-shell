---
title: Adding a launcher view
description: Write a launcher view against the current, unstable Rust API.
---

> **Unstable.** The `LauncherView` trait can still change before 1.0, and
> views live in-tree and are written in Rust. The data path (items in,
> activations out) is the part a future plugin API will build on.

We'll build a `Links` view: type `;l`, filter a list of saved links, press
Enter to open one.

```toml
[launcher.views.links]
prefix = ";l"
links = [
  { name = "GPUi Shell", url = "https://github.com/andre-brandao/gpui-shell" },
  { name = "GPUI", url = "https://gpui.rs" },
]
```

## How a view works

The launcher owns the window, the input, the selection and the rows. A view
is a GPUI entity, picked by its prefix; the text after the prefix is its
query.

1. When the query changes, the launcher calls `items(query)` once. The view
   returns plain `Item`s: id, title, subtitle, icon, keywords, actions.
2. The launcher fuzzy-ranks them, highlights the matched letters and draws
   only the rows on screen.
3. Enter, a click, or Shift+Enter calls `activate(item, action)`, which
   returns `Close`, `Stay` or `SetQuery(..)`.

A view never filters per row, wires a click, or touches the input. When its
own data changes (a service updated, a scan finished), it emits
`ViewEvent::ItemsChanged` and the launcher asks again, keeping the cursor on
the same item. `cx.notify()` on the view redraws the launcher.

Views are built the first time their prefix is typed, so a view that
subscribes to a service in its constructor starts it only when someone opens
the view.

## 1. Create the view

Each view is a directory in `crates/app/src/launcher/views/`.

```rust
// crates/app/src/launcher/views/links/config.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    /// Shown in the list and matched against the query.
    pub name: String,
    /// Opened with `xdg-open`.
    pub url: String,
}

/// `[launcher.views.links]`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LinksConfig {
    /// Prefix that opens the view.
    pub prefix: String,
    /// Saved links, in display order.
    pub links: Vec<Link>,
}

impl Default for LinksConfig {
    fn default() -> Self {
        Self {
            prefix: ";l".into(),
            links: Vec::new(),
        }
    }
}
```

```rust
// crates/app/src/launcher/views/links/mod.rs
//! Links view for opening saved URLs.

pub mod config;

use std::process::Command;

use gpui::{Context, EventEmitter};
use ui::IconName;

use self::config::{Link, LinksConfig};
use crate::launcher::item::{Item, ItemIcon};
use crate::launcher::view::{Items, LauncherView, Outcome, ViewDescriptor, ViewEvent, ViewMeta};

pub fn descriptor(config: &LinksConfig) -> ViewDescriptor {
    let links = config.links.clone();
    ViewDescriptor::new(
        ViewMeta {
            id: "links",
            prefix: config.prefix.clone().into(),
            name: "Links",
            icon: IconName::Link,
            description: "Open a saved link",
            default: false,
            in_help: true,
        },
        move |_cx| LinksView { links: links.clone() },
    )
}

pub struct LinksView {
    links: Vec<Link>,
}

impl EventEmitter<ViewEvent> for LinksView {}

impl LauncherView for LinksView {
    fn items(&mut self, _query: &str, _cx: &mut Context<Self>) -> Items {
        Items::Ready(
            self.links
                .iter()
                .map(|link| {
                    Item::new(link.url.clone(), link.name.clone())
                        .subtitle(link.url.clone())
                        .icon(ItemIcon::Icon(IconName::Link.into()))
                        .action("open", "Open")
                })
                .collect(),
        )
    }

    fn activate(&mut self, item: &Item, _action: &str, _cx: &mut Context<Self>) -> Outcome {
        let mut open = Command::new("xdg-open");
        open.arg(item.id.as_ref());
        let _ = services::spawn_reaped(open, "xdg-open");
        Outcome::Close
    }
}
```

That's the whole view. Matching, highlighting, clicks, key hints (`Open
Enter`, built from the item's actions) and the empty list are the
launcher's job.

## Going further

Everything below is an optional method on `LauncherView`.

| Want | Use |
| --- | --- |
| Filter yourself, keep your order (a shell command) | `matching() -> Matching::View` |
| Slow source (files, history, network) | return `Items::Pending(task)`; the old items stay until it resolves |
| A second action on Shift+Enter | a second `.action(..)` on the item |
| Your own row or card | `render_row`, plus `row_height` if it isn't the default |
| A grid of cards (wallpapers) | `body() -> Body::Grid { columns }`; arrows move in 2D |
| No list at all, fully interactive | `body() -> Body::Custom(cx.entity().into())` and `impl Render` |
| Toggles or a summary above the list | `render_header`, with `cx.listener` for clicks |
| Text when there are no items | `empty_text` |
| Claim Enter or an arrow key (a custom body) | `on_key` |
| Custom footer hints | `footer_hints` |

The rule for `render_row`: draw the row, don't wire its click. The launcher
wraps every row in one clickable element that calls `activate`, so click and
Enter always do the same thing. For images, `launcher::row::thumbnail_image`
returns a cached, downscaled thumbnail and notifies the view when it lands.

The calculator (`views/calculator`) is the smallest custom body and web
search (`views/web`) a fuller one; the wallpaper view (`views/wallpaper`) is
the reference for a grid.

## 2. Register it

1. **`launcher/views/mod.rs`**: `pub mod links;` and
   `links::descriptor(&config.links)` in `registry`.
2. **`launcher/config.rs`**: add `pub links: LinksConfig` to `ViewsConfig`,
   which makes `[launcher.views.links]` load.

Prefixes must be unique. Check `?` in the launcher for the ones taken.

## 3. Try it

```bash
cargo run -- --input ";l"
```

Before the PR, give every config field a doc comment. You don't need to edit
`reference/launcher.md`: the Claude Docs workflow (`claude-docs.yml`)
updates the reference pages from changed `config.rs` files when `dev` merges
into `main`. Keep the config in a file named `config.rs` so it's picked up.

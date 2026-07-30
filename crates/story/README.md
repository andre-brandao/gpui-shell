# story

Two views of the same component set. Neither is the shell — both are plain
windows, which is the point: the shell itself is a Wayland layer-shell client
spread across a bar, a dock and a handful of transient popups, so "does this
widget look right under this palette" is nearly impossible to answer in situ.

```sh
cargo run -p story                 # gallery: one component per page
cargo run -p story --bin showcase  # showcase: everything on one page
```

**`story`** isolates one component at a time with every variant laid out, and
a sidebar to move between them. Use it when working *on* a component.

**`showcase`** puts them all on one page, wired to real state — you can click
the checkboxes, drag the sliders, open the modal. Use it when judging a
palette: contrast collisions between neighbouring components only show up
when they're actually neighbours.

Both carry the same Base16 scheme picker (`crates/story/src/schemes.rs`),
deliberately including a light and a low-contrast palette. Deriving ~50 theme
tokens from 16 colors is only worth anything if the derivation holds on
palettes it wasn't tuned against.

Not part of the shell binary — the workspace sets
`default-members = ["crates/app"]`, so a plain `cargo build` skips it.

# Icon bundle

Every SVG here comes from [Lucide](https://lucide.dev), fetched from
`lucide-icons/lucide@main`. They are unmodified upstream files: 24x24,
`stroke-width="2"`, `stroke="currentColor"` so `Icon` can tint them from
the theme.

Licensed under ISC — see [`LICENSE`](./LICENSE), which also carries the MIT
notice for the subset Lucide derives from Feather.

## Adding an icon

1. Fetch it from upstream rather than hand-writing the path data:

   ```sh
   curl -sSf https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/<kebab-name>.svg \
     -o crates/assets/icons/<snake_name>.svg
   ```

2. Add the matching variant to `IconName` in `crates/ui/src/components/icon.rs`
   (`strum` maps `PascalCase` to the `snake_case` filename).

`crates/ui/tests/icon_assets.rs` fails if a name has no file, or a file has
no name — a mismatch is otherwise invisible, since gpui silently paints
nothing when an icon path doesn't resolve.

No rebuild step to remember: `build.rs` watches this directory, so adding a
file here invalidates the crate. Without that, `rust-embed` only leaves
cargo tracking the files it embedded last time, and a new icon resolves on
disk (so the tests pass) while the shipped binary keeps a bundle that never
had it.

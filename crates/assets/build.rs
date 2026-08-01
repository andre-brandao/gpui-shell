//! Make cargo watch the icon bundle.
//!
//! `rust-embed`'s proc macro only leaves cargo tracking the files it
//! embedded *last* time, so a newly added SVG is invisible: the crate is
//! not rebuilt, the macro never re-runs, and the binary silently ships a
//! bundle without it. That surfaces at runtime as
//! `could not find asset at path "icons/<new>.svg"` - the file is right
//! there on disk, which makes it a confusing failure to chase.
//!
//! Watching the directory closes the gap: any add, remove or edit under
//! `icons/` invalidates this crate.

fn main() {
    println!("cargo:rerun-if-changed=icons");
}

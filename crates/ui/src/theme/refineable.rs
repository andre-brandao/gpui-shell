//! `refineable!` - declarative companion-struct generator.
//!
//! Expands one invocation into the partial-override twin of a base struct:
//! a `*Refinement` with every field wrapped in [`Option`], plus `refine()`,
//! `from_full()`, and `is_empty()` methods.
//!
//! This is what lets a user's `theme.toml` override a single token on top
//! of a whole palette:
//!
//! ```toml
//! [base16]
//! base00 = "#1e1e2e"
//! # ...
//!
//! [colors]           # everything omitted here falls through to the
//! accent = "#f38ba8" # palette-derived value
//! ```
//!
//! Base structs stay hand-written so their doc comments and field grouping
//! survive. The macro only generates the refinement companion and its
//! glue - keeping a single source of truth for field names is the caller's
//! responsibility.

/// Generic `skip_serializing_if` helper for any refinement struct.
///
/// Every struct emitted by [`refineable!`] derives `PartialEq + Default`,
/// so a refinement "is empty" exactly when it equals its default value.
pub(crate) fn is_default<T: ::std::cmp::PartialEq + ::std::default::Default>(v: &T) -> bool {
    *v == T::default()
}

macro_rules! refineable {
    (
        $(#[$sm:meta])*
        $svis:vis struct $rname:ident refines $base:ident {
            colors { $($field:ident),* $(,)? }
            $( nested { $($nfield:ident : $nref:ident),* $(,)? } )?
        }
    ) => {
        $(#[$sm])*
        #[derive(
            ::std::fmt::Debug,
            ::std::clone::Clone,
            ::std::marker::Copy,
            ::std::default::Default,
            ::std::cmp::PartialEq,
            ::serde::Deserialize,
            ::serde::Serialize,
        )]
        #[serde(default, deny_unknown_fields)]
        $svis struct $rname {
            $(
                #[serde(
                    default,
                    with = "crate::theme::color_string::opt",
                    skip_serializing_if = "::std::option::Option::is_none",
                )]
                pub $field: ::std::option::Option<::gpui::Hsla>,
            )*
            $($(
                #[serde(default, skip_serializing_if = "crate::theme::refineable::is_default")]
                pub $nfield: $nref,
            )*)?
        }

        impl $rname {
            /// Copy every `Some(..)` field of `self` onto `base`, leaving
            /// the rest of `base` untouched. Nested refinements recurse.
            pub fn refine(self, base: &mut $base) {
                $(
                    if let Some(v) = self.$field {
                        base.$field = v;
                    }
                )*
                $($(
                    self.$nfield.refine(&mut base.$nfield);
                )*)?
            }

            /// Wrap every field of `base` in `Some`. Used to dump a fully
            /// resolved theme - nothing is skipped on serialize.
            #[allow(dead_code)]
            pub fn from_full(base: &$base) -> Self {
                Self {
                    $( $field: ::std::option::Option::Some(base.$field), )*
                    $($( $nfield: <$nref>::from_full(&base.$nfield), )*)?
                }
            }

            /// True when no field is overridden. Lets parent refinements
            /// skip empty sub-objects on serialize.
            #[allow(dead_code)]
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }
        }
    };
}

//! The icon type config files speak.
//!
//! A config outlives any one icon set, so this layer is deliberately
//! forgiving: a value it can't resolve degrades to the caller's built-in
//! icon with a warning, rather than failing the TOML parse and taking every
//! other setting in the file down with it.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ui::{IconName, IconSource};

/// An icon named in a config file.
///
/// Two spellings:
///
/// - a bare name from the embedded set - `icon = "layers"`, matching the
///   file stems under `crates/assets/icons/`;
/// - a path to your own file - `icon = "~/.config/gpuishell/icons/mine.svg"`.
///   `.svg` stays tintable by the theme; PNG/JPG render as-is.
///
/// Anything with a `/` or a leading `~` is a path; everything else is a
/// name. That means a name can never be mistaken for a path, and a relative
/// path always needs a `./` prefix - a deliberate trade so the common case
/// (a bare name) needs no punctuation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigIcon {
    Embedded(IconName),
    /// Held as an `Arc<Path>` rather than a `PathBuf` because [`Self::source`]
    /// runs once per widget per frame: an `Arc` clone is a refcount bump,
    /// where rebuilding one from a `PathBuf` would copy the path every time.
    File(Arc<Path>),
}

impl ConfigIcon {
    /// Resolve to something [`ui::Icon`] can render.
    pub fn source(&self) -> IconSource {
        match self {
            Self::Embedded(name) => IconSource::Embedded(*name),
            Self::File(path) => IconSource::External(path.clone()),
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }

        if raw.starts_with('~') || raw.contains('/') {
            return Some(Self::File(expand_home(raw).into()));
        }

        raw.parse().ok().map(Self::Embedded)
    }
}

/// Expand a leading `~` against `$HOME`. Left alone when `$HOME` is unset -
/// the path then simply fails to load, which is the same outcome as any
/// other bad path and needs no separate branch.
fn expand_home(raw: &str) -> PathBuf {
    match raw.strip_prefix("~/") {
        Some(rest) => match std::env::var_os("HOME") {
            Some(home) => PathBuf::from(home).join(rest),
            None => PathBuf::from(raw),
        },
        None => PathBuf::from(raw),
    }
}

impl Serialize for ConfigIcon {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Embedded(name) => name.serialize(serializer),
            Self::File(path) => serializer.serialize_str(&path.to_string_lossy()),
        }
    }
}

/// Deserialize a configured icon, tolerating values we can't resolve.
///
/// An old Nerd Font glyph left over from before the SVG switch, an empty
/// string, a typo - all yield `None`, and the caller falls back to its
/// built-in icon. See the module docs for why this is lenient rather than
/// strict.
pub fn deserialize_lenient<'de, D>(deserializer: D) -> Result<Option<ConfigIcon>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(raw) = Option::<String>::deserialize(deserializer)? else {
        return Ok(None);
    };

    Ok(ConfigIcon::parse(&raw).or_else(|| {
        tracing::warn!("Unusable icon {raw:?} in config, using the built-in default");
        None
    }))
}

/// The source to render for a configured icon, falling back to `fallback`
/// when the config said nothing usable.
pub fn source_or(icon: Option<&ConfigIcon>, fallback: IconName) -> IconSource {
    icon.map_or(IconSource::Embedded(fallback), ConfigIcon::source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_name_resolves_against_the_embedded_set() {
        assert_eq!(
            ConfigIcon::parse("battery_low"),
            Some(ConfigIcon::Embedded(IconName::BatteryLow))
        );
    }

    #[test]
    fn anything_with_a_separator_is_a_path() {
        assert_eq!(
            ConfigIcon::parse("./icons/mine.svg"),
            Some(ConfigIcon::File(Arc::from(Path::new("./icons/mine.svg"))))
        );
    }

    #[test]
    fn a_leading_tilde_expands_against_home() {
        // SAFETY: single-threaded test, and the value is restored by the
        // process exiting - no other test reads HOME.
        unsafe { std::env::set_var("HOME", "/home/tester") };

        assert_eq!(
            ConfigIcon::parse("~/icons/mine.svg"),
            Some(ConfigIcon::File(Arc::from(Path::new(
                "/home/tester/icons/mine.svg"
            ))))
        );
    }

    /// A stale glyph must degrade, not fail - a strict parse here would take
    /// every other setting in the file down with it.
    #[test]
    fn an_unresolvable_value_degrades_to_none() {
        assert_eq!(ConfigIcon::parse("\u{f003b}"), None);
        assert_eq!(ConfigIcon::parse("not_an_icon"), None);
        assert_eq!(ConfigIcon::parse("   "), None);
    }
}

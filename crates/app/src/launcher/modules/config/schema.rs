//! How a config struct describes itself to the editor: a title, its
//! [`Field`]s with their serde keys (optionally in [`Group`]s), and
//! optionally a page of its own.
//!
//! A [`ConfigPath`] ties a section to its place in the live [`Config`] and in
//! `config.toml`, so a page deep in the tree can read and write without
//! knowing its parents.

use std::fmt::Write as _;
use std::rc::Rc;

use gpui::{AnyElement, App, AppContext};
use serde::Serialize;
use toml_edit::ser::ValueSerializer;
use ui::IconName;

use super::section_page::{SectionPage, hoisted_item, primary_action, render_form};
use crate::config::Config;
use crate::config::store::{self, Edit};
use crate::launcher::action::Action;
use crate::launcher::content::Item;
use crate::launcher::page::Page;

pub trait Editable: Serialize + Default + Clone + 'static {
    fn title() -> &'static str;

    /// Fields in display order.
    fn fields() -> Vec<Field<Self>>;

    /// Fields under headings. One unnamed group by default; a struct that
    /// overrides this builds `fields()` by flattening it.
    fn groups() -> Vec<Group<Self>> {
        vec![Group::new("", Self::fields())]
    }

    /// A page that replaces the generic field list; `None` gets
    /// [`SectionPage`].
    fn page(_path: ConfigPath<Self>, _cx: &mut App) -> Option<Page> {
        None
    }

    /// Rows that are not fields, listed last: shortcuts to related pages.
    fn shortcuts(_cx: &App) -> Vec<Item> {
        Vec::new()
    }
}

/// Serialize a field the way `config.toml` spells it, enum renames included.
pub fn to_value<T: Serialize + ?Sized>(value: &T) -> toml_edit::Value {
    value
        .serialize(ValueSerializer::new())
        .unwrap_or_else(|err| {
            tracing::error!("config field does not serialize to TOML: {err}");
            toml_edit::Value::from("")
        })
}

/// Typed access to one field of `S`. Plain fn pointers, so a lens is `Copy`
/// and `'static` without captures.
pub struct Lens<S, T> {
    pub get: fn(&S) -> T,
    pub set: fn(&mut S, T),
    /// Put the struct default back, which for an `Option` is `None`, not
    /// the number the editor shows for it.
    pub reset: fn(&mut S),
    pub to_toml: fn(&S) -> toml_edit::Value,
}

impl<S, T> Clone for Lens<S, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, T> Copy for Lens<S, T> {}

/// `lens!(field)` or `lens!(outer.inner)`; `lens!(field as f64)` for numeric
/// fields, which the editor handles as `f64` but writes with their own type;
/// `lens!(field or 1.0)` for an `Option` number that shows as `1.0` while
/// unset and goes back to unset on reset. Expands inside an `impl Editable`,
/// where `Self` is the struct.
macro_rules! lens {
    ($($field:ident).+ as f64) => {
        $crate::launcher::modules::config::schema::Lens {
            get: |s: &Self| s.$($field).+ as f64,
            set: |s: &mut Self, v: f64| s.$($field).+ = v as _,
            reset: |s: &mut Self| s.$($field).+ = Self::default().$($field).+,
            to_toml: |s: &Self| $crate::launcher::modules::config::schema::to_value(&s.$($field).+),
        }
    };
    ($($field:ident).+ or $unset:expr) => {
        $crate::launcher::modules::config::schema::Lens {
            get: |s: &Self| s.$($field).+.map_or($unset, |v| v as f64),
            set: |s: &mut Self, v: f64| s.$($field).+ = Some(v as _),
            reset: |s: &mut Self| s.$($field).+ = None,
            to_toml: |s: &Self| {
                $crate::launcher::modules::config::schema::to_value(
                    &s.$($field).+.map_or($unset, |v| v as f64),
                )
            },
        }
    };
    ($($field:ident).+) => {
        $crate::launcher::modules::config::schema::Lens {
            get: |s: &Self| s.$($field).+.clone(),
            set: |s: &mut Self, v| s.$($field).+ = v,
            reset: |s: &mut Self| s.$($field).+ = Self::default().$($field).+,
            to_toml: |s: &Self| $crate::launcher::modules::config::schema::to_value(&s.$($field).+),
        }
    };
}
pub(crate) use lens;

/// What a free-text field holds, for validation and preview.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TextHint {
    Plain,
    Path,
    Command,
    Strftime,
    /// A launcher prefix: short, no whitespace.
    Prefix,
}

impl TextHint {
    pub fn validate(self, text: &str) -> Result<(), String> {
        match self {
            TextHint::Prefix if text.is_empty() => Err("A prefix cannot be empty".into()),
            TextHint::Prefix if text.chars().any(char::is_whitespace) => {
                Err("A prefix cannot contain spaces".into())
            }
            TextHint::Strftime if text.is_empty() => Err("A format cannot be empty".into()),
            TextHint::Strftime
                if chrono::format::StrftimeItems::new(text)
                    .any(|item| matches!(item, chrono::format::Item::Error)) =>
            {
                Err("Not a valid strftime format".into())
            }
            _ => Ok(()),
        }
    }

    /// What the value would show, where that is not obvious from the text.
    pub fn preview(self, text: &str) -> Option<String> {
        match self {
            // chrono's `to_string()` panics on a bad specifier; `write!` returns `Err`.
            TextHint::Strftime => {
                let mut out = String::new();
                write!(out, "{}", chrono::Local::now().format(text)).ok()?;
                Some(out)
            }
            _ => None,
        }
    }
}

pub struct ChoiceOption<S> {
    pub label: &'static str,
    pub apply: Rc<dyn Fn(&mut S)>,
    pub value: toml_edit::Value,
}

/// Opens a page for a nested or custom field.
pub type Open<S> = Rc<dyn Fn(ConfigPath<S>, &mut App) -> Page>;

/// Applies a preset: the chosen option's index, with the section's path.
pub type Commit<S> = Rc<dyn Fn(&ConfigPath<S>, usize, &mut App)>;

/// Builds the row for a child with a single field, titled after its parent.
pub type Hoist<S> = Rc<dyn Fn(&Field<S>, &ConfigPath<S>, &App) -> Item>;

/// Every child field's primary action, for the parent row's menu.
pub type Quick<S> = Rc<dyn Fn(&ConfigPath<S>) -> Vec<Action>>;

/// A picture of what a field (or bundle) controls, drawn above its controls.
pub type Preview<S> = Rc<dyn Fn(&ConfigPath<S>, &App) -> AnyElement>;

/// Draws a child struct's fields inline, `depth` levels down.
pub type Form<S> = Rc<dyn Fn(&ConfigPath<S>, usize, &App) -> AnyElement>;

pub enum Editor<S> {
    Toggle(Lens<S, bool>),
    Number {
        lens: Lens<S, f64>,
        min: f64,
        max: f64,
        step: f64,
        integer: bool,
        unit: Option<&'static str>,
    },
    Text {
        lens: Lens<S, String>,
        hint: TextHint,
    },
    Choice {
        current: Rc<dyn Fn(&S) -> usize>,
        options: Rc<[ChoiceOption<S>]>,
        /// A preset: choosing writes several keys (and maybe the theme)
        /// through this instead of one key through the field's lens.
        commit: Option<Commit<S>>,
    },
    /// A child struct: its page on Enter, its form in the detail pane, and
    /// its fields' actions in the menu, so one row edits the whole child.
    Nested {
        open: Open<S>,
        reset: Rc<dyn Fn(&mut S)>,
        form: Form<S>,
        quick: Quick<S>,
        summary: Rc<dyn Fn(&S) -> String>,
        modified: Rc<dyn Fn(&S) -> bool>,
        hoist: Option<Hoist<S>>,
        /// How many settings the child has; none means nothing to show.
        count: usize,
    },
    Custom {
        open: Open<S>,
    },
    /// Several fields of this same struct shown as one row: the form holds
    /// them all, the menu lists each one's action.
    Bundle(Rc<[Field<S>]>),
}

impl<S> Clone for Editor<S> {
    fn clone(&self) -> Self {
        match self {
            Editor::Toggle(lens) => Editor::Toggle(*lens),
            Editor::Number {
                lens,
                min,
                max,
                step,
                integer,
                unit,
            } => Editor::Number {
                lens: *lens,
                min: *min,
                max: *max,
                step: *step,
                integer: *integer,
                unit: *unit,
            },
            Editor::Text { lens, hint } => Editor::Text {
                lens: *lens,
                hint: *hint,
            },
            Editor::Choice {
                current,
                options,
                commit,
            } => Editor::Choice {
                current: current.clone(),
                options: options.clone(),
                commit: commit.clone(),
            },
            Editor::Nested {
                open,
                reset,
                form,
                quick,
                summary,
                modified,
                hoist,
                count,
            } => Editor::Nested {
                open: open.clone(),
                reset: reset.clone(),
                form: form.clone(),
                quick: quick.clone(),
                summary: summary.clone(),
                modified: modified.clone(),
                hoist: hoist.clone(),
                count: *count,
            },
            Editor::Custom { open } => Editor::Custom { open: open.clone() },
            Editor::Bundle(fields) => Editor::Bundle(fields.clone()),
        }
    }
}

impl<S> Editor<S> {
    /// A nested child with nothing to edit; rows and forms skip it.
    pub fn is_empty(&self) -> bool {
        match self {
            Editor::Nested { count, .. } => *count == 0,
            Editor::Bundle(fields) => fields.is_empty(),
            _ => false,
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            Editor::Toggle(_) => IconName::Check,
            Editor::Number { .. } => IconName::Hash,
            Editor::Text { .. } => IconName::Pencil,
            Editor::Choice { .. } => IconName::ListTree,
            Editor::Nested { .. } => IconName::ChevronRight,
            Editor::Custom { .. } => IconName::Layout,
            Editor::Bundle(_) => IconName::Layers,
        }
    }
}

/// Fields under one heading.
pub struct Group<S> {
    pub title: &'static str,
    pub fields: Vec<Field<S>>,
}

impl<S> Group<S> {
    pub fn new(title: &'static str, fields: Vec<Field<S>>) -> Self {
        Self { title, fields }
    }
}

/// `fields()` for a struct that defines `groups()`.
pub fn flatten<S>(groups: Vec<Group<S>>) -> Vec<Field<S>> {
    groups.into_iter().flat_map(|group| group.fields).collect()
}

pub struct Field<S> {
    /// The serde key, dotted for a field of a flat child struct.
    pub key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub icon: Option<IconName>,
    pub preview: Option<Preview<S>>,
    pub editor: Editor<S>,
}

impl<S> Clone for Field<S> {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            label: self.label,
            description: self.description,
            icon: self.icon,
            preview: self.preview.clone(),
            editor: self.editor.clone(),
        }
    }
}

impl<S: Editable> Field<S> {
    fn new(key: &'static str, label: &'static str, editor: Editor<S>) -> Self {
        Self {
            key,
            label,
            description: "",
            icon: None,
            preview: None,
            editor,
        }
    }

    pub fn toggle(key: &'static str, label: &'static str, lens: Lens<S, bool>) -> Self {
        Self::new(key, label, Editor::Toggle(lens))
    }

    /// A float; `.range()`, `.step()` and `.unit()` refine it.
    pub fn number(key: &'static str, label: &'static str, lens: Lens<S, f64>) -> Self {
        Self::new(
            key,
            label,
            Editor::Number {
                lens,
                min: 0.0,
                max: 100.0,
                step: 1.0,
                integer: false,
                unit: None,
            },
        )
    }

    pub fn integer(key: &'static str, label: &'static str, lens: Lens<S, f64>) -> Self {
        let mut field = Self::number(key, label, lens);
        if let Editor::Number { integer, .. } = &mut field.editor {
            *integer = true;
        }
        field
    }

    pub fn text(key: &'static str, label: &'static str, lens: Lens<S, String>) -> Self {
        Self::new(
            key,
            label,
            Editor::Text {
                lens,
                hint: TextHint::Plain,
            },
        )
    }

    /// One option per value in `all`, written with the enum's own serde
    /// spelling.
    pub fn choice<E>(
        key: &'static str,
        label: &'static str,
        lens: Lens<S, E>,
        all: &[E],
        name: fn(E) -> &'static str,
    ) -> Self
    where
        E: Serialize + Clone + PartialEq + 'static,
    {
        let all: Rc<[E]> = all.into();
        let options = all
            .iter()
            .map(|value| {
                let chosen = value.clone();
                ChoiceOption {
                    label: name(value.clone()),
                    apply: Rc::new(move |s: &mut S| (lens.set)(s, chosen.clone())),
                    value: to_value(value),
                }
            })
            .collect();
        let current = Rc::new(move |s: &S| {
            let value = (lens.get)(s);
            all.iter().position(|e| *e == value).unwrap_or(0)
        });
        Self::new(
            key,
            label,
            Editor::Choice {
                current,
                options,
                commit: None,
            },
        )
    }

    /// A choice between presets that each set several keys: `current` says
    /// which one the struct is in (out of range: none, shown as "Custom")
    /// and `commit` applies the chosen one. Rendered like a choice - buttons
    /// in the detail pane, Enter cycles.
    pub fn presets(
        key: &'static str,
        label: &'static str,
        labels: &[&'static str],
        current: impl Fn(&S) -> usize + 'static,
        commit: impl Fn(&ConfigPath<S>, usize, &mut App) + 'static,
    ) -> Self {
        let options = labels
            .iter()
            .map(|label| ChoiceOption {
                label,
                apply: Rc::new(|_: &mut S| {}),
                value: toml_edit::Value::from(*label),
            })
            .collect();
        Self::new(
            key,
            label,
            Editor::Choice {
                current: Rc::new(current),
                options,
                commit: Some(Rc::new(commit)),
            },
        )
    }

    /// Pick option `ix` of a choice: through the preset's `commit`, or by
    /// writing the field's own key.
    pub fn choose(&self, path: &ConfigPath<S>, ix: usize, cx: &mut App) {
        let Editor::Choice {
            options, commit, ..
        } = &self.editor
        else {
            return;
        };
        match commit {
            Some(commit) => commit(path, ix, cx),
            None => {
                let Some(option) = options.get(ix) else {
                    return;
                };
                let apply = option.apply.clone();
                path.write(self, |s| apply(s), cx);
            }
        }
    }

    /// A child struct with its own `Editable` impl.
    pub fn nested<C: Editable>(
        key: &'static str,
        label: &'static str,
        get: fn(&S) -> &C,
        get_mut: fn(&mut S) -> &mut C,
    ) -> Self {
        let child = move |path: &ConfigPath<S>| path.then(key, get, get_mut);
        let open: Open<S> =
            Rc::new(move |path: ConfigPath<S>, cx: &mut App| open_section(child(&path), cx));
        let reset = Rc::new(move |s: &mut S| *get_mut(s) = C::default());
        let form: Form<S> = Rc::new(move |path: &ConfigPath<S>, depth: usize, cx: &App| {
            render_form(&child(path), &C::groups(), depth, cx)
        });
        let quick = Rc::new(move |path: &ConfigPath<S>| {
            let child = child(path);
            C::fields()
                .iter()
                .filter(|field| !field.editor.is_empty())
                .map(|field| primary_action(&child, field, true))
                .collect()
        });
        let summary = Rc::new(move |s: &S| summarize(&C::fields(), get(s)));
        let modified = Rc::new(move |s: &S| {
            let child = get(s);
            C::fields().iter().any(|field| field.is_modified(child))
        });
        let hoist: Option<Hoist<S>> = (C::fields().len() == 1).then(|| {
            Rc::new(move |parent: &Field<S>, path: &ConfigPath<S>, cx: &App| {
                hoisted_item::<C>(parent, &child(path), cx)
            }) as Hoist<S>
        });
        Self::new(
            key,
            label,
            Editor::Nested {
                open,
                reset,
                form,
                quick,
                summary,
                modified,
                hoist,
                count: C::fields().len(),
            },
        )
    }

    /// Several fields of this struct behind one row. `key` only names the
    /// row; each field keeps its own key.
    pub fn bundle(key: &'static str, label: &'static str, fields: Vec<Field<S>>) -> Self {
        Self::new(key, label, Editor::Bundle(fields.into()))
    }

    /// A hand-written page for fields the generic editors cannot express.
    pub fn custom(
        key: &'static str,
        label: &'static str,
        open: impl Fn(ConfigPath<S>, &mut App) -> Page + 'static,
    ) -> Self {
        Self::new(
            key,
            label,
            Editor::Custom {
                open: Rc::new(open),
            },
        )
    }

    pub fn describe(mut self, description: &'static str) -> Self {
        self.description = description;
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn preview(mut self, draw: impl Fn(&ConfigPath<S>, &App) -> AnyElement + 'static) -> Self {
        self.preview = Some(Rc::new(draw));
        self
    }

    pub fn range(mut self, lo: f64, hi: f64) -> Self {
        if let Editor::Number { min, max, .. } = &mut self.editor {
            *min = lo;
            *max = hi;
        }
        self
    }

    pub fn step(mut self, by: f64) -> Self {
        if let Editor::Number { step, .. } = &mut self.editor {
            *step = by;
        }
        self
    }

    pub fn unit(mut self, suffix: &'static str) -> Self {
        if let Editor::Number { unit, .. } = &mut self.editor {
            *unit = Some(suffix);
        }
        self
    }

    pub fn hint(mut self, value: TextHint) -> Self {
        if let Editor::Text { hint, .. } = &mut self.editor {
            *hint = value;
        }
        self
    }

    /// The field as TOML; `None` for nested and custom fields, which span
    /// several keys.
    pub fn current_value(&self, section: &S) -> Option<toml_edit::Value> {
        match &self.editor {
            Editor::Toggle(lens) => Some((lens.to_toml)(section)),
            Editor::Number { lens, .. } => Some((lens.to_toml)(section)),
            Editor::Text { lens, .. } => Some((lens.to_toml)(section)),
            Editor::Choice {
                current, options, ..
            } => options.get(current(section)).map(|o| o.value.clone()),
            Editor::Nested { .. } | Editor::Custom { .. } | Editor::Bundle(_) => None,
        }
    }

    /// The value as the row shows it.
    pub fn display(&self, section: &S) -> String {
        match &self.editor {
            Editor::Toggle(lens) => if (lens.get)(section) { "on" } else { "off" }.into(),
            Editor::Number {
                lens,
                integer,
                unit,
                ..
            } => {
                let value = (lens.get)(section);
                let number = if *integer {
                    format!("{value:.0}")
                } else {
                    trim_float(value)
                };
                match unit {
                    Some(unit) => format!("{number} {unit}"),
                    None => number,
                }
            }
            Editor::Text { lens, .. } => (lens.get)(section),
            Editor::Choice {
                current, options, ..
            } => options
                .get(current(section))
                .map_or_else(|| "Custom".to_string(), |o| o.label.to_string()),
            Editor::Nested { summary, .. } => summary(section),
            Editor::Custom { .. } => String::new(),
            Editor::Bundle(fields) => summarize(fields, section),
        }
    }

    /// Differs from the default. `toml_edit::Value` has no `PartialEq`, so the
    /// comparison goes through its text.
    pub fn is_modified(&self, section: &S) -> bool {
        match &self.editor {
            Editor::Nested { modified, .. } => return modified(section),
            Editor::Bundle(fields) => return fields.iter().any(|f| f.is_modified(section)),
            _ => {}
        }
        match (
            self.current_value(section),
            self.current_value(&S::default()),
        ) {
            (Some(now), Some(default)) => now.to_string() != default.to_string(),
            _ => false,
        }
    }

    /// Put the default back in memory; the store drops the key.
    pub fn reset(&self, section: &mut S) {
        match &self.editor {
            Editor::Toggle(lens) => (lens.reset)(section),
            Editor::Number { lens, .. } => (lens.reset)(section),
            Editor::Text { lens, .. } => (lens.reset)(section),
            Editor::Choice {
                current, options, ..
            } => {
                if let Some(option) = options.get(current(&S::default())) {
                    (option.apply)(section);
                }
            }
            Editor::Nested { reset, .. } => reset(section),
            Editor::Custom { .. } => {}
            Editor::Bundle(fields) => fields.iter().for_each(|f| f.reset(section)),
        }
    }
}

/// The first few values of a child, for a nested row's subtitle; a child
/// made of children counts them instead.
fn summarize<C: Editable>(fields: &[Field<C>], child: &C) -> String {
    let values: Vec<String> = fields
        .iter()
        .filter(|field| {
            !matches!(
                field.editor,
                Editor::Nested { .. } | Editor::Custom { .. } | Editor::Bundle(_)
            )
        })
        .map(|field| field.display(child))
        .filter(|value| !value.is_empty())
        .take(4)
        .collect();
    if values.is_empty() {
        return format!("{} settings", fields.len());
    }
    values.join(" · ")
}

/// `C::page` if it has one, else the generic field list.
pub fn open_section<C: Editable>(path: ConfigPath<C>, cx: &mut App) -> Page {
    C::page(path.clone(), cx).unwrap_or_else(|| Page::new(cx.new(|cx| SectionPage::new(path, cx))))
}

type Get<S> = Rc<dyn Fn(&Config) -> &S>;
type GetMut<S> = Rc<dyn Fn(&mut Config) -> &mut S>;

/// Where a section lives: accessors from the root config, and its table in
/// `config.toml`.
pub struct ConfigPath<S> {
    keys: Rc<[&'static str]>,
    get: Get<S>,
    get_mut: GetMut<S>,
}

impl<S> Clone for ConfigPath<S> {
    fn clone(&self) -> Self {
        Self {
            keys: self.keys.clone(),
            get: self.get.clone(),
            get_mut: self.get_mut.clone(),
        }
    }
}

// Closures handed straight to `Rc::new` lose the `for<'a>` on the returned
// reference; a bound on a helper keeps it.
fn getter<S>(f: impl for<'a> Fn(&'a Config) -> &'a S + 'static) -> Get<S> {
    Rc::new(f)
}

fn getter_mut<S>(f: impl for<'a> Fn(&'a mut Config) -> &'a mut S + 'static) -> GetMut<S> {
    Rc::new(f)
}

impl ConfigPath<Config> {
    pub fn root() -> Self {
        Self {
            keys: Rc::from([]),
            get: getter(|config| config),
            get_mut: getter_mut(|config| config),
        }
    }
}

impl<S: 'static> ConfigPath<S> {
    pub fn then<C: 'static>(
        &self,
        key: &'static str,
        get: fn(&S) -> &C,
        get_mut: fn(&mut S) -> &mut C,
    ) -> ConfigPath<C> {
        let parent = self.get.clone();
        let parent_mut = self.get_mut.clone();
        ConfigPath {
            keys: self.keys.iter().copied().chain([key]).collect(),
            get: getter(move |config| get(parent(config))),
            get_mut: getter_mut(move |config| get_mut(parent_mut(config))),
        }
    }

    pub fn read<'a>(&self, cx: &'a App) -> &'a S {
        (self.get)(Config::global(cx))
    }

    /// Full key path of a field, dotted segments split.
    pub fn keys_for(&self, field_key: &'static str) -> Vec<&'static str> {
        self.keys
            .iter()
            .copied()
            .chain(field_key.split('.'))
            .collect()
    }

    pub fn key_string(&self, field_key: &'static str) -> String {
        self.keys_for(field_key).join(".")
    }

    /// Apply `set` to the section and record the field's new value.
    pub fn write(&self, field: &Field<S>, set: impl FnOnce(&mut S), cx: &mut App)
    where
        S: Editable,
    {
        let mut section = self.read(cx).clone();
        set(&mut section);
        let Some(value) = field.current_value(&section) else {
            tracing::warn!(
                "{} spans several keys and cannot be written as one",
                field.key
            );
            return;
        };
        self.set_raw(self.keys_for(field.key), value, section, cx);
    }

    pub fn reset(&self, field: &Field<S>, cx: &mut App)
    where
        S: Editable,
    {
        let mut section = self.read(cx).clone();
        field.reset(&mut section);
        let keys: Vec<&'static str> = match &field.editor {
            Editor::Bundle(fields) => fields.iter().map(|f| f.key).collect(),
            _ => vec![field.key],
        };
        for key in keys {
            let get_mut = self.get_mut.clone();
            let section = section.clone();
            store::apply(cx, Edit::Remove(self.keys_for(key)), move |config| {
                *get_mut(config) = section;
            });
        }
    }

    /// Replace the whole section in memory and record that `keys` should be
    /// dropped from the file, so its default applies again.
    pub fn remove_raw(&self, keys: Vec<&'static str>, section: S, cx: &mut App) {
        let get_mut = self.get_mut.clone();
        store::apply(cx, Edit::Remove(keys), move |config| {
            *get_mut(config) = section;
        });
    }

    /// Replace the whole section in memory and record one key's value.
    pub fn set_raw(
        &self,
        keys: Vec<&'static str>,
        value: toml_edit::Value,
        section: S,
        cx: &mut App,
    ) {
        let get_mut = self.get_mut.clone();
        store::apply(cx, Edit::Set(keys, value), move |config| {
            *get_mut(config) = section;
        });
    }
}

fn trim_float(value: f64) -> String {
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0');
    text.trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_display_without_trailing_zeros() {
        assert_eq!(trim_float(32.0), "32");
        assert_eq!(trim_float(0.5), "0.5");
        assert_eq!(trim_float(0.05), "0.05");
    }

    #[test]
    fn a_bad_strftime_format_is_rejected_without_panicking() {
        for bad in ["%Q", "%"] {
            assert!(TextHint::Strftime.validate(bad).is_err());
            assert_eq!(TextHint::Strftime.preview(bad), None);
        }
        assert!(TextHint::Strftime.validate("%H:%M").is_ok());
        assert!(TextHint::Strftime.preview("%H:%M").is_some());
    }

    #[test]
    fn paths_compose_keys() {
        let root = ConfigPath::root();
        let bar = root.then("bar", |c: &Config| &c.bar, |c: &mut Config| &mut c.bar);
        assert_eq!(bar.keys_for("size"), vec!["bar", "size"]);
        assert_eq!(bar.key_string("modules.clock"), "bar.modules.clock");
    }

    /// An optional number shows its stand-in while unset, is not "modified"
    /// then, and goes back to unset - not to the stand-in - on reset.
    #[test]
    fn an_optional_number_resets_to_unset() {
        let opacity = crate::bar::config::BarConfig::fields()
            .into_iter()
            .find(|field| field.key == "opacity")
            .unwrap();
        let mut bar = crate::bar::config::BarConfig::default();
        assert!(!opacity.is_modified(&bar));
        assert_eq!(opacity.display(&bar), "1");

        bar.opacity = Some(0.5);
        assert!(opacity.is_modified(&bar));
        assert_eq!(opacity.display(&bar), "0.5");

        opacity.reset(&mut bar);
        assert_eq!(bar.opacity, None);
    }

    #[test]
    fn a_nested_row_summarizes_and_tracks_its_child() {
        let bar = Config::fields()
            .into_iter()
            .find(|field| field.key == "bar")
            .unwrap();
        let mut config = Config::default();
        assert!(!bar.is_modified(&config));
        assert!(!bar.display(&config).is_empty());
        config.bar.size += 1.0;
        assert!(bar.is_modified(&config));
    }
}

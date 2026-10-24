//! Calculator view: a custom body with no items. `= 2 * (3 + 4)` shows the
//! steps and the result as you type; Enter copies it.

pub mod config;
mod eval;

use gpui::{
    ClipboardItem, Context, EventEmitter, FontWeight, IntoElement, Render, SharedString, Window,
    div, prelude::*,
};
use ui::{ActiveTheme, Color, IconName, Label, LabelCommon, Radius, Spacing, TextSize};

use self::config::CalculatorConfig;
use self::eval::{Evaluation, evaluate, format_number};
use crate::launcher::view::{
    Body, Hint, Items, LauncherView, NavKey, Outcome, ViewDescriptor, ViewEvent, ViewMeta,
};

pub fn descriptor(config: &CalculatorConfig) -> ViewDescriptor {
    ViewDescriptor::new(
        ViewMeta {
            id: "calculator",
            prefix: config.prefix.clone().into(),
            name: "Calculator",
            icon: IconName::Hash,
            description: "Evaluate arithmetic: + - * / % ^ ( )",
            default: false,
            in_help: true,
        },
        |_| CalculatorView {
            expression: String::new(),
            evaluation: None,
        },
    )
}

pub struct CalculatorView {
    expression: String,
    evaluation: Option<Evaluation>,
}

impl CalculatorView {
    fn formatted(&self) -> Option<String> {
        match self.evaluation.as_ref()?.result {
            Ok(value) => Some(format_number(value)),
            Err(_) => None,
        }
    }
}

impl EventEmitter<ViewEvent> for CalculatorView {}

impl LauncherView for CalculatorView {
    fn items(&mut self, query: &str, cx: &mut Context<Self>) -> Items {
        self.expression = query.trim().to_string();
        self.evaluation = (!self.expression.is_empty()).then(|| evaluate(&self.expression));
        cx.notify();
        Items::Ready(Vec::new())
    }

    fn activate(
        &mut self,
        _item: &crate::launcher::item::Item,
        _action: &str,
        _cx: &mut Context<Self>,
    ) -> Outcome {
        Outcome::Stay
    }

    fn body(&mut self, cx: &mut Context<Self>) -> Body {
        Body::Custom(cx.entity().into())
    }

    fn on_key(&mut self, key: NavKey, cx: &mut Context<Self>) -> Option<Outcome> {
        if key != NavKey::Enter {
            return None;
        }
        let value = self.formatted()?;
        cx.write_to_clipboard(ClipboardItem::new_string(value));
        Some(Outcome::Close)
    }

    fn footer_hints(&self, _selected: Option<&crate::launcher::item::Item>) -> Option<Vec<Hint>> {
        let mut hints: Vec<Hint> = Vec::new();
        if self.formatted().is_some() {
            hints.push(("Copy".into(), "Enter".into()));
        }
        hints.push(("Close".into(), "Esc".into()));
        Some(hints)
    }
}

impl Render for CalculatorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();
        let (result, color): (SharedString, Color) = match &self.evaluation {
            None => (
                "Type an expression, e.g. 2 * (3 + 4)".into(),
                Color::Placeholder,
            ),
            Some(Evaluation {
                result: Ok(value), ..
            }) => (format_number(*value).into(), Color::Default),
            Some(Evaluation {
                result: Err(err), ..
            }) => ((*err).into(), Color::Error),
        };
        let steps = self
            .evaluation
            .iter()
            .flat_map(|eval| &eval.steps)
            .map(|step| {
                Label::new(format!("= {step}"))
                    .size(TextSize::Small)
                    .color(Color::Muted)
            });
        let steps: Vec<_> = steps.collect();

        div().p(Spacing::Large.pixels()).child(
            div()
                .w_full()
                .p(Spacing::XLarge.pixels())
                .rounded(Radius::Medium.pixels())
                .bg(colors.surface_background)
                .flex()
                .flex_col()
                .gap(Spacing::Medium.pixels())
                .child(
                    Label::new(if self.expression.is_empty() {
                        SharedString::from("=")
                    } else {
                        SharedString::from(self.expression.clone())
                    })
                    .size(TextSize::Small)
                    .color(Color::Muted),
                )
                .when(!steps.is_empty(), |el| {
                    el.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(Spacing::XSmall.pixels())
                            .pl(Spacing::Medium.pixels())
                            .border_l_2()
                            .border_color(colors.border)
                            .children(steps),
                    )
                })
                .child(
                    div()
                        .text_size(TextSize::Large.rems())
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color.hsla(colors))
                        .child(result),
                ),
        )
    }
}

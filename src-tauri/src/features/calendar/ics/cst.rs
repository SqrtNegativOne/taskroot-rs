//! Shared `ical-rs` CST helpers for reading and editing components.

use crate::error::AppError;
use ical::param::IcalParam;
use ical::prop::{IcalProp, IcalPropKind, IcalPropName};
use ical::tree::codec::mode::Escaper;
use ical::tree::cst::{IcalCst, IcalItem};
use ical::tree::line::IcalLine;
use ical::tree::param::reltype::RELTYPE;
use ical::value::IcalValue;
use ical::value::text::IcalText;
use std::borrow::Cow;

pub(super) const EVENT_KIND: &str = "VEVENT";
pub(super) const TASK_KIND: &str = "VTODO";
pub(super) const ALARM_KIND: &str = "VALARM";

pub(super) fn parse_calendar<'a>(raw: &'a str, source: &str) -> Result<IcalCst<'a>, AppError> {
    let cst = IcalCst::parse(raw)
        .map_err(|error| AppError::InvalidInput(format!("{source}: {error}")))?;
    if !is_component(&cst, "VCALENDAR") {
        return Err(AppError::InvalidInput(format!(
            "{source}: missing BEGIN:VCALENDAR"
        )));
    }
    Ok(cst)
}

pub(super) fn render_cst(cst: &IcalCst<'_>) -> String {
    let bytes = cst.to_bytes();
    String::from_utf8(bytes)
        .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

pub(super) fn is_component(cst: &IcalCst<'_>, kind: &str) -> bool {
    cst.begin
        .as_ref()
        .is_some_and(|line| line.raw_value_str().eq_ignore_ascii_case(kind))
}

pub(super) fn child_components<'a>(
    parent: &'a IcalCst<'a>,
    kind: &'a str,
) -> impl Iterator<Item = &'a IcalCst<'a>> {
    parent.items.iter().filter_map(move |item| match item {
        IcalItem::Component(child) if is_component(child, kind) => Some(child.as_ref()),
        _ => None,
    })
}

pub(super) fn prop<'a>(component: &'a IcalCst<'a>, name: &str) -> Option<&'a IcalLine<'a>> {
    component.items.iter().find_map(|item| match item {
        IcalItem::Prop(line) if line.name.get().eq_ignore_ascii_case(name) => Some(line),
        _ => None,
    })
}

pub(super) fn props<'a>(
    component: &'a IcalCst<'a>,
    name: &'a str,
) -> impl Iterator<Item = &'a IcalLine<'a>> {
    component.items.iter().filter_map(move |item| match item {
        IcalItem::Prop(line) if line.name.get().eq_ignore_ascii_case(name) => Some(line),
        _ => None,
    })
}

pub(super) fn text_prop(component: &IcalCst<'_>, name: &str) -> Option<String> {
    prop(component, name).map(|line| line.value.decode().into_owned())
}

pub(super) fn is_parent_relation(line: &IcalLine<'_>) -> bool {
    line.name.get().eq_ignore_ascii_case("RELATED-TO")
        && line
            .param::<RELTYPE>()
            .is_none_or(|kind| kind.eq_ignore_ascii_case("PARENT"))
}

pub(super) fn is_dependency_relation(line: &IcalLine<'_>) -> bool {
    line.name.get().eq_ignore_ascii_case("RELATED-TO")
        && line
            .param::<RELTYPE>()
            .is_some_and(|kind| kind.eq_ignore_ascii_case("DEPENDS-ON"))
}

pub(super) fn encode(
    name: impl Into<IcalPropName<'static>>,
    params: Vec<IcalParam<'_>>,
    value: IcalValue<'_>,
) -> IcalLine<'static> {
    IcalProp {
        name: name.into(),
        params,
        value,
    }
    .encode(Escaper::Modern)
}

pub(super) fn text_line(kind: IcalPropKind, value: &str) -> IcalLine<'static> {
    encode(kind, Vec::new(), IcalValue::Text(IcalText(strip_cr(value))))
}

pub(super) fn strip_cr(value: &str) -> Cow<'_, str> {
    if value.contains('\r') {
        Cow::Owned(value.replace('\r', ""))
    } else {
        Cow::Borrowed(value)
    }
}

/// Remove owned direct properties from `component` and insert `lines` ahead of
/// nested components, leaving every non-owned property and child untouched.
pub(super) fn replace_props(
    component: &mut IcalCst<'_>,
    is_owned: impl Fn(&IcalLine<'_>) -> bool,
    lines: Vec<IcalLine<'static>>,
) {
    component
        .items
        .retain(|item| !matches!(item, IcalItem::Prop(line) if is_owned(line)));
    let at = component
        .items
        .iter()
        .position(|item| matches!(item, IcalItem::Component(_)))
        .unwrap_or(component.items.len());
    let tail = component.items.split_off(at);
    component
        .items
        .extend(lines.into_iter().map(IcalItem::Prop));
    component.items.extend(tail);
}

//! Line-preserving iCalendar editing primitives (plan §7, invariant I3).
//!
//! A [`Document`] keeps every physical line of the resource it parsed, so an edit
//! is expressed as "replace this one content line". Untouched lines are re-emitted
//! byte-for-byte, including their original folding; only lines this module creates
//! are folded, at 75 octets (RFC 5545 §3.1).

use std::collections::BTreeMap;

/// Longest content line, in octets, excluding the line break.
const MAX_LINE_OCTETS: usize = 75;
const CALENDAR_KIND: &str = "VCALENDAR";
const CALENDAR_END: &str = "END:VCALENDAR";

/// One unfolded content line plus the physical lines it was parsed from.
#[derive(Debug, Clone)]
struct RawLine {
    text: String,
    physical: Vec<String>,
}

impl RawLine {
    fn new(text: String) -> Self {
        Self {
            physical: fold(&text),
            text,
        }
    }

    fn as_str(&self) -> &str {
        &self.text
    }
}

/// Lines to substitute or drop, keyed by the index of the original line.
#[derive(Debug, Default)]
struct Edits {
    replace: BTreeMap<usize, Vec<RawLine>>,
    insert_before: BTreeMap<usize, Vec<RawLine>>,
}

#[derive(Debug)]
struct OpenComponent {
    kind: String,
    begin: usize,
    /// Property lines that are *direct* children of this component.
    properties: Vec<usize>,
    children: Vec<ComponentSpan>,
}

/// A `BEGIN`/`END` pair that is a direct child of the calendar.
#[derive(Debug, Clone)]
pub(super) struct ComponentSpan {
    pub(super) kind: String,
    pub(super) uid: Option<String>,
    begin: usize,
    end: usize,
    /// Indexes of this component's own property lines, never a nested
    /// component's. A `VALARM`'s `DESCRIPTION` must never look like the
    /// `VTODO`'s `DESCRIPTION`.
    properties: Vec<usize>,
    /// Nested components such as `VALARM`, `STANDARD` or `DAYLIGHT`.
    children: Vec<Self>,
}

/// Structured access to a raw resource body that never re-serializes it.
#[derive(Debug, Clone)]
pub(super) struct Document {
    lines: Vec<RawLine>,
    eol: String,
    trailing_eol: bool,
}

impl Document {
    /// Split `raw` into unfolded content lines, remembering the original bytes.
    pub(super) fn parse(raw: &str) -> Self {
        let eol = if raw.contains("\r\n") { "\r\n" } else { "\n" };
        let trailing_eol = raw.ends_with('\n');
        let mut pieces = raw.split('\n').collect::<Vec<_>>();
        if trailing_eol {
            pieces.pop();
        }
        let mut lines = Vec::new();
        let mut current: Option<RawLine> = None;
        for piece in pieces {
            let content = piece.strip_suffix('\r').unwrap_or(piece);
            if let Some(continuation) = content.strip_prefix([' ', '\t']) {
                if let Some(open) = current.as_mut() {
                    open.text.push_str(continuation);
                    open.physical.push(content.to_string());
                    continue;
                }
            }
            if let Some(previous) = current.take() {
                lines.push(previous);
            }
            current = Some(RawLine {
                text: content.to_string(),
                physical: vec![content.to_string()],
            });
        }
        if let Some(last) = current {
            lines.push(last);
        }
        Self {
            lines,
            eol: eol.to_string(),
            trailing_eol,
        }
    }

    /// Every direct child of the calendar, in file order, each with its own
    /// nested components attached.
    pub(super) fn components(&self) -> Vec<ComponentSpan> {
        let mut roots: Vec<ComponentSpan> = Vec::new();
        let mut stack: Vec<OpenComponent> = Vec::new();
        for (index, line) in self.lines.iter().enumerate() {
            let text = line.as_str();
            if let Some(kind) = text.strip_prefix("BEGIN:") {
                stack.push(OpenComponent {
                    kind: kind.to_string(),
                    begin: index,
                    properties: Vec::new(),
                    children: Vec::new(),
                });
                continue;
            }
            if text.starts_with("END:") {
                let Some(open) = stack.pop() else { continue };
                let uid = open
                    .properties
                    .iter()
                    .find_map(|line| self.property_value(*line, "UID"));
                let span = ComponentSpan {
                    kind: open.kind,
                    uid,
                    begin: open.begin,
                    end: index,
                    properties: open.properties,
                    children: open.children,
                };
                match stack.last_mut() {
                    Some(parent) => parent.children.push(span),
                    None => roots.push(span),
                }
                continue;
            }
            // A content line belongs to the innermost open component only, so a
            // nested component's properties never leak into its parent's span.
            if let Some(owner) = stack.last_mut() {
                owner.properties.push(index);
            }
        }
        flatten_calendar(roots)
    }

    /// Rewrite the owned property groups of `span`, leaving everything else alone.
    pub(super) fn edit_component(
        &self,
        span: &ComponentSpan,
        specs: &[PropertySpec],
        nested: &[NestedSpec],
    ) -> String {
        let mut edits = Edits::default();
        for spec in specs {
            self.apply_spec(span, spec, &mut edits);
        }
        for spec in nested {
            self.apply_nested(span, spec, &mut edits);
        }
        self.render(&edits)
    }

    /// Remove whole components from the resource.
    pub(super) fn delete_components(&self, spans: &[&ComponentSpan]) -> String {
        let mut edits = Edits::default();
        for span in spans {
            for index in span.begin..=span.end {
                edits.replace.insert(index, Vec::new());
            }
        }
        self.render(&edits)
    }

    /// Append a freshly built component just before `END:VCALENDAR`.
    pub(super) fn insert_component(&self, component: &[String]) -> String {
        let anchor = self
            .lines
            .iter()
            .rposition(|line| line.as_str() == CALENDAR_END)
            .unwrap_or(self.lines.len());
        let mut edits = Edits::default();
        edits.insert_before.insert(
            anchor,
            component.iter().cloned().map(RawLine::new).collect(),
        );
        self.render(&edits)
    }

    fn apply_spec(&self, span: &ComponentSpan, spec: &PropertySpec, edits: &mut Edits) {
        let matching = span
            .properties
            .iter()
            .copied()
            .filter(|index| {
                self.lines
                    .get(*index)
                    .is_some_and(|line| spec.kind.matches(line.as_str()))
            })
            .collect::<Vec<_>>();
        for (position, index) in matching.iter().copied().enumerate() {
            let current = self
                .lines
                .get(index)
                .map(RawLine::as_str)
                .unwrap_or_default();
            match spec.values.get(position) {
                Some(value) if value == current => {}
                Some(value) => {
                    edits
                        .replace
                        .insert(index, vec![RawLine::new(value.clone())]);
                }
                None => {
                    edits.replace.insert(index, Vec::new());
                }
            }
        }
        if spec.values.len() <= matching.len() {
            return;
        }
        let Some(extra) = spec.values.get(matching.len()..) else {
            return;
        };
        // Keep the component's own properties ahead of its nested components,
        // which is the shape tasks.org writes.
        let anchor = matching.last().map_or_else(
            || span.children.first().map_or(span.end, |child| child.begin),
            |last| last.saturating_add(1),
        );
        edits
            .insert_before
            .entry(anchor)
            .or_default()
            .extend(extra.iter().cloned().map(RawLine::new));
    }

    /// Apply a [`NestedSpec`] to `span`'s child components of that kind.
    ///
    /// Instances are matched by position, so the DTO's order is the document's
    /// order. Existing instances keep every line the specs do not own; surplus
    /// instances are removed; missing ones are appended from their block.
    fn apply_nested(&self, span: &ComponentSpan, spec: &NestedSpec, edits: &mut Edits) {
        let existing = span
            .children
            .iter()
            .filter(|child| child.kind == spec.kind)
            .collect::<Vec<_>>();
        let mut pending: Vec<RawLine> = Vec::new();
        for (position, instance) in spec.instances.iter().enumerate() {
            let Some(child) = existing.get(position) else {
                pending.extend(instance.block.iter().cloned().map(RawLine::new));
                continue;
            };
            if !pending.is_empty() {
                edits
                    .insert_before
                    .entry(child.begin)
                    .or_default()
                    .append(&mut pending);
            }
            for property in &instance.specs {
                self.apply_spec(child, property, edits);
            }
        }
        for surplus in existing.iter().skip(spec.instances.len()) {
            for index in surplus.begin..=surplus.end {
                edits.replace.insert(index, Vec::new());
            }
        }
        if !pending.is_empty() {
            let anchor = existing
                .last()
                .map_or(span.end, |child| child.end.saturating_add(1));
            edits
                .insert_before
                .entry(anchor)
                .or_default()
                .append(&mut pending);
        }
    }

    fn property_value(&self, index: usize, name: &str) -> Option<String> {
        let text = self.lines.get(index)?.as_str();
        if property_name(text) != name {
            return None;
        }
        let (_, value) = text.split_once(':')?;
        Some(value.to_string())
    }

    fn render(&self, edits: &Edits) -> String {
        let mut out = String::new();
        for (index, line) in self.lines.iter().enumerate() {
            if let Some(inserted) = edits.insert_before.get(&index) {
                push_lines(&mut out, inserted, &self.eol);
            }
            match edits.replace.get(&index) {
                Some(replacement) => push_lines(&mut out, replacement, &self.eol),
                None => push_lines(&mut out, std::slice::from_ref(line), &self.eol),
            }
        }
        if let Some(inserted) = edits.insert_before.get(&self.lines.len()) {
            push_lines(&mut out, inserted, &self.eol);
        }
        if !self.trailing_eol {
            if let Some(stripped) = out.strip_suffix(self.eol.as_str()) {
                out = stripped.to_string();
            }
        }
        out
    }
}

/// A property group this crate owns and may rewrite in place.
#[derive(Debug, Clone, Copy)]
pub(super) enum OwnedKind {
    /// Every line whose property name is this value.
    Name(&'static str),
    /// `RELATED-TO` lines whose `RELTYPE` is (or defaults to) `PARENT`.
    ParentRelation,
}

impl OwnedKind {
    fn matches(self, line: &str) -> bool {
        match self {
            Self::Name(name) => property_name(line) == name,
            Self::ParentRelation => is_parent_relation(line),
        }
    }
}

/// The desired content lines for one owned property group.
#[derive(Debug, Clone)]
pub(super) struct PropertySpec {
    pub(super) kind: OwnedKind,
    /// Complete content lines. An empty list removes every matching line.
    pub(super) values: Vec<String>,
}

impl PropertySpec {
    pub(super) const fn new(kind: OwnedKind, values: Vec<String>) -> Self {
        Self { kind, values }
    }

    pub(super) fn optional(kind: OwnedKind, value: Option<String>) -> Self {
        Self {
            kind,
            values: value.into_iter().collect(),
        }
    }
}

/// A nested component kind whose instances this crate owns lines of, such as
/// `VALARM` inside a `VTODO`.
#[derive(Debug, Clone)]
pub(super) struct NestedSpec {
    pub(super) kind: &'static str,
    /// The desired instances, in document order.
    pub(super) instances: Vec<NestedInstance>,
}

impl NestedSpec {
    pub(super) const fn new(kind: &'static str) -> Self {
        Self {
            kind,
            instances: Vec::new(),
        }
    }
}

/// One desired nested component.
#[derive(Debug, Clone)]
pub(super) struct NestedInstance {
    /// Property groups to rewrite inside this instance.
    pub(super) specs: Vec<PropertySpec>,
    /// Complete content lines (including `BEGIN`/`END`), used only when the
    /// instance has to be created.
    pub(super) block: Vec<String>,
}

impl NestedInstance {
    pub(super) const fn new(specs: Vec<PropertySpec>, block: Vec<String>) -> Self {
        Self { specs, block }
    }
}

/// Property name of a content line, without parameters.
pub(super) fn property_name(line: &str) -> &str {
    let end = line.find([';', ':']).unwrap_or(line.len());
    line.get(..end).unwrap_or(line)
}

/// The resource's components are the children of the calendar object.
fn flatten_calendar(roots: Vec<ComponentSpan>) -> Vec<ComponentSpan> {
    let mut spans = Vec::new();
    for root in roots {
        if root.kind == CALENDAR_KIND {
            spans.extend(root.children);
        } else {
            spans.push(root);
        }
    }
    spans
}

fn is_parent_relation(line: &str) -> bool {
    if property_name(line) != "RELATED-TO" {
        return false;
    }
    let params = line
        .split_once(';')
        .and_then(|(_, rest)| rest.split_once(':').map(|(params, _)| params));
    let Some(params) = params else {
        return true;
    };
    let reltype = params.split(';').find_map(|param| {
        let (key, value) = param.split_once('=')?;
        key.eq_ignore_ascii_case("RELTYPE").then_some(value)
    });
    reltype.is_none_or(|value| value.eq_ignore_ascii_case("PARENT"))
}

fn push_lines(out: &mut String, lines: &[RawLine], eol: &str) {
    for line in lines {
        for physical in &line.physical {
            out.push_str(physical);
            out.push_str(eol);
        }
    }
}

/// Fold a content line at [`MAX_LINE_OCTETS`], never splitting a UTF-8 sequence.
fn fold(text: &str) -> Vec<String> {
    let mut physical = Vec::new();
    let mut current = String::new();
    let mut octets: usize = 0;
    for character in text.chars() {
        let width = character.len_utf8();
        if octets.saturating_add(width) > MAX_LINE_OCTETS && !current.is_empty() {
            physical.push(std::mem::take(&mut current));
            current.push(' ');
            octets = 1;
        }
        current.push(character);
        octets = octets.saturating_add(width);
    }
    physical.push(current);
    physical
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_crlf_and_reports_calendar_children() {
        let document = Document::parse(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VTIMEZONE\r\nTZID:UTC\r\nEND:VTIMEZONE\r\nBEGIN:VEVENT\r\nUID:one\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        );
        let spans = document.components();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].kind, "VTIMEZONE");
        assert_eq!(spans[0].uid, None);
        assert_eq!(spans[1].kind, "VEVENT");
        assert_eq!(spans[1].uid.as_deref(), Some("one"));
    }

    #[test]
    fn round_trips_a_document_without_edits_byte_for_byte() {
        let raw = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:a\r\n  b\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        assert_eq!(Document::parse(raw).render(&Edits::default()), raw);
    }

    #[test]
    fn folded_lines_are_unfolded_for_matching_and_kept_verbatim_on_output() {
        let raw = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:x\r\nSUMMARY:hello\r\n world\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let document = Document::parse(raw);
        let span = document.components().remove(0);
        let specs = vec![PropertySpec::new(
            OwnedKind::Name("SUMMARY"),
            vec!["DESCRIPTION:new".to_string()],
        )];
        let edited = document.edit_component(&span, &specs, &[]);
        assert!(!edited.contains("hello"));
        assert!(edited.contains("DESCRIPTION:new"));
    }

    #[test]
    fn folding_never_exceeds_seventy_five_octets() {
        let long = format!("SUMMARY:{}", "é".repeat(100));
        for physical in fold(&long) {
            assert!(
                physical.len() <= MAX_LINE_OCTETS,
                "line of {} octets",
                physical.len()
            );
            assert!(physical.is_char_boundary(physical.len()));
        }
    }

    #[test]
    fn folding_is_lossless_when_unfolded() {
        let long = format!("SUMMARY:{}", "word ".repeat(40));
        let folded = fold(&long);
        assert!(folded.len() > 1);
        let joined: String = folded
            .iter()
            .enumerate()
            .map(|(index, line)| {
                if index == 0 {
                    line.clone()
                } else {
                    line.strip_prefix(' ').unwrap_or(line).to_string()
                }
            })
            .collect();
        assert_eq!(joined, long);
    }

    #[test]
    fn only_parent_relations_are_owned() {
        assert!(is_parent_relation("RELATED-TO;RELTYPE=PARENT:abc"));
        assert!(is_parent_relation("RELATED-TO:abc"));
        assert!(!is_parent_relation("RELATED-TO;RELTYPE=DEPENDS-ON:abc"));
        assert!(!is_parent_relation("X-RELATED-TO:abc"));
    }

    #[test]
    fn missing_owned_property_is_inserted_before_the_component_end() {
        let raw = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:x\r\nSUMMARY:a\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let document = Document::parse(raw);
        let span = document.components().remove(0);
        let edited = document.edit_component(
            &span,
            &[PropertySpec::new(
                OwnedKind::Name("COLOR"),
                vec!["COLOR:#112233".to_string()],
            )],
            &[],
        );
        assert_eq!(
            edited,
            "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:x\r\nSUMMARY:a\r\nCOLOR:#112233\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
        );
    }

    #[test]
    fn empty_values_remove_every_matching_line() {
        let raw = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:x\r\nEXDATE:20260101T000000\r\nEXDATE:20260102T000000\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let document = Document::parse(raw);
        let span = document.components().remove(0);
        let edited = document.edit_component(
            &span,
            &[PropertySpec::new(OwnedKind::Name("EXDATE"), vec![])],
            &[],
        );
        assert!(!edited.contains("EXDATE"));
    }

    const TASK_WITH_ALARM: &str = "BEGIN:VCALENDAR\r\nBEGIN:VTODO\r\nUID:x\r\nSUMMARY:Brush\r\nDESCRIPTION:task notes\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nDESCRIPTION:Default Tasks.org description\r\nTRIGGER;RELATED=END:-PT15M\r\nEND:VALARM\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";

    fn task_span(document: &Document) -> ComponentSpan {
        let spans = document.components();
        assert_eq!(spans.len(), 1, "the resource holds one VTODO");
        spans.into_iter().next().unwrap()
    }

    #[test]
    fn a_nested_components_properties_are_not_the_parents() {
        let document = Document::parse(TASK_WITH_ALARM);
        let span = task_span(&document);
        let properties = span
            .properties
            .iter()
            .map(|index| document.lines[*index].as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            properties,
            vec!["UID:x", "SUMMARY:Brush", "DESCRIPTION:task notes"],
            "only direct VTODO properties belong to the VTODO"
        );
        assert_eq!(span.children.len(), 1);
        assert_eq!(span.children[0].kind, "VALARM");
    }

    #[test]
    fn rewriting_a_parent_description_leaves_the_alarm_description_alone() {
        let document = Document::parse(TASK_WITH_ALARM);
        let span = task_span(&document);
        let edited = document.edit_component(
            &span,
            &[PropertySpec::new(
                OwnedKind::Name("DESCRIPTION"),
                vec!["DESCRIPTION:new notes".to_string()],
            )],
            &[],
        );

        assert!(edited.contains("DESCRIPTION:new notes"));
        assert!(
            edited.contains("DESCRIPTION:Default Tasks.org description"),
            "the VALARM's own DESCRIPTION must survive: {edited}"
        );
        assert_eq!(
            edited,
            TASK_WITH_ALARM.replace("DESCRIPTION:task notes", "DESCRIPTION:new notes")
        );
    }

    #[test]
    fn a_new_property_is_inserted_before_a_nested_component() {
        let document = Document::parse(TASK_WITH_ALARM);
        let span = task_span(&document);
        let edited = document.edit_component(
            &span,
            &[PropertySpec::new(
                OwnedKind::Name("CATEGORIES"),
                vec!["CATEGORIES:home".to_string()],
            )],
            &[],
        );

        assert!(
            edited.contains("CATEGORIES:home\r\nBEGIN:VALARM"),
            "properties stay ahead of nested components: {edited}"
        );
    }

    #[test]
    fn a_nested_spec_rewrites_only_its_owned_lines() {
        let document = Document::parse(&TASK_WITH_ALARM.replace(
            "TRIGGER;RELATED=END:-PT15M",
            "TRIGGER;RELATED=END:-PT15M\r\nX-MOZ-LASTACK:20261020T000000Z",
        ));
        let span = task_span(&document);
        let nested = NestedSpec {
            kind: "VALARM",
            instances: vec![NestedInstance::new(
                vec![PropertySpec::new(
                    OwnedKind::Name("TRIGGER"),
                    vec!["TRIGGER;RELATED=END:-PT30M".to_string()],
                )],
                Vec::new(),
            )],
        };
        let edited = document.edit_component(&span, &[], &[nested]);

        assert!(edited.contains("TRIGGER;RELATED=END:-PT30M"));
        assert!(!edited.contains("-PT15M"));
        assert!(edited.contains("X-MOZ-LASTACK:20261020T000000Z"));
        assert!(edited.contains("ACTION:DISPLAY"));
        assert!(edited.contains("DESCRIPTION:Default Tasks.org description"));
    }

    #[test]
    fn a_missing_nested_component_is_inserted_before_the_parent_end() {
        let document = Document::parse(
            "BEGIN:VCALENDAR\r\nBEGIN:VTODO\r\nUID:x\r\nSUMMARY:a\r\nEND:VTODO\r\nEND:VCALENDAR\r\n",
        );
        let span = task_span(&document);
        let nested = NestedSpec {
            kind: "VALARM",
            instances: vec![NestedInstance::new(
                Vec::new(),
                vec!["BEGIN:VALARM", "ACTION:DISPLAY", "END:VALARM"]
                    .into_iter()
                    .map(ToString::to_string)
                    .collect(),
            )],
        };
        let edited = document.edit_component(&span, &[], &[nested]);

        assert_eq!(
            edited,
            "BEGIN:VCALENDAR\r\nBEGIN:VTODO\r\nUID:x\r\nSUMMARY:a\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nEND:VALARM\r\nEND:VTODO\r\nEND:VCALENDAR\r\n"
        );
    }

    #[test]
    fn surplus_nested_components_are_removed() {
        let document = Document::parse(&TASK_WITH_ALARM.replace(
            "END:VALARM\r\n",
            "END:VALARM\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER;RELATED=END:-PT45M\r\nEND:VALARM\r\n",
        ));
        let span = task_span(&document);
        let nested = NestedSpec {
            kind: "VALARM",
            instances: vec![NestedInstance::new(Vec::new(), Vec::new())],
        };
        let edited = document.edit_component(&span, &[], &[nested]);

        assert!(edited.contains("-PT15M"), "the first alarm is kept");
        assert!(!edited.contains("-PT45M"), "the surplus alarm is removed");
    }
}

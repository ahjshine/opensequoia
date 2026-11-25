//! Flat arena implements the pinned html5ever TreeSink contract without recursive ownership.
//!
//! Source contract: https://github.com/servo/html5ever/blob/d7232d746d3112f08926d169591576bd12fe2fdf/markup5ever/interface/tree_builder.rs

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashSet;

use html5ever::interface::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::{
    Attribute, LocalName, ParseOpts, QualName, local_name, ns, parse_document, parse_fragment,
};

use crate::css::ComputedStyle;
use crate::{Diagnostic, DisplayList, Error, MAX_INPUT_BYTES, MAX_NODES, Namespace, ParseOptions};

const MAX_DIAGNOSTICS: usize = 64;
const PARSE_CHUNK_BYTES: usize = 256;

#[derive(Debug)]
pub struct Document {
    pub(crate) nodes: Vec<Node>,
    pub(crate) styles: Vec<ComputedStyle>,
    diagnostics: Vec<Diagnostic>,
    unsupported_css: usize,
    inspection_root: usize,
}

impl Document {
    /// Parse bounded UTF-8 with HTML error recovery; no scripts or requests are executed.
    /// Node overflow is an error, never a successfully returned partial document.
    pub fn parse(input: &str) -> Result<Self, Error> {
        Self::parse_with_options(input, &ParseOptions::default())
    }

    /// Parse a document or standalone fragment with explicit tokenizer/tree-builder flags.
    /// `scripting_enabled` changes HTML parsing (e.g. noscript); scripts never execute.
    /// A fragment has no ancestor form element, contextual attributes or browser host APIs.
    pub fn parse_with_options(input: &str, configuration: &ParseOptions) -> Result<Self, Error> {
        if input.len() > MAX_INPUT_BYTES {
            return Err(Error::InputTooLarge);
        }
        let options = ParseOpts {
            tree_builder: html5ever::tree_builder::TreeBuilderOpts {
                scripting_enabled: configuration.scripting_enabled,
                ..Default::default()
            },
            ..Default::default()
        };
        // Fragment context is a bounded name, not markup or an arbitrary DOM handle.
        let mut parser = match &configuration.fragment {
            None => parse_document(Sink::new(), options),
            Some(context) => {
                if context.local_name.is_empty()
                    || context.local_name.len() > 256
                    || context.local_name.chars().any(|ch| {
                        ch.is_control()
                            || ch.is_whitespace()
                            || matches!(ch, '<' | '>' | '/' | '\'' | '"' | '=')
                    })
                {
                    return Err(Error::InvalidContext);
                }
                let namespace = match context.namespace {
                    Namespace::Html => ns!(html),
                    Namespace::Svg => ns!(svg),
                    Namespace::MathMl => ns!(mathml),
                };
                let name = QualName::new(
                    None,
                    namespace,
                    LocalName::from(context.local_name.as_str()),
                );
                parse_fragment(Sink::new(), options, name, vec![], true)
            }
        };
        let mut start = 0;
        while start < input.len() {
            let mut end = (start + PARSE_CHUNK_BYTES).min(input.len());
            while !input.is_char_boundary(end) {
                end -= 1;
            }
            parser.process(StrTendril::from(&input[start..end]));
            if parser.tokenizer.sink.sink.state.borrow().overflow {
                // TreeSink callbacks are infallible. Overflow handles retain stable identity
                // and names until this bounded input chunk returns; no arena node is added.
                // Dropping the flat parser/arena here cannot recursively destroy the DOM.
                return Err(Error::NodeLimit);
            }
            start = end;
        }
        let mut document = parser.finish()?;
        if configuration.fragment.is_some() {
            // html5ever returns fragment children under a synthetic html element.
            let root = document.nodes[0].first.ok_or(Error::InvalidTree)?;
            let node = document.nodes.get(root).ok_or(Error::InvalidTree)?;
            if node.next.is_some()
                || !matches!(&node.kind,
                NodeKind::Element { name, .. } if name.ns == ns!(html) && name.local == local_name!("html"))
            {
                return Err(Error::InvalidTree);
            }
            document.inspection_root = root;
        }
        Ok(document)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Count of CSS selectors/declarations skipped as unsupported in this subset.
    /// This is a support-boundary counter, not a conformance percentage.
    pub fn unsupported_css_declarations(&self) -> usize {
        self.unsupported_css
    }

    /// Canonical HTML tree-construction inspection, without the `#document` heading.
    /// This is a bounded diagnostic tree dump, not HTML serialization or a live DOM API.
    pub fn tree_dump(&self) -> Result<String, Error> {
        crate::tree_dump::serialize(self, self.inspection_root)
    }

    /// Limited block/text layout: estimated glyph advances plus the documented CSS subset.
    /// Block background colours become fill records; solid border widths are painted as four
    /// edge fills using the element's resolved border colour.
    pub fn layout(&self, width: u32) -> Result<DisplayList, Error> {
        crate::layout::layout(self, width)
    }
}

#[derive(Debug)]
pub(crate) struct Node {
    pub(crate) kind: NodeKind,
    pub(crate) parent: Option<usize>,
    pub(crate) previous: Option<usize>,
    pub(crate) next: Option<usize>,
    pub(crate) first: Option<usize>,
    pub(crate) last: Option<usize>,
}

impl Node {
    fn new(kind: NodeKind) -> Self {
        Self {
            kind,
            parent: None,
            previous: None,
            next: None,
            first: None,
            last: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum NodeKind {
    Document,
    Element {
        name: QualName,
        attrs: Vec<Attribute>,
        template_contents: Option<usize>,
    },
    Text(String),
    Comment(String),
    Doctype {
        name: String,
        public_id: String,
        system_id: String,
    },
    ProcessingInstruction {
        target: String,
        data: String,
    },
}

// Names remain on handles so elem_name never borrows the mutable arena. Unique virtual
// overflow ids let html5ever complete its current small chunk without invalid indexing.
#[derive(Clone)]
struct Handle {
    id: usize,
    name: QualName,
    template: Option<usize>,
    mathml_integration: bool,
}

impl Handle {
    fn plain(id: usize) -> Self {
        Self {
            id,
            name: document_name(),
            template: None,
            mathml_integration: false,
        }
    }
}

fn document_name() -> QualName {
    QualName::new(None, ns!(html), local_name!("html"))
}

struct State {
    nodes: Vec<Node>,
    next_id: usize,
    overflow: bool,
    diagnostics: Vec<Diagnostic>,
    selectedcontent_cloned_options: HashSet<usize>,
}

impl State {
    fn diagnose(&mut self, diagnostic: Diagnostic) {
        if self.diagnostics.len() < MAX_DIAGNOSTICS && !self.diagnostics.contains(&diagnostic) {
            self.diagnostics.push(diagnostic);
        }
    }

    fn allocate(&mut self, kind: NodeKind) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        if id < MAX_NODES {
            self.nodes.push(Node::new(kind));
        } else {
            self.overflow = true;
        }
        id
    }

    fn detach(&mut self, id: usize) {
        let Some(node) = self.nodes.get(id) else {
            return;
        };
        let (parent, previous, next) = (node.parent, node.previous, node.next);
        if let Some(previous) = previous {
            self.nodes[previous].next = next;
        } else if let Some(parent) = parent {
            self.nodes[parent].first = next;
        }
        if let Some(next) = next {
            self.nodes[next].previous = previous;
        } else if let Some(parent) = parent {
            self.nodes[parent].last = previous;
        }
        let node = &mut self.nodes[id];
        node.parent = None;
        node.previous = None;
        node.next = None;
    }

    fn attach(&mut self, parent: usize, id: usize, before: Option<usize>) {
        if parent >= self.nodes.len() || id >= self.nodes.len() || parent == id {
            return;
        }
        if let Some(before) = before
            && (before >= self.nodes.len()
                || before == id
                || self.nodes[before].parent != Some(parent))
        {
            return;
        }
        // html5ever's TreeSink contract supplies a tree insertion, never an ancestor
        // under its descendant. IDs are private, so callers cannot mutate links.
        // Layout also validates visited identities to fail closed on internal defects.
        self.detach(id);
        let previous = before
            .and_then(|sibling| self.nodes[sibling].previous)
            .or_else(|| {
                if before.is_none() {
                    self.nodes[parent].last
                } else {
                    None
                }
            });
        let node = &mut self.nodes[id];
        node.parent = Some(parent);
        node.previous = previous;
        node.next = before;
        if let Some(previous) = previous {
            self.nodes[previous].next = Some(id);
        } else {
            self.nodes[parent].first = Some(id);
        }
        if let Some(before) = before {
            self.nodes[before].previous = Some(id);
        } else {
            self.nodes[parent].last = Some(id);
        }
    }

    fn is_html_element(&self, id: usize, local: &str) -> bool {
        matches!(
            self.nodes.get(id).map(|node| &node.kind),
            Some(NodeKind::Element { name, .. }) if {
                let tag: &str = name.local.as_ref();
                name.ns == ns!(html) && tag == local
            }
        )
    }

    fn has_attribute(&self, id: usize, local: &str) -> bool {
        match self.nodes.get(id).map(|node| &node.kind) {
            Some(NodeKind::Element { attrs, .. }) => attrs.iter().any(|attribute| {
                let name: &str = attribute.name.local.as_ref();
                name == local
            }),
            _ => false,
        }
    }

    fn child_ids(&self, parent: usize) -> Option<Vec<usize>> {
        let mut children = Vec::new();
        let mut current = self.nodes.get(parent)?.first;
        let mut steps = 0;
        while let Some(id) = current {
            steps += 1;
            if steps > MAX_NODES {
                return None;
            }
            let child = self.nodes.get(id)?;
            if child.parent != Some(parent) {
                return None;
            }
            children.push(id);
            current = child.next;
        }
        Some(children)
    }

    // Pinned RcDom: option element nearest ancestor select, with at most one optgroup.
    fn nearest_select(&self, option: usize) -> Option<usize> {
        let mut current = self.nodes.get(option)?.parent;
        let mut saw_optgroup = false;
        for _ in 0..MAX_NODES {
            let id = current?;
            let node = self.nodes.get(id)?;
            if let NodeKind::Element { name, .. } = &node.kind
                && name.ns == ns!(html)
            {
                match name.local.as_ref() {
                    "datalist" | "hr" | "option" => return None,
                    "optgroup" => {
                        if saw_optgroup {
                            return None;
                        }
                        saw_optgroup = true;
                    }
                    "select" => return Some(id),
                    _ => {}
                }
            }
            current = node.parent;
        }
        None
    }

    // Static selectedness after parsing: the last explicitly selected option wins;
    // otherwise the first enabled option is selected at the default display size of 1.
    fn selected_option(&self, select: usize) -> Option<usize> {
        let mut pending = self
            .child_ids(select)?
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        let mut first_enabled = None;
        let mut last_selected = None;
        for _ in 0..MAX_NODES {
            let Some(id) = pending.pop() else {
                break;
            };
            if self.is_html_element(id, "option") {
                if self.nearest_select(id) != Some(select) {
                    continue;
                }
                if first_enabled.is_none() && !self.has_attribute(id, "disabled") {
                    first_enabled = Some(id);
                }
                if self.has_attribute(id, "selected") {
                    last_selected = Some(id);
                }
                continue;
            }
            if self.is_html_element(id, "select")
                || self.is_html_element(id, "hr")
                || self.is_html_element(id, "datalist")
            {
                continue;
            }
            pending.extend(self.child_ids(id)?.into_iter().rev());
        }
        if last_selected.is_some() {
            return last_selected;
        }
        let display_size_one = match &self.nodes.get(select)?.kind {
            NodeKind::Element { attrs, .. } => {
                match attrs.iter().find(|attribute| {
                    let name: &str = attribute.name.local.as_ref();
                    name == "size"
                }) {
                    None => true,
                    Some(attribute) => match attribute.value.parse::<u64>() {
                        Ok(size) => size <= 1,
                        Err(_) => true,
                    },
                }
            }
            _ => false,
        };
        display_size_one.then_some(first_enabled).flatten()
    }

    // The pinned hook excludes multiple selects. RcDom leaves live disabledness
    // unimplemented; this static walk finds the first selectedcontent in tree order.
    fn enabled_selectedcontent(&self, select: usize) -> Option<usize> {
        if self.has_attribute(select, "multiple") {
            return None;
        }
        let mut pending = self
            .child_ids(select)?
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        for _ in 0..MAX_NODES {
            let id = pending.pop()?;
            if self.is_html_element(id, "selectedcontent") {
                return Some(id);
            }
            pending.extend(self.child_ids(id)?.into_iter().rev());
        }
        None
    }

    // Clone into detached fresh nodes before replacing anything. In particular, a
    // selectedcontent inside the source subtree cannot invalidate this traversal.
    fn clone_children(&mut self, source_parent: usize) -> Option<Vec<usize>> {
        let source_len = self.nodes.len();
        let mut pending = self
            .child_ids(source_parent)?
            .into_iter()
            .rev()
            .map(|id| (id, None))
            .collect::<Vec<_>>();
        let mut roots = Vec::new();
        let mut steps = 0;
        while let Some((source, destination)) = pending.pop() {
            steps += 1;
            if steps > MAX_NODES || source >= source_len || self.overflow {
                return None;
            }
            let mut kind = self.nodes.get(source)?.kind.clone();
            let template_source = match &mut kind {
                NodeKind::Element {
                    template_contents, ..
                } => template_contents.take(),
                NodeKind::Document => return None,
                _ => None,
            };
            let clone = self.allocate(kind);
            if self.overflow {
                return None;
            }
            if let Some(parent) = destination {
                self.attach(parent, clone, None);
            } else {
                roots.push(clone);
            }
            if let Some(template_source) = template_source {
                let template_clone = self.allocate(NodeKind::Document);
                if self.overflow {
                    return None;
                }
                let Some(Node {
                    kind:
                        NodeKind::Element {
                            template_contents, ..
                        },
                    ..
                }) = self.nodes.get_mut(clone)
                else {
                    return None;
                };
                *template_contents = Some(template_clone);
                pending.extend(
                    self.child_ids(template_source)?
                        .into_iter()
                        .rev()
                        .map(|id| (id, Some(template_clone))),
                );
            }
            pending.extend(
                self.child_ids(source)?
                    .into_iter()
                    .rev()
                    .map(|id| (id, Some(clone))),
            );
        }
        Some(roots)
    }

    fn replace_children(&mut self, parent: usize, roots: Vec<usize>) -> bool {
        let Some(existing) = self.child_ids(parent) else {
            return false;
        };
        for child in existing {
            self.detach(child);
        }
        for root in roots {
            self.attach(parent, root, None);
        }
        true
    }

    fn clone_option_into_selectedcontent(&mut self, option: usize) {
        if self.selectedcontent_cloned_options.contains(&option) {
            return;
        }
        let Some(select) = self.nearest_select(option) else {
            return;
        };
        let Some(selectedcontent) = self.enabled_selectedcontent(select) else {
            return;
        };
        if self.selected_option(select) != Some(option) {
            return;
        }
        let Some(clones) = self.clone_children(option) else {
            return;
        };
        if !self.overflow && self.replace_children(selectedcontent, clones) {
            self.selectedcontent_cloned_options.insert(option);
        }
    }

    fn clone_unreported_options_into_selectedcontent(&mut self) {
        if !self
            .nodes
            .iter()
            .any(|node| matches!(&node.kind, NodeKind::Element { name, .. } if name.ns == ns!(html) && name.local == local_name!("selectedcontent")))
        {
            return;
        }
        // html5ever 0.40.1 does not report implicit option closes or EOF to the
        // selectedcontent hook. Reconcile those nodes once the static tree is complete.
        let parser_and_hook_nodes = self.nodes.len();
        for option in 0..parser_and_hook_nodes {
            if self.overflow {
                break;
            }
            if self.is_html_element(option, "option") {
                self.clone_option_into_selectedcontent(option);
            }
        }
    }

    fn append(&mut self, parent: usize, child: NodeOrText<Handle>, before: Option<usize>) {
        if parent >= self.nodes.len() {
            return;
        }
        match child {
            NodeOrText::AppendNode(handle) => self.attach(parent, handle.id, before),
            NodeOrText::AppendText(text) => {
                let previous = before
                    .and_then(|id| self.nodes.get(id).and_then(|node| node.previous))
                    .or_else(|| {
                        if before.is_none() {
                            self.nodes[parent].last
                        } else {
                            None
                        }
                    });
                if let Some(previous) = previous
                    && let NodeKind::Text(existing) = &mut self.nodes[previous].kind
                {
                    existing.push_str(&text);
                    return;
                }
                let id = self.allocate(NodeKind::Text(text.to_string()));
                self.attach(parent, id, before);
            }
        }
    }
}

struct Sink {
    state: RefCell<State>,
}

impl Sink {
    fn new() -> Self {
        Self {
            state: RefCell::new(State {
                nodes: vec![Node::new(NodeKind::Document)],
                next_id: 1,
                overflow: false,
                diagnostics: vec![Diagnostic::UnsupportedFeature(
                    "Approximate text metrics; no scripting or interactive DOM",
                )],
                selectedcontent_cloned_options: HashSet::new(),
            }),
        }
    }
}

impl TreeSink for Sink {
    type Handle = Handle;
    type Output = Result<Document, Error>;
    type ElemName<'a> = &'a QualName;

    fn finish(self) -> Self::Output {
        let mut state = self.state.into_inner();
        state.clone_unreported_options_into_selectedcontent();
        if state.overflow {
            return Err(Error::NodeLimit);
        }
        // Styles are resolved once after tree construction and are bounded by the node arena.
        let (styles, unsupported_css) = crate::css::compute_styles(&state.nodes);
        let mut diagnostics = state.diagnostics;
        if unsupported_css > 0
            && !diagnostics.contains(&Diagnostic::UnsupportedCss)
            && diagnostics.len() < MAX_DIAGNOSTICS
        {
            diagnostics.push(Diagnostic::UnsupportedCss);
        }
        Ok(Document {
            nodes: state.nodes,
            styles,
            diagnostics,
            unsupported_css,
            inspection_root: 0,
        })
    }

    fn parse_error(&self, msg: Cow<'static, str>) {
        self.state
            .borrow_mut()
            .diagnose(Diagnostic::HtmlParseError(msg.chars().take(512).collect()));
    }

    fn get_document(&self) -> Handle {
        Handle::plain(0)
    }
    fn elem_name<'a>(&'a self, target: &'a Handle) -> &'a QualName {
        &target.name
    }
    fn same_node(&self, x: &Handle, y: &Handle) -> bool {
        x.id == y.id
    }
    fn set_quirks_mode(&self, mode: QuirksMode) {
        if mode != QuirksMode::NoQuirks {
            self.state
                .borrow_mut()
                .diagnose(Diagnostic::UnsupportedFeature("Quirks-mode layout"));
        }
    }

    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags) -> Handle {
        let mut state = self.state.borrow_mut();
        let tag: &str = name.local.as_ref();
        if name.ns != ns!(html)
            || !matches!(
                tag,
                "html"
                    | "head"
                    | "body"
                    | "title"
                    | "style"
                    | "meta"
                    | "p"
                    | "div"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "br"
                    | "span"
                    | "b"
                    | "strong"
            )
        {
            state.diagnose(Diagnostic::UnsupportedElement(tag.to_string()));
        }
        let template = flags.template.then(|| state.allocate(NodeKind::Document));
        let id = state.allocate(NodeKind::Element {
            name: name.clone(),
            attrs,
            template_contents: template,
        });
        Handle {
            id,
            name,
            template,
            mathml_integration: flags.mathml_annotation_xml_integration_point,
        }
    }

    fn create_comment(&self, text: StrTendril) -> Handle {
        Handle::plain(
            self.state
                .borrow_mut()
                .allocate(NodeKind::Comment(text.to_string())),
        )
    }

    fn create_pi(&self, target: StrTendril, data: StrTendril) -> Handle {
        Handle::plain(
            self.state
                .borrow_mut()
                .allocate(NodeKind::ProcessingInstruction {
                    target: target.to_string(),
                    data: data.to_string(),
                }),
        )
    }

    fn append(&self, parent: &Handle, child: NodeOrText<Handle>) {
        self.state.borrow_mut().append(parent.id, child, None);
    }

    fn append_before_sibling(&self, sibling: &Handle, child: NodeOrText<Handle>) {
        let parent = self
            .state
            .borrow()
            .nodes
            .get(sibling.id)
            .and_then(|node| node.parent);
        if let Some(parent) = parent {
            self.state
                .borrow_mut()
                .append(parent, child, Some(sibling.id));
        }
    }

    fn append_based_on_parent_node(
        &self,
        element: &Handle,
        prev_element: &Handle,
        child: NodeOrText<Handle>,
    ) {
        if self
            .state
            .borrow()
            .nodes
            .get(element.id)
            .and_then(|node| node.parent)
            .is_some()
        {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        let mut state = self.state.borrow_mut();
        let id = state.allocate(NodeKind::Doctype {
            name: name.to_string(),
            public_id: public_id.to_string(),
            system_id: system_id.to_string(),
        });
        state.attach(0, id, None);
    }

    fn get_template_contents(&self, target: &Handle) -> Handle {
        Handle::plain(target.template.unwrap_or(target.id))
    }

    fn add_attrs_if_missing(&self, target: &Handle, attrs: Vec<Attribute>) {
        let mut state = self.state.borrow_mut();
        if let Some(Node {
            kind: NodeKind::Element {
                attrs: existing, ..
            },
            ..
        }) = state.nodes.get_mut(target.id)
        {
            for attr in attrs {
                if !existing.iter().any(|item| item.name == attr.name) {
                    existing.push(attr)
                }
            }
        }
    }

    fn remove_from_parent(&self, target: &Handle) {
        self.state.borrow_mut().detach(target.id);
    }

    fn reparent_children(&self, node: &Handle, new_parent: &Handle) {
        let mut state = self.state.borrow_mut();
        if node.id == new_parent.id || new_parent.id >= state.nodes.len() {
            return;
        }
        // At most MAX_NODES moves; iteration stays bounded even on an internal link defect.
        for _ in 0..MAX_NODES {
            let child = state.nodes.get(node.id).and_then(|node| node.first);
            let Some(child) = child else { break };
            state.attach(new_parent.id, child, None);
        }
    }

    fn is_mathml_annotation_xml_integration_point(&self, handle: &Handle) -> bool {
        handle.mathml_integration
    }

    fn allow_declarative_shadow_roots(&self, _parent: &Handle) -> bool {
        false
    }

    fn maybe_clone_an_option_into_selectedcontent(&self, option: &Self::Handle) {
        self.state
            .borrow_mut()
            .clone_option_into_selectedcontent(option.id);
    }
}

#[cfg(test)]
mod tests {
    use html5ever::local_name;

    use super::{Document, NodeKind};

    // Value: protects=doctype/comment retention and template contents allocation; fails_when=TreeSink callbacks discard required nodes; why_new=public layout intentionally hides them; seam=none; no_claim=canonical serialization by itself.
    #[test]
    fn parser_preserves_non_rendered_nodes_and_template_identity() {
        let document = Document::parse(
            "<!doctype html PUBLIC 'pub' 'sys'><!--before--><template><b>inside</b></template><p>visible",
        )
        .unwrap();
        assert!(document.nodes.iter().any(|node| matches!(&node.kind,
            NodeKind::Doctype { name, public_id, system_id }
            if name == "html" && public_id == "pub" && system_id == "sys")));
        assert!(document.nodes.iter().any(|node| matches!(&node.kind,
            NodeKind::Comment(text) if text == "before")));
        let contents = document
            .nodes
            .iter()
            .find_map(|node| match &node.kind {
                NodeKind::Element {
                    name,
                    template_contents,
                    ..
                } if name.local == local_name!("template") => *template_contents,
                _ => None,
            })
            .unwrap();
        assert!(matches!(document.nodes[contents].kind, NodeKind::Document));
        assert!(document.nodes[contents].first.is_some());
    }
}

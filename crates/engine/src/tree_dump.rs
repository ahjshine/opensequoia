//! Canonical .dat tree output. The format is specified by WPT's tree-construction runner:
//! https://github.com/web-platform-tests/wpt/blob/31d1feb70e2d02f25ae26254caba19c8de0b56e0/html/syntax/parsing/resources/test.js

use html5ever::{QualName, ns};

use crate::dom::{Document, NodeKind};
use crate::{Error, MAX_NODES, MAX_TREE_DUMP_BYTES};

enum Event {
    Node(usize, usize),
    Template(usize, usize),
}

struct Output(String);

impl Output {
    fn line(&mut self, indent: usize, parts: &[&str]) -> Result<(), Error> {
        let length = parts.iter().try_fold(indent + 2, |length, part| {
            length.checked_add(part.len()).ok_or(Error::TreeDumpLimit)
        })?;
        if length > MAX_TREE_DUMP_BYTES - self.0.len() {
            return Err(Error::TreeDumpLimit);
        }
        self.0.push('|');
        self.0.extend(std::iter::repeat_n(' ', indent));
        for part in parts {
            self.0.push_str(part);
        }
        self.0.push('\n');
        Ok(())
    }
}

fn qualified(name: &QualName) -> String {
    let prefix = match name.ns {
        ns!(svg) => "svg ",
        ns!(mathml) => "math ",
        ns!(xlink) => "xlink ",
        ns!(xml) => "xml ",
        ns!(xmlns) => "xmlns ",
        _ => "",
    };
    format!("{prefix}{}", name.local)
}

fn children(
    document: &Document,
    parent: usize,
    indent: usize,
    events: &mut Vec<Event>,
) -> Result<(), Error> {
    let node = document.nodes.get(parent).ok_or(Error::InvalidTree)?;
    let mut next = None;
    let mut current = node.last;
    let mut count = 0;
    while let Some(id) = current {
        count += 1;
        if count > MAX_NODES || events.len() >= MAX_NODES {
            return Err(Error::InvalidTree);
        }
        let child = document.nodes.get(id).ok_or(Error::InvalidTree)?;
        if child.parent != Some(parent) || child.next != next {
            return Err(Error::InvalidTree);
        }
        events.push(Event::Node(id, indent));
        next = Some(id);
        current = child.previous;
    }
    if next != node.first {
        return Err(Error::InvalidTree);
    }
    Ok(())
}

pub(crate) fn serialize(document: &Document, root: usize) -> Result<String, Error> {
    let mut output = Output(String::new());
    let mut events = Vec::new();
    let mut visited = vec![false; document.nodes.len()];
    *visited.get_mut(root).ok_or(Error::InvalidTree)? = true;
    children(document, root, 1, &mut events)?;
    let mut steps = 0;
    while let Some(event) = events.pop() {
        steps += 1;
        if steps > MAX_NODES {
            return Err(Error::InvalidTree);
        }
        let (id, indent) = match event {
            Event::Node(id, indent) | Event::Template(id, indent) => (id, indent),
        };
        let node = document.nodes.get(id).ok_or(Error::InvalidTree)?;
        if visited[id] {
            return Err(Error::InvalidTree);
        }
        visited[id] = true;
        match event {
            Event::Template(_, _) => {
                if !matches!(node.kind, NodeKind::Document) || node.parent.is_some() {
                    return Err(Error::InvalidTree);
                }
                output.line(indent, &["content"])?;
            }
            Event::Node(_, _) => match &node.kind {
                NodeKind::Document => return Err(Error::InvalidTree),
                NodeKind::Text(text) => output.line(indent, &["\"", text, "\""])?,
                NodeKind::Comment(text) => output.line(indent, &["<!-- ", text, " -->"])?,
                NodeKind::Doctype {
                    name,
                    public_id,
                    system_id,
                } => {
                    if public_id.is_empty() && system_id.is_empty() {
                        output.line(indent, &["<!DOCTYPE ", name, ">"])?;
                    } else {
                        output.line(
                            indent,
                            &[
                                "<!DOCTYPE ",
                                name,
                                " \"",
                                public_id,
                                "\" \"",
                                system_id,
                                "\">",
                            ],
                        )?;
                    }
                }
                NodeKind::ProcessingInstruction { target, data } => {
                    output.line(indent, &["<?", target, " ", data, ">"])?;
                }
                NodeKind::Element {
                    name,
                    attrs,
                    template_contents,
                } => {
                    output.line(indent, &["<", &qualified(name), ">"])?;
                    let mut attrs: Vec<_> = attrs
                        .iter()
                        .map(|attr| (qualified(&attr.name), &attr.value))
                        .collect();
                    // JavaScript's string ordering uses UTF-16 units, including astral names.
                    attrs.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
                    for (name, value) in attrs {
                        output.line(indent + 2, &[&name, "=\"", value, "\""])?;
                    }
                    if let Some(contents) = template_contents {
                        events.push(Event::Template(*contents, indent + 2));
                    }
                }
            },
        }
        children(document, id, indent + 2, &mut events)?;
    }
    output.0.pop(); // Canonical output has no trailing newline; an empty tree stays empty.
    Ok(output.0)
}

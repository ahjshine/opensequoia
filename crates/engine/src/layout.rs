//! Deliberately limited static block/text flow. Glyph advances are estimates, not shaping.
//! Block boxes honour the documented CSS subset (margins, padding, border width, alignment);
//! background-color and border colours are resolved but not painted by this display list.

use html5ever::ns;

use crate::css::{ComputedStyle, DEFAULT_TEXT_COLOR, DisplayValue, TextAlign};
use crate::dom::{Document, NodeKind};
use crate::{Color, DisplayItem, DisplayList, Error, MAX_NODES, TextRun};

const MARGIN: u32 = 16;
const MAX_HEIGHT: u32 = 1_000_000;
const MAX_RUNS: usize = 65_536;

#[derive(Clone, Copy)]
struct Style {
    size: u32,
    bold: bool,
    color: Color,
}

impl Style {
    fn initial() -> Self {
        Self {
            size: 16,
            bold: false,
            color: DEFAULT_TEXT_COLOR,
        }
    }

    fn from_computed(computed: ComputedStyle) -> Self {
        Self {
            size: computed.font_size,
            bold: computed.bold,
            color: computed.color,
        }
    }
}

#[derive(Clone, Copy)]
struct Exit {
    block: bool,
    left: u32,
    right: u32,
    align: TextAlign,
    bottom: u32,
}

enum Event {
    Enter(usize, Style),
    Exit(Exit),
}

struct Flow {
    left: u32,
    right: u32,
    x: u32,
    y: u32,
    line_height: u32,
    pending_space: bool,
    align: TextAlign,
    line_start: usize,
    items: Vec<DisplayItem>,
}

impl Flow {
    /// Shift the finished line for text-align. Lines hold only text runs in this increment.
    fn finish_line(&mut self) {
        if self.line_start >= self.items.len() {
            return;
        }
        let available = self.right.saturating_sub(self.left);
        let mut used = 0;
        for DisplayItem::Text(run) in &self.items[self.line_start..] {
            let end = u32::try_from(run.x).unwrap_or(0).saturating_add(run.width);
            used = used.max(end.saturating_sub(self.left));
        }
        let shift = match self.align {
            TextAlign::Left => 0,
            TextAlign::Center => available.saturating_sub(used) / 2,
            TextAlign::Right => available.saturating_sub(used),
        };
        let Ok(shift) = i32::try_from(shift) else {
            return;
        };
        if shift == 0 {
            return;
        }
        for DisplayItem::Text(run) in &mut self.items[self.line_start..] {
            run.x = run.x.saturating_add(shift);
        }
    }

    fn newline(&mut self, force: bool) -> Result<(), Error> {
        if self.x > self.left || force {
            self.finish_line();
            self.y = self
                .y
                .checked_add(self.line_height.max(24))
                .ok_or(Error::LayoutLimit)?;
            if self.y > MAX_HEIGHT {
                return Err(Error::LayoutLimit);
            }
        }
        self.x = self.left;
        self.line_height = 0;
        self.pending_space = false;
        self.line_start = self.items.len();
        Ok(())
    }

    fn word(&mut self, word: &str, style: Style) -> Result<(), Error> {
        if word.is_empty() {
            return Ok(());
        }
        let advance = (style.size * 3 / 5).max(1);
        let right = self.right;
        let available = right.saturating_sub(self.left).max(advance);
        let chars_per_line = (available / advance).max(1) as usize;
        let mut chars = word.chars().peekable();
        while chars.peek().is_some() {
            let part: String = chars.by_ref().take(chars_per_line).collect();
            let run_width = part.chars().count() as u32 * advance;
            let space = if self.pending_space && self.x > self.left {
                advance
            } else {
                0
            };
            if self.x + space + run_width > right {
                self.newline(false)?
            }
            if self.pending_space && self.x > self.left {
                self.x += advance
            }
            let height = style.size + 8;
            if self.y + height > MAX_HEIGHT || self.items.len() >= MAX_RUNS {
                return Err(Error::LayoutLimit);
            }
            self.items.push(DisplayItem::Text(TextRun {
                text: part,
                x: self.x as i32,
                y: self.y as i32,
                width: run_width,
                height,
                font_size: style.size,
                bold: style.bold,
                color: style.color,
            }));
            self.x += run_width;
            self.line_height = self.line_height.max(height);
            self.pending_space = false;
            if chars.peek().is_some() {
                self.newline(false)?;
            }
        }
        Ok(())
    }

    fn text(&mut self, text: &str, style: Style) -> Result<(), Error> {
        let mut word_start = 0;
        for (index, character) in text.char_indices() {
            if matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{000c}') {
                self.word(&text[word_start..index], style)?;
                self.pending_space = true;
                word_start = index + character.len_utf8();
            }
        }
        self.word(&text[word_start..], style)
    }
}

pub(crate) fn layout(document: &Document, width: u32) -> Result<DisplayList, Error> {
    if !(64..=8192).contains(&width) {
        return Err(Error::InvalidViewport);
    }
    let mut flow = Flow {
        left: MARGIN,
        right: width - MARGIN,
        x: MARGIN,
        y: MARGIN,
        line_height: 0,
        pending_space: false,
        align: TextAlign::Left,
        line_start: 0,
        items: Vec::new(),
    };
    let mut events = vec![Event::Enter(0, Style::initial())];
    let mut visited = vec![false; document.nodes.len()];
    let mut steps = 0;
    while let Some(event) = events.pop() {
        steps += 1;
        if steps > MAX_NODES * 2 {
            return Err(Error::InvalidTree);
        }
        match event {
            Event::Exit(exit) => {
                if exit.block {
                    flow.newline(false)?;
                    flow.y = flow.y.checked_add(exit.bottom).ok_or(Error::LayoutLimit)?;
                    if flow.y > MAX_HEIGHT {
                        return Err(Error::LayoutLimit);
                    }
                    flow.left = exit.left;
                    flow.right = exit.right;
                    flow.align = exit.align;
                    flow.x = flow.left;
                }
            }
            Event::Enter(id, inherited) => {
                let Some(node) = document.nodes.get(id) else {
                    return Err(Error::InvalidTree);
                };
                if visited[id] {
                    return Err(Error::InvalidTree);
                }
                visited[id] = true;
                let mut style = inherited;
                let mut exit = Exit {
                    block: false,
                    left: flow.left,
                    right: flow.right,
                    align: flow.align,
                    bottom: 0,
                };
                match &node.kind {
                    NodeKind::Text(text) => {
                        flow.text(text, style)?;
                        continue;
                    }
                    NodeKind::Comment(_)
                    | NodeKind::Doctype { .. }
                    | NodeKind::ProcessingInstruction { .. } => continue,
                    NodeKind::Document => {}
                    NodeKind::Element { name, .. } => {
                        let tag = name.local.as_ref();
                        if name.ns != ns!(html)
                            || matches!(tag, "head" | "script" | "style" | "template")
                        {
                            continue;
                        }
                        let computed = *document.styles.get(id).ok_or(Error::InvalidTree)?;
                        if computed.display == DisplayValue::None {
                            continue;
                        }
                        style = Style::from_computed(computed);
                        if tag == "br" {
                            flow.newline(true)?;
                            continue;
                        }
                        if computed.display == DisplayValue::Block {
                            flow.newline(false)?;
                            exit.block = true;
                            exit.left = flow.left;
                            exit.right = flow.right;
                            exit.align = flow.align;
                            let border = computed.effective_border();
                            flow.left = flow.left.saturating_add(
                                computed.margin.left + computed.padding.left + border,
                            );
                            flow.right = flow
                                .right
                                .saturating_sub(
                                    computed.margin.right + computed.padding.right + border,
                                )
                                .max(flow.left.saturating_add(1));
                            flow.y = flow
                                .y
                                .checked_add(computed.margin.top + computed.padding.top + border)
                                .ok_or(Error::LayoutLimit)?;
                            if flow.y > MAX_HEIGHT {
                                return Err(Error::LayoutLimit);
                            }
                            flow.x = flow.left;
                            flow.align = computed.align;
                            exit.bottom = computed.padding.bottom + border + computed.margin.bottom;
                        }
                    }
                }
                events.push(Event::Exit(exit));
                // Reverse sibling walk pushes children in document order without recursion.
                let mut child = node.last;
                let mut siblings = 0;
                while let Some(child_id) = child {
                    siblings += 1;
                    if siblings > MAX_NODES {
                        return Err(Error::InvalidTree);
                    }
                    let Some(child_node) = document.nodes.get(child_id) else {
                        return Err(Error::InvalidTree);
                    };
                    if child_node.parent != Some(id) {
                        return Err(Error::InvalidTree);
                    }
                    events.push(Event::Enter(child_id, style));
                    child = child_node.previous;
                }
            }
        }
    }
    flow.finish_line();
    let height = (flow.y + flow.line_height + MARGIN).min(MAX_HEIGHT);
    Ok(DisplayList {
        width,
        height,
        items: flow.items,
    })
}

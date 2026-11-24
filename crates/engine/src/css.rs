//! Deliberately small CSS subset: `<style>` element text and `style=""` attributes only.
//!
//! Supported selectors: type, `.class`, `#id`, type.class/type#id and comma-separated lists.
//! Supported declarations: color, background-color, font-size (1-256 px), font-weight
//! (normal|bold), text-align (left|center|right), margin/margin-* and padding/padding-*
//! (1-4 px values), border/border-width/border-style(solid|none)/border-color, display
//! (block|inline|none). Everything else is skipped and counted, never fatal.
//!
//! This is explicitly not CSS conformance: no combinators, pseudo-classes, attribute or
//! universal selectors, `!important`, inheritance of box properties, or units other than px.

use html5ever::ns;

use crate::dom::{Node, NodeKind};
use crate::{Color, MAX_NODES};

/// Inline `style` attributes outrank every selector in the subset.
const INLINE_SPECIFICITY: u32 = 1000;
const MAX_CSS_RULES: usize = 4096;
const MAX_CSS_SELECTOR_BYTES: usize = 256;
/// Explicitly bounded px input; absurd lengths become unsupported rather than fatal.
const MAX_PX: u32 = 100_000;
/// IPC display schema v1 represents `font_size` as 1..=256; larger CSS values are
/// unsupported in this subset rather than producing a display run the wire rejects.
const MAX_FONT_SIZE: u32 = 256;

pub(crate) const DEFAULT_TEXT_COLOR: Color = Color {
    r: 28,
    g: 41,
    b: 35,
};
const DEFAULT_FONT_SIZE: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DisplayValue {
    Block,
    Inline,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BorderStyle {
    None,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Edges {
    pub(crate) top: u32,
    pub(crate) right: u32,
    pub(crate) bottom: u32,
    pub(crate) left: u32,
}

/// Fully resolved values for one element: inherited text properties plus declared box values.
/// Only text properties inherit; box properties restart at zero on every element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ComputedStyle {
    pub(crate) color: Color,
    pub(crate) background: Option<Color>,
    pub(crate) font_size: u32,
    pub(crate) bold: bool,
    pub(crate) align: TextAlign,
    pub(crate) display: DisplayValue,
    pub(crate) margin: Edges,
    pub(crate) padding: Edges,
    pub(crate) border_width: u32,
    pub(crate) border_style: BorderStyle,
    pub(crate) border_color: Color,
}

impl ComputedStyle {
    fn initial() -> Self {
        Self {
            color: DEFAULT_TEXT_COLOR,
            background: None,
            font_size: DEFAULT_FONT_SIZE,
            bold: false,
            align: TextAlign::Left,
            display: DisplayValue::Inline,
            margin: Edges::default(),
            padding: Edges::default(),
            border_width: 0,
            border_style: BorderStyle::None,
            border_color: DEFAULT_TEXT_COLOR,
        }
    }

    fn inherited(parent: &Self) -> Self {
        Self {
            color: parent.color,
            font_size: parent.font_size,
            bold: parent.bold,
            align: parent.align,
            ..Self::initial()
        }
    }

    /// `border-style: none` computes to a zero width, as in CSS.
    pub(crate) fn effective_border(&self) -> u32 {
        match self.border_style {
            BorderStyle::None => 0,
            BorderStyle::Solid => self.border_width,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Selector {
    element: Option<String>,
    class: Option<String>,
    id: Option<String>,
    specificity: u32,
}

impl Selector {
    /// Simple specificity integer: type 1, class 10, id 100, inline attribute 1000.
    fn matches(&self, tag: &str, class: &str, id: &str) -> bool {
        if let Some(element) = &self.element
            && element != tag
        {
            return false;
        }
        if let Some(name) = &self.class
            && !class.split_whitespace().any(|word| word == name)
        {
            return false;
        }
        if let Some(name) = &self.id
            && id != name
        {
            return false;
        }
        true
    }
}

/// One declaration block after shorthand expansion; a later declaration in the same block wins.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Declared {
    color: Option<Color>,
    background: Option<Color>,
    font_size: Option<u32>,
    font_weight: Option<bool>,
    text_align: Option<TextAlign>,
    display: Option<DisplayValue>,
    margin: [Option<u32>; 4],
    padding: [Option<u32>; 4],
    border_width: Option<u32>,
    border_style: Option<BorderStyle>,
    border_color: Option<Color>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Slot<T> {
    specificity: u32,
    order: u32,
    value: T,
}

#[derive(Debug, Default)]
struct Cascade {
    color: Option<Slot<Color>>,
    background: Option<Slot<Color>>,
    font_size: Option<Slot<u32>>,
    font_weight: Option<Slot<bool>>,
    text_align: Option<Slot<TextAlign>>,
    display: Option<Slot<DisplayValue>>,
    margin: [Option<Slot<u32>>; 4],
    padding: [Option<Slot<u32>>; 4],
    border_width: Option<Slot<u32>>,
    border_style: Option<Slot<BorderStyle>>,
    border_color: Option<Slot<Color>>,
}

fn set<T>(slot: &mut Option<Slot<T>>, specificity: u32, order: u32, value: Option<T>) {
    let Some(value) = value else {
        return;
    };
    let wins = match slot {
        None => true,
        Some(current) => (specificity, order) > (current.specificity, current.order),
    };
    if wins {
        *slot = Some(Slot {
            specificity,
            order,
            value,
        });
    }
}

impl Cascade {
    fn merge(&mut self, declared: &Declared, specificity: u32, order: u32) {
        set(&mut self.color, specificity, order, declared.color);
        set(
            &mut self.background,
            specificity,
            order,
            declared.background,
        );
        set(&mut self.font_size, specificity, order, declared.font_size);
        set(
            &mut self.font_weight,
            specificity,
            order,
            declared.font_weight,
        );
        set(
            &mut self.text_align,
            specificity,
            order,
            declared.text_align,
        );
        set(&mut self.display, specificity, order, declared.display);
        for index in 0..4 {
            set(
                &mut self.margin[index],
                specificity,
                order,
                declared.margin[index],
            );
            set(
                &mut self.padding[index],
                specificity,
                order,
                declared.padding[index],
            );
        }
        set(
            &mut self.border_width,
            specificity,
            order,
            declared.border_width,
        );
        set(
            &mut self.border_style,
            specificity,
            order,
            declared.border_style,
        );
        set(
            &mut self.border_color,
            specificity,
            order,
            declared.border_color,
        );
    }

    fn resolve(&self, base: ComputedStyle) -> ComputedStyle {
        let color = self.color.map_or(base.color, |slot| slot.value);
        ComputedStyle {
            color,
            background: self.background.map(|slot| slot.value),
            font_size: self.font_size.map_or(base.font_size, |slot| slot.value),
            bold: self.font_weight.map_or(base.bold, |slot| slot.value),
            align: self.text_align.map_or(base.align, |slot| slot.value),
            display: self.display.map_or(base.display, |slot| slot.value),
            margin: Edges {
                top: self.margin[0].map_or(0, |slot| slot.value),
                right: self.margin[1].map_or(0, |slot| slot.value),
                bottom: self.margin[2].map_or(0, |slot| slot.value),
                left: self.margin[3].map_or(0, |slot| slot.value),
            },
            padding: Edges {
                top: self.padding[0].map_or(0, |slot| slot.value),
                right: self.padding[1].map_or(0, |slot| slot.value),
                bottom: self.padding[2].map_or(0, |slot| slot.value),
                left: self.padding[3].map_or(0, |slot| slot.value),
            },
            border_width: self.border_width.map_or(0, |slot| slot.value),
            border_style: self
                .border_style
                .map_or(BorderStyle::None, |slot| slot.value),
            border_color: self.border_color.map_or(color, |slot| slot.value),
        }
    }
}

fn parse_px(value: &str) -> Option<u32> {
    let value = value.trim();
    if value == "0" {
        return Some(0);
    }
    let number = value.strip_suffix("px")?;
    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let px: u32 = number.parse().ok()?;
    (px <= MAX_PX).then_some(px)
}

fn parse_hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn named_color(value: &str) -> Option<Color> {
    let (r, g, b) = match value {
        "aqua" => (0, 255, 255),
        "black" => (0, 0, 0),
        "blue" => (0, 0, 255),
        "fuchsia" => (255, 0, 255),
        "gray" => (128, 128, 128),
        "green" => (0, 128, 0),
        "lime" => (0, 255, 0),
        "maroon" => (128, 0, 0),
        "navy" => (0, 0, 128),
        "olive" => (128, 128, 0),
        "purple" => (128, 0, 128),
        "red" => (255, 0, 0),
        "silver" => (192, 192, 192),
        "teal" => (0, 128, 128),
        "white" => (255, 255, 255),
        "yellow" => (255, 255, 0),
        _ => return None,
    };
    Some(Color { r, g, b })
}

fn parse_color(value: &str) -> Option<Color> {
    let Some(hex) = value.strip_prefix('#') else {
        return named_color(value);
    };
    let bytes = hex.as_bytes();
    match bytes.len() {
        3 => {
            let mut channels = [0u8; 3];
            for (index, byte) in bytes.iter().enumerate() {
                let digit = parse_hex_digit(*byte)?;
                channels[index] = digit * 17;
            }
            Some(Color {
                r: channels[0],
                g: channels[1],
                b: channels[2],
            })
        }
        6 => {
            let mut channels = [0u8; 3];
            for (index, channel) in channels.iter_mut().enumerate() {
                let high = parse_hex_digit(bytes[index * 2])?;
                let low = parse_hex_digit(bytes[index * 2 + 1])?;
                *channel = high * 16 + low;
            }
            Some(Color {
                r: channels[0],
                g: channels[1],
                b: channels[2],
            })
        }
        _ => None,
    }
}

fn set_color(target: &mut Option<Color>, value: &str) -> bool {
    match parse_color(value) {
        Some(color) => {
            *target = Some(color);
            true
        }
        None => false,
    }
}

fn set_length(target: &mut Option<u32>, value: &str) -> bool {
    match parse_px(value) {
        Some(px) => {
            *target = Some(px);
            true
        }
        None => false,
    }
}

fn set_font_size(target: &mut Option<u32>, value: &str) -> bool {
    match parse_px(value) {
        Some(px) if (1..=MAX_FONT_SIZE).contains(&px) => {
            *target = Some(px);
            true
        }
        _ => false,
    }
}

/// 1-4 value shorthands only; two values are vertical/horizontal and three mirror CSS.
fn shorthand_lengths(value: &str) -> Option<[u32; 4]> {
    let mut values = [0u32; 4];
    let mut count = 0;
    for token in value.split_whitespace() {
        if count == values.len() {
            return None;
        }
        values[count] = parse_px(token)?;
        count += 1;
    }
    let top = values[0];
    match count {
        1 => Some([top, top, top, top]),
        2 => Some([top, values[1], top, values[1]]),
        3 => Some([top, values[1], values[2], values[1]]),
        4 => Some(values),
        _ => None,
    }
}

fn apply_declaration(declared: &mut Declared, name: &str, value: &str) -> bool {
    // `!important`, CSS-wide keywords and functions are outside the subset.
    if value.contains('!') || value.contains('(') {
        return false;
    }
    match name {
        "color" => set_color(&mut declared.color, value),
        "background-color" => set_color(&mut declared.background, value),
        "font-size" => set_font_size(&mut declared.font_size, value),
        "font-weight" => match value {
            "normal" => {
                declared.font_weight = Some(false);
                true
            }
            "bold" => {
                declared.font_weight = Some(true);
                true
            }
            _ => false,
        },
        "text-align" => match value {
            "left" => {
                declared.text_align = Some(TextAlign::Left);
                true
            }
            "center" => {
                declared.text_align = Some(TextAlign::Center);
                true
            }
            "right" => {
                declared.text_align = Some(TextAlign::Right);
                true
            }
            _ => false,
        },
        "display" => match value {
            "block" => {
                declared.display = Some(DisplayValue::Block);
                true
            }
            "inline" => {
                declared.display = Some(DisplayValue::Inline);
                true
            }
            "none" => {
                declared.display = Some(DisplayValue::None);
                true
            }
            _ => false,
        },
        "margin" => match shorthand_lengths(value) {
            Some([top, right, bottom, left]) => {
                declared.margin = [Some(top), Some(right), Some(bottom), Some(left)];
                true
            }
            None => false,
        },
        "margin-top" => set_length(&mut declared.margin[0], value),
        "margin-right" => set_length(&mut declared.margin[1], value),
        "margin-bottom" => set_length(&mut declared.margin[2], value),
        "margin-left" => set_length(&mut declared.margin[3], value),
        "padding" => match shorthand_lengths(value) {
            Some([top, right, bottom, left]) => {
                declared.padding = [Some(top), Some(right), Some(bottom), Some(left)];
                true
            }
            None => false,
        },
        "padding-top" => set_length(&mut declared.padding[0], value),
        "padding-right" => set_length(&mut declared.padding[1], value),
        "padding-bottom" => set_length(&mut declared.padding[2], value),
        "padding-left" => set_length(&mut declared.padding[3], value),
        "border" if value == "none" => {
            declared.border_width = Some(0);
            declared.border_style = Some(BorderStyle::None);
            true
        }
        "border" => match value.split_whitespace().count() {
            1 => match parse_px(value) {
                Some(width) => {
                    declared.border_width = Some(width);
                    declared.border_style = Some(BorderStyle::Solid);
                    true
                }
                None => false,
            },
            _ => false,
        },
        "border-width" => set_length(&mut declared.border_width, value),
        "border-style" => match value {
            "solid" => {
                declared.border_style = Some(BorderStyle::Solid);
                true
            }
            "none" => {
                declared.border_style = Some(BorderStyle::None);
                true
            }
            _ => false,
        },
        "border-color" => set_color(&mut declared.border_color, value),
        _ => false,
    }
}

fn parse_declarations(text: &str, unsupported: &mut usize) -> Declared {
    let text = strip_comments(text);
    let mut declared = Declared::default();
    for piece in text.split(';') {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        let Some(colon) = piece.find(':') else {
            *unsupported += 1;
            continue;
        };
        let name = piece[..colon].trim().to_ascii_lowercase();
        // CSS keywords and colour names are ASCII case-insensitive; hex digits are
        // lowercased harmlessly. Only names/values are normalized, never document text.
        let value = piece[colon + 1..].trim().to_ascii_lowercase();
        if name.is_empty() || value.is_empty() || !apply_declaration(&mut declared, &name, &value) {
            *unsupported += 1;
        }
    }
    declared
}

fn strip_comments(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        output.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => {
                rest = "";
                break;
            }
        }
    }
    output.push_str(rest);
    output
}

fn parse_selector(text: &str) -> Option<Selector> {
    if text.is_empty() || text.len() > MAX_CSS_SELECTOR_BYTES {
        return None;
    }
    let mut element = String::new();
    let mut class = None;
    let mut id = None;
    let mut chars = text.chars().peekable();
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            element.push(ch.to_ascii_lowercase());
            chars.next();
        } else {
            break;
        }
    }
    match chars.next() {
        None => {}
        Some(prefix) if prefix == '.' || prefix == '#' => {
            let mut name = String::new();
            for ch in chars.by_ref() {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    name.push(ch);
                } else {
                    return None;
                }
            }
            if name.is_empty() {
                return None;
            }
            if prefix == '.' {
                class = Some(name);
            } else {
                id = Some(name);
            }
        }
        // Combinators, pseudo-classes, attribute and universal selectors are unsupported.
        Some(_) => return None,
    }
    if element.is_empty() && class.is_none() && id.is_none() {
        return None;
    }
    let specificity = u32::from(!element.is_empty())
        + 10 * u32::from(class.is_some())
        + 100 * u32::from(id.is_some());
    Some(Selector {
        element: (!element.is_empty()).then_some(element),
        class,
        id,
        specificity,
    })
}

#[derive(Debug, Clone)]
struct Rule {
    selectors: Vec<Selector>,
    declared: Declared,
}

/// Parse a bounded stylesheet body. Unsupported selectors/declarations are skipped and counted.
fn parse_stylesheet(text: &str, unsupported: &mut usize, budget: usize) -> Vec<Rule> {
    let text = strip_comments(text);
    let mut rules = Vec::new();
    let mut position = 0;
    while let Some(open) = text[position..].find('{') {
        let open = position + open;
        let prelude = text[position..open].trim().to_string();
        let Some(close) = text[open + 1..].find('}') else {
            *unsupported += 1;
            return rules;
        };
        let close = open + 1 + close;
        let body = &text[open + 1..close];
        position = close + 1;
        if prelude.is_empty() || prelude.starts_with('@') || rules.len() >= budget {
            *unsupported += 1;
            continue;
        }
        let mut selectors = Vec::new();
        for candidate in prelude.split(',') {
            match parse_selector(candidate.trim()) {
                Some(selector) => selectors.push(selector),
                None => *unsupported += 1,
            }
        }
        if selectors.is_empty() {
            continue;
        }
        rules.push(Rule {
            selectors,
            declared: parse_declarations(body, unsupported),
        });
    }
    if !text[position..].trim().is_empty() {
        *unsupported += 1;
    }
    rules
}

fn default_display(tag: &str) -> DisplayValue {
    // The document and body are block boxes in the HTML UA stylesheet; treating body as
    // inline would silently discard its margin/padding and every block inset beneath it.
    if matches!(
        tag,
        "html" | "body" | "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
    ) {
        DisplayValue::Block
    } else {
        DisplayValue::Inline
    }
}

fn attribute<'a>(attrs: &'a [html5ever::Attribute], name: &str) -> &'a str {
    attrs
        .iter()
        .find(|attr| {
            let local: &str = attr.name.local.as_ref();
            local == name
        })
        .map_or("", |attr| attr.value.as_ref())
}

fn tag_defaults(tag: &str, base: &mut ComputedStyle) {
    base.display = default_display(tag);
    match tag {
        "h1" => base.font_size = 32,
        "h2" => base.font_size = 28,
        "h3" => base.font_size = 24,
        "h4" => base.font_size = 20,
        "h5" => base.font_size = 18,
        "h6" => base.font_size = 16,
        _ => {}
    }
    if matches!(
        tag,
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "b" | "strong"
    ) {
        base.bold = true;
    }
}

/// Concatenated text of every `<style>` element, in document order.
fn style_texts(nodes: &[Node]) -> Vec<String> {
    let mut texts = Vec::new();
    for node in nodes {
        let NodeKind::Element { name, .. } = &node.kind else {
            continue;
        };
        let tag: &str = name.local.as_ref();
        if name.ns != ns!(html) || tag != "style" {
            continue;
        }
        let mut text = String::new();
        let mut current = node.first;
        let mut steps = 0;
        while let Some(id) = current {
            steps += 1;
            if steps > MAX_NODES {
                break;
            }
            let Some(child) = nodes.get(id) else {
                break;
            };
            if let NodeKind::Text(part) = &child.kind
                && text.len() + part.len() <= crate::MAX_INPUT_BYTES
            {
                text.push_str(part);
            }
            current = child.next;
        }
        texts.push(text);
    }
    texts
}

/// Resolve computed styles for every node and report how much CSS was skipped.
pub(crate) fn compute_styles(nodes: &[Node]) -> (Vec<ComputedStyle>, usize) {
    let mut unsupported = 0;
    let mut rules = Vec::new();
    for text in style_texts(nodes) {
        if rules.len() >= MAX_CSS_RULES {
            // One count for each additional stylesheet skipped after the shared cap.
            unsupported += 1;
            continue;
        }
        let budget = MAX_CSS_RULES - rules.len();
        rules.extend(parse_stylesheet(&text, &mut unsupported, budget));
    }
    let initial = ComputedStyle::initial();
    let mut styles = vec![initial; nodes.len()];
    let mut stack = vec![(0usize, initial)];
    let mut steps = 0;
    while let Some((id, parent)) = stack.pop() {
        steps += 1;
        if steps > MAX_NODES * 2 {
            break;
        }
        let Some(node) = nodes.get(id) else {
            break;
        };
        let style = match &node.kind {
            NodeKind::Element { name, attrs, .. } if name.ns == ns!(html) => {
                let tag: &str = name.local.as_ref();
                let class = attribute(attrs, "class");
                let id_attribute = attribute(attrs, "id");
                let mut base = ComputedStyle::inherited(&parent);
                tag_defaults(tag, &mut base);
                let mut cascade = Cascade::default();
                for (order, rule) in rules.iter().enumerate() {
                    for selector in &rule.selectors {
                        if selector.matches(tag, class, id_attribute) {
                            cascade.merge(
                                &rule.declared,
                                selector.specificity,
                                u32::try_from(order).unwrap_or(u32::MAX),
                            );
                        }
                    }
                }
                if let Some(inline) = attrs.iter().find(|attr| {
                    let local: &str = attr.name.local.as_ref();
                    local == "style"
                }) {
                    let declared = parse_declarations(inline.value.as_ref(), &mut unsupported);
                    cascade.merge(&declared, INLINE_SPECIFICITY, u32::MAX);
                }
                cascade.resolve(base)
            }
            // Text/comment/doctype nodes inherit text properties but no box values;
            // copying the parent style here would leak margin/padding/background to them.
            _ => ComputedStyle::inherited(&parent),
        };
        styles[id] = style;
        let mut child = node.last;
        let mut siblings = 0;
        while let Some(child_id) = child {
            siblings += 1;
            if siblings > MAX_NODES {
                break;
            }
            let Some(child_node) = nodes.get(child_id) else {
                break;
            };
            if child_node.parent != Some(id) {
                break;
            }
            stack.push((child_id, style));
            child = child_node.previous;
        }
    }
    (styles, unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Value: protects=currentColor defaults for borders; fails_when=border_color resolves
    // from the inherited parent colour instead of this element's declared colour;
    // why_new=resolve used base.color before the cascade colour was applied;
    // seam=no border painting yet; no_claim=painted border fidelity.
    #[test]
    fn border_color_defaults_to_the_elements_own_color() {
        let declared = Declared {
            color: Some(Color { r: 255, g: 0, b: 0 }),
            border_width: Some(1),
            border_style: Some(BorderStyle::Solid),
            ..Declared::default()
        };
        let mut cascade = Cascade::default();
        cascade.merge(&declared, 1, 0);
        let resolved = cascade.resolve(ComputedStyle::initial());
        assert_eq!(resolved.border_color, Color { r: 255, g: 0, b: 0 });
    }
}

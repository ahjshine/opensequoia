#![forbid(unsafe_code)]

//! Bounded static HTML document increment. Full browser standards support remains unimplemented.

use std::fmt;

mod css;
mod dom;
mod layout;
mod tree_dump;

pub use dom::Document;

pub const MAX_INPUT_BYTES: usize = 262_144;
pub const MAX_NODES: usize = 32_768;
/// Inspection output bound; deep indentation can grow quadratically with tree depth.
pub const MAX_TREE_DUMP_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Namespace {
    Html,
    Svg,
    MathMl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentContext {
    pub namespace: Namespace,
    pub local_name: String,
}

/// HTML tree-construction configuration only. No setting executes scripts or requests.
/// The application uses `Document::parse`, whose scripting flag is always off.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParseOptions {
    pub scripting_enabled: bool,
    pub fragment: Option<FragmentContext>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InputTooLarge,
    NodeLimit,
    InvalidViewport,
    LayoutLimit,
    InvalidTree,
    TreeDumpLimit,
    InvalidContext,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnostic {
    HtmlParseError(String),
    UnsupportedElement(String),
    UnsupportedCss,
    UnsupportedFeature(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRun {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub font_size: u32,
    pub bold: bool,
    pub color: Color,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub color: Color,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayItem {
    /// Painted before later items; block backgrounds use this variant.
    Fill(FillRect),
    Text(TextRun),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayList {
    pub width: u32,
    pub height: u32,
    pub items: Vec<DisplayItem>,
}

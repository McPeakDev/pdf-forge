//! Layout config – the intermediate representation between layout computation
//! and PDF rendering. This is the "frozen" structure that encodes exactly what
//! goes on each page.

use serde::{Deserialize, Serialize};

/// A complete document layout ready for rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Document title embedded in the PDF metadata.
    #[serde(default = "LayoutConfig::default_title")]
    pub title: String,
    /// Width of each page in PDF points (1 pt = 1/72 inch).
    pub page_width_pt: f32,
    /// Height of each page in PDF points.
    pub page_height_pt: f32,
    /// Ordered list of pages.
    pub pages: Vec<PageLayout>,
}

/// One page of content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageLayout {
    pub page_index: usize,
    pub boxes: Vec<LayoutBox>,
}

/// A positioned rectangle with optional content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutBox {
    /// Position relative to page top-left, in points.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,

    /// Visual styling
    pub background_color: Option<[f32; 4]>,
    pub border: Option<BorderStyle>,

    /// Content (mutually exclusive in practice)
    pub text: Option<TextContent>,
    pub image: Option<ImageContent>,

    /// Children (nested boxes)
    pub children: Vec<LayoutBox>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorderStyle {
    pub width: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    /// Pre-wrapped lines of text.
    pub lines: Vec<TextLine>,
    pub font_family: String,
    pub font_size: f32,
    pub bold: bool,
    pub italic: bool,
    pub color: [f32; 4],
    pub line_height: f32,
    pub text_align: String,
    pub underline: bool,
    /// List bullet/number prefix (e.g. "• " or "1. ")
    pub list_marker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextLine {
    pub text: String,
    /// X offset within the layout box (for alignment)
    pub x_offset: f32,
    /// Y offset from the top of the text content area
    pub y_offset: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    pub src: String,
    pub width: f32,
    pub height: f32,
}

impl LayoutConfig {
    /// Create an A4 layout config.
    pub fn a4() -> Self {
        Self {
            title: Self::default_title(),
            // A4: 210mm × 297mm = 595.28 × 841.89 points
            page_width_pt: 595.28,
            page_height_pt: 841.89,
            pages: Vec::new(),
        }
    }

    fn default_title() -> String {
        "rpdf output".to_string()
    }

    /// Serialise to JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Deserialise from JSON.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }
}

impl LayoutBox {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            background_color: None,
            border: None,
            text: None,
            image: None,
            children: Vec::new(),
        }
    }
}

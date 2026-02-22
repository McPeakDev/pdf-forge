//! Pipeline – ties together parsing, styling, layout, pagination, and
//! rendering into a single function call.

use crate::dom::{body_children, parse_html};
use crate::fonts::FontManager;
use crate::layout::compute_layout;
use crate::layout_config::LayoutConfig;
use crate::pagination::{paginate, PAGE_MARGIN_PT};
use crate::render::render_pdf;
use crate::style::build_styled_tree;

/// Page orientation for the generated PDF.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PageOrientation {
    /// Portrait mode: height > width (default).
    #[default]
    Portrait,
    /// Landscape mode: width > height (short-edge binding).
    Landscape,
}

/// Configuration for the PDF generation pipeline.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Document title embedded in the PDF metadata (default: "rpdf output").
    pub title: String,
    /// Page width in points (default: A4 = 595.28).
    pub page_width: f32,
    /// Page height in points (default: A4 = 841.89).
    pub page_height: f32,
    /// Page margin in points (default: 40).
    pub page_margin: f32,
    /// Page orientation; swaps effective width/height when `Landscape`.
    pub orientation: PageOrientation,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            title: "rpdf output".to_string(),
            page_width: 595.28,
            page_height: 841.89,
            page_margin: PAGE_MARGIN_PT,
            orientation: PageOrientation::Portrait,
        }
    }
}

impl PipelineConfig {
    /// Effective page width after applying orientation.
    pub fn effective_width(&self) -> f32 {
        match self.orientation {
            PageOrientation::Portrait => self.page_width,
            PageOrientation::Landscape => self.page_height,
        }
    }

    /// Effective page height after applying orientation.
    pub fn effective_height(&self) -> f32 {
        match self.orientation {
            PageOrientation::Portrait => self.page_height,
            PageOrientation::Landscape => self.page_width,
        }
    }

    /// Create an A4 landscape config.
    pub fn a4_landscape() -> Self {
        Self {
            orientation: PageOrientation::Landscape,
            ..Self::default()
        }
    }
}

/// Full pipeline: HTML string → PDF bytes.
///
/// Returns `(pdf_bytes, layout_config_json)`.
pub fn generate_pdf(
    html: &str,
    config: &PipelineConfig,
) -> Result<(Vec<u8>, LayoutConfig), String> {
    // 1. Parse HTML
    let dom = parse_html(html);
    let dom_nodes = body_children(&dom);

    // 2. Build styled tree
    let styled = build_styled_tree(&dom_nodes, None);

    // 3. Compute layout
    let fonts = FontManager::default();
    let eff_w = config.effective_width();
    let eff_h = config.effective_height();
    let boxes = compute_layout(&styled, eff_w, config.page_margin, &fonts);

    // 4. Paginate
    let mut layout_config = paginate(&boxes, eff_w, eff_h, config.page_margin, &fonts);
    layout_config.title = config.title.clone();

    // 5. Render PDF
    let pdf_bytes = render_pdf(&layout_config)?;

    Ok((pdf_bytes, layout_config))
}

/// Convenience: generate PDF with default A4 config.
pub fn generate_pdf_from_html(html: &str) -> Result<Vec<u8>, String> {
    let (bytes, _) = generate_pdf(html, &PipelineConfig::default())?;
    Ok(bytes)
}

/// Generate only the layout config (no PDF rendering) – useful for testing.
pub fn compute_layout_config(html: &str, config: &PipelineConfig) -> LayoutConfig {
    let dom = parse_html(html);
    let dom_nodes = body_children(&dom);
    let styled = build_styled_tree(&dom_nodes, None);
    let fonts = FontManager::default();
    let eff_w = config.effective_width();
    let eff_h = config.effective_height();
    let boxes = compute_layout(&styled, eff_w, config.page_margin, &fonts);
    paginate(&boxes, eff_w, eff_h, config.page_margin, &fonts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_basic() {
        let html = "<h1>Hello</h1><p>World</p>";
        let (bytes, config) = generate_pdf(html, &PipelineConfig::default()).unwrap();
        assert!(!bytes.is_empty());
        assert!(!config.pages.is_empty());
        assert_eq!(&bytes[0..5], b"%PDF-");
    }
}

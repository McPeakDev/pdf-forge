//! Pagination – splits a flat list of positioned boxes into pages.
//!
//! Handles:
//! - A4 page boundaries
//! - Page-break-before / page-break-after hints
//! - Table row splitting across pages
//! - Orphan avoidance for text blocks

use crate::fonts::FontManager;
use crate::layout::{BoxContent, PositionedBox};
use crate::layout_config::*;
use crate::style;

/// Default page margins in points.
pub const PAGE_MARGIN_PT: f32 = 40.0;

/// Recursively expand any pure-container box whose height exceeds a single
/// page so its children can be split across pages individually.
fn flatten_for_pagination<'a>(
    boxes: &'a [PositionedBox],
    content_height: f32,
) -> Vec<&'a PositionedBox> {
    let mut result = Vec::new();
    for pbox in boxes {
        if pbox.height > content_height
            && matches!(pbox.content, BoxContent::None)
            && !pbox.children.is_empty()
        {
            result.extend(flatten_for_pagination(&pbox.children, content_height));
        } else {
            result.push(pbox);
        }
    }
    result
}

/// Convert positioned boxes into a paginated LayoutConfig.
pub fn paginate(
    boxes: &[PositionedBox],
    page_width: f32,
    page_height: f32,
    page_margin: f32,
    fonts: &FontManager,
) -> LayoutConfig {
    let mut config = LayoutConfig {
        title: "rpdf output".to_string(),
        page_width_pt: page_width,
        page_height_pt: page_height,
        pages: Vec::new(),
    };

    let content_height = page_height - 2.0 * page_margin;

    // Expand oversized wrapper divs so their children can paginate individually.
    let flat = flatten_for_pagination(boxes, content_height);

    let mut current_page = PageLayout {
        page_index: 0,
        boxes: Vec::new(),
    };

    // Document-space y at which the current page begins.  All PositionedBox.y
    // values are absolute document coordinates produced by the layout extractor,
    // so `pbox.y - page_start_doc_y` gives the y-on-page for any box.
    let mut page_start_doc_y = 0.0f32;

    for pbox in &flat {
        // Page break before
        if pbox.page_break_before && !current_page.boxes.is_empty() {
            config.pages.push(current_page);
            current_page = PageLayout {
                page_index: config.pages.len(),
                boxes: Vec::new(),
            };
            page_start_doc_y = pbox.y;
        }

        let y_on_page = (pbox.y - page_start_doc_y).max(0.0);
        let box_bottom = y_on_page + pbox.height;

        // Does this box overflow the current page?
        if box_bottom > content_height && !current_page.boxes.is_empty() {
            if is_table_like(pbox) && !pbox.page_break_inside_avoid {
                split_table_box(
                    pbox,
                    &mut config,
                    &mut current_page,
                    &mut page_start_doc_y,
                    content_height,
                    page_margin,
                    fonts,
                );
                continue;
            } else {
                config.pages.push(current_page);
                current_page = PageLayout {
                    page_index: config.pages.len(),
                    boxes: Vec::new(),
                };
                page_start_doc_y = pbox.y;
            }
        }

        let y_on_page = (pbox.y - page_start_doc_y).max(0.0);
        let layout_box = positioned_to_layout_box(pbox, page_margin, y_on_page, fonts);
        current_page.boxes.push(layout_box);

        // Page break after
        if pbox.page_break_after {
            config.pages.push(current_page);
            current_page = PageLayout {
                page_index: config.pages.len(),
                boxes: Vec::new(),
            };
            page_start_doc_y = pbox.y + pbox.height;
        }
    }

    if !current_page.boxes.is_empty() {
        config.pages.push(current_page);
    }
    if config.pages.is_empty() {
        config.pages.push(PageLayout {
            page_index: 0,
            boxes: Vec::new(),
        });
    }
    config
}

fn is_table_like(pbox: &PositionedBox) -> bool {
    pbox.style.display == style::Display::Grid && !pbox.children.is_empty()
}

fn split_table_box(
    pbox: &PositionedBox,
    config: &mut LayoutConfig,
    current_page: &mut PageLayout,
    page_start_doc_y: &mut f32,
    content_height: f32,
    page_margin: f32,
    fonts: &FontManager,
) {
    for child in &pbox.children {
        let y_on_page = (child.y - *page_start_doc_y).max(0.0);
        if y_on_page + child.height > content_height && !current_page.boxes.is_empty() {
            config.pages.push(std::mem::replace(
                current_page,
                PageLayout {
                    page_index: config.pages.len(),
                    boxes: Vec::new(),
                },
            ));
            *page_start_doc_y = child.y;
        }
        let y = (child.y - *page_start_doc_y).max(0.0);
        let row_box = positioned_to_layout_box(child, page_margin, y, fonts);
        current_page.boxes.push(row_box);
    }
}

/// Convert a PositionedBox to a LayoutBox with page-absolute coordinates.
/// `y_on_page` = `pbox.y - page_start_doc_y`; Taffy's layout already encodes
/// margin spacing into `pbox.y`, so we do not add margin_top separately.
fn positioned_to_layout_box(
    pbox: &PositionedBox,
    page_margin: f32,
    y_on_page: f32,
    fonts: &FontManager,
) -> LayoutBox {
    let abs_x = pbox.x;
    let abs_y = page_margin + y_on_page;
    build_layout_box(pbox, abs_x, abs_y, fonts)
}

/// Recursively build a LayoutBox tree where every box carries *page-absolute*
/// x/y coordinates (origin = top-left of the physical page).
///
/// For each child, its absolute y is derived by:
///   `child_abs_y = parent_abs_y + (child.y − parent.y)`
/// because PositionedBox.y values are accumulated document-space absolutes
/// (set by `extract` starting with `offset_y = 0`), so the difference gives
/// the child's position relative to its parent.
fn build_layout_box(
    pbox: &PositionedBox,
    abs_x: f32,
    abs_y: f32,
    fonts: &FontManager,
) -> LayoutBox {
    let mut lb = LayoutBox::new(abs_x, abs_y, pbox.width, pbox.height);

    // Background
    if !pbox.style.background_color.is_transparent() {
        let c = &pbox.style.background_color;
        lb.background_color = Some([c.r, c.g, c.b, c.a]);
    }

    // Border
    if pbox.style.border_width > 0.5 {
        let c = &pbox.style.border_color;
        lb.border = Some(BorderStyle {
            width: pbox.style.border_width,
            color: [c.r, c.g, c.b, c.a],
        });
    }

    // Content
    match &pbox.content {
        BoxContent::Text { lines, .. } => {
            let c = &pbox.style.color;
            let line_height = fonts.line_height_px(pbox.style.font_size, pbox.style.line_height);
            let text_lines: Vec<TextLine> = lines
                .iter()
                .enumerate()
                .map(|(i, line)| TextLine {
                    text: line.clone(),
                    x_offset: 0.0,
                    y_offset: i as f32 * line_height,
                })
                .collect();

            lb.text = Some(TextContent {
                lines: text_lines,
                font_family: pbox.style.font_family.clone(),
                font_size: pbox.style.font_size,
                bold: pbox.style.font_weight == style::FontWeight::Bold,
                italic: pbox.style.font_style == style::FontStyle::Italic,
                color: [c.r, c.g, c.b, c.a],
                line_height,
                text_align: match pbox.style.text_align {
                    style::TextAlign::Left => "left".to_string(),
                    style::TextAlign::Center => "center".to_string(),
                    style::TextAlign::Right => "right".to_string(),
                },
                underline: pbox.style.text_decoration == style::TextDecoration::Underline,
                list_marker: None,
            });
        }
        BoxContent::Image { src } => {
            lb.image = Some(ImageContent {
                src: src.clone(),
                width: pbox.width,
                height: pbox.height,
            });
        }
        BoxContent::ListItem { marker } => {
            let c = &pbox.style.color;
            let line_height = fonts.line_height_px(pbox.style.font_size, pbox.style.line_height);
            // `lines` is empty – the bullet / number is rendered via
            // `list_marker` (drawn 16 pt to the left of the li box), while
            // the li's actual text content comes from its child boxes.
            lb.text = Some(TextContent {
                lines: vec![],
                font_family: pbox.style.font_family.clone(),
                font_size: pbox.style.font_size,
                bold: pbox.style.font_weight == style::FontWeight::Bold,
                italic: false,
                color: [c.r, c.g, c.b, c.a],
                line_height,
                text_align: "left".to_string(),
                underline: false,
                list_marker: Some(marker.clone()),
            });
        }
        BoxContent::None => {}
    }

    // Recurse into children, propagating absolute coordinates.
    // Each child's PositionedBox.y is a document-space absolute, so
    // (child.y − pbox.y) gives the child's offset within the parent.
    for child in &pbox.children {
        let child_abs_x = child.x; // already page-absolute (extract accumulated page_margin)
        let child_abs_y = abs_y + (child.y - pbox.y);
        let child_box = build_layout_box(child, child_abs_x, child_abs_y, fonts);
        lb.children.push(child_box);
    }

    lb
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::parse_html;
    use crate::layout::compute_layout;
    use crate::style::build_styled_tree;

    #[test]
    fn single_page() {
        let html = "<p>Short text</p>";
        let dom = parse_html(html);
        let styled = build_styled_tree(&dom, None);
        let fonts = FontManager::default();
        let boxes = compute_layout(&styled, 595.0, PAGE_MARGIN_PT, &fonts);
        let config = paginate(&boxes, 595.0, 842.0, PAGE_MARGIN_PT, &fonts);
        assert_eq!(config.pages.len(), 1);
    }

    #[test]
    fn multiple_pages() {
        // Generate enough content to fill multiple pages
        let mut html = String::new();
        for i in 0..60 {
            html.push_str(&format!("<p>Paragraph {} with some text</p>", i));
        }
        let dom = parse_html(&html);
        let styled = build_styled_tree(&dom, None);
        let fonts = FontManager::default();
        let boxes = compute_layout(&styled, 595.0, PAGE_MARGIN_PT, &fonts);
        let config = paginate(&boxes, 595.0, 842.0, PAGE_MARGIN_PT, &fonts);
        assert!(
            config.pages.len() > 1,
            "Expected multiple pages, got {}",
            config.pages.len()
        );
    }
}

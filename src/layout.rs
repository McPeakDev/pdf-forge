//! Layout engine – uses Taffy to compute flexbox / grid layout from a styled
//! DOM tree, then converts the result into a flat list of positioned boxes.

use std::collections::HashMap;
use taffy::prelude::*;

use crate::fonts::{wrap_text, FontManager};
use crate::style::{self, ComputedStyle, FontStyle as CssFontStyle, FontWeight, StyledNode};

// ---------------------------------------------------------------------------
// Intermediate layout tree (pre-pagination)
// ---------------------------------------------------------------------------

/// A positioned box in document coordinates (before page splitting).
#[derive(Debug, Clone)]
pub struct PositionedBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub style: ComputedStyle,
    pub content: BoxContent,
    pub children: Vec<PositionedBox>,
    pub page_break_before: bool,
    pub page_break_after: bool,
    pub page_break_inside_avoid: bool,
}

#[derive(Debug, Clone)]
pub enum BoxContent {
    None,
    Text {
        text: String,
        lines: Vec<String>,
    },
    Image {
        src: String,
    },
    /// List item marker
    ListItem {
        marker: String,
    },
}

// ---------------------------------------------------------------------------
// Build Taffy tree from styled nodes
// ---------------------------------------------------------------------------

struct LayoutBuilder<'a> {
    taffy: TaffyTree<()>,
    fonts: &'a FontManager,
    node_styles: HashMap<NodeId, ComputedStyle>,
    node_content: HashMap<NodeId, BoxContent>,
    available_width: f32,
}

impl<'a> LayoutBuilder<'a> {
    fn new(fonts: &'a FontManager, available_width: f32) -> Self {
        Self {
            taffy: TaffyTree::new(),
            fonts,
            node_styles: HashMap::new(),
            node_content: HashMap::new(),
            available_width,
        }
    }

    /// Collect all text content from an inline subtree (spans, text nodes).
    fn collect_inline_text(node: &StyledNode) -> String {
        match node {
            StyledNode::Text { text, .. } => text.clone(),
            StyledNode::Element { children, .. } => children
                .iter()
                .map(Self::collect_inline_text)
                .collect::<Vec<_>>()
                .join(""),
        }
    }

    /// Return true when every child is a text node or a display:inline element
    /// (no block-level children).
    fn all_inline(children: &[StyledNode]) -> bool {
        children.iter().all(|c| match c {
            StyledNode::Text { .. } => true,
            StyledNode::Element {
                style,
                children: gc,
                ..
            } => {
                matches!(
                    style.display,
                    style::Display::Inline | style::Display::InlineBlock
                ) && Self::all_inline(gc)
            }
        })
    }

    fn build_node(&mut self, styled: &StyledNode, parent_width: f32) -> NodeId {
        match styled {
            StyledNode::Text { text, style } => self.build_text_node(text, style, parent_width),
            StyledNode::Element {
                tag,
                style,
                children,
                attrs,
            } => self.build_element_node(tag, style, children, attrs, parent_width),
        }
    }

    /// Like build_text_node but also applies paragraph-level margin/padding
    /// from the enclosing block style so that headings keep their spacing.
    fn build_text_node_with_para_style(
        &mut self,
        text: &str,
        block_style: &ComputedStyle,
        parent_width: f32,
    ) -> NodeId {
        let node = self.build_text_node(text, block_style, parent_width);
        // Replace the Taffy style to include margin/padding from the block.
        let current = self.taffy.style(node).unwrap().clone();
        let updated = Style {
            margin: Rect {
                top: LengthPercentageAuto::Length(block_style.margin_top),
                right: LengthPercentageAuto::Length(block_style.margin_right),
                bottom: LengthPercentageAuto::Length(block_style.margin_bottom),
                left: LengthPercentageAuto::Length(block_style.margin_left),
            },
            padding: Rect {
                top: LengthPercentage::Length(block_style.padding_top),
                right: LengthPercentage::Length(block_style.padding_right),
                bottom: LengthPercentage::Length(block_style.padding_bottom),
                left: LengthPercentage::Length(block_style.padding_left),
            },
            ..current
        };
        self.taffy.set_style(node, updated).unwrap();
        node
    }

    fn build_text_node(&mut self, text: &str, style: &ComputedStyle, parent_width: f32) -> NodeId {
        let bold = style.font_weight == FontWeight::Bold;
        let italic = style.font_style == CssFontStyle::Italic;
        let family = &style.font_family;
        let font_size = style.font_size;
        let line_height_px = self.fonts.line_height_px(font_size, style.line_height);

        // Word-wrap the text
        let max_w = if parent_width > 0.0 {
            parent_width
        } else {
            self.available_width
        };
        let lines = wrap_text(
            text.trim(),
            font_size,
            bold,
            italic,
            family,
            max_w,
            self.fonts,
        );

        let text_width = lines
            .iter()
            .map(|l| {
                self.fonts
                    .measure_text_width(l, font_size, bold, italic, family)
            })
            .fold(0.0f32, f32::max);
        let text_height = lines.len() as f32 * line_height_px;

        let taffy_style = Style {
            size: Size {
                width: Dimension::Length(text_width),
                height: Dimension::Length(text_height),
            },
            ..Default::default()
        };

        let node = self.taffy.new_leaf(taffy_style).unwrap();
        self.node_styles.insert(node, style.clone());
        self.node_content.insert(
            node,
            BoxContent::Text {
                text: text.trim().to_string(),
                lines,
            },
        );
        node
    }

    fn build_element_node(
        &mut self,
        tag: &crate::dom::Tag,
        style: &ComputedStyle,
        children: &[StyledNode],
        attrs: &HashMap<String, String>,
        parent_width: f32,
    ) -> NodeId {
        // Paragraph-like block elements whose children are all inline get their
        // text merged into a single wrapped text node so spans flow correctly.
        let is_paragraph = matches!(
            tag,
            crate::dom::Tag::P | crate::dom::Tag::H1 | crate::dom::Tag::H2 | crate::dom::Tag::H3
        );
        if is_paragraph && !children.is_empty() && Self::all_inline(children) {
            let raw: String = children.iter().map(Self::collect_inline_text).collect();
            // Normalise runs of whitespace/newlines to single spaces.
            let combined: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
            if !combined.is_empty() {
                return self.build_text_node_with_para_style(&combined, style, parent_width);
            }
        }

        // Compute the width available for children
        let my_width = match style.width {
            crate::style::Dimension::Px(w) => w,
            crate::style::Dimension::Percent(p) => parent_width * p / 100.0,
            crate::style::Dimension::Auto => parent_width,
        };
        let inner_width = my_width - style.padding_left - style.padding_right;

        // Estimate per-child width for flex-row containers and table rows so
        // that text is word-wrapped to the right column width at build time.
        let is_flex_row = style.display == style::Display::Flex
            && style.flex_direction == style::FlexDirection::Row;
        let is_table_row = *tag == crate::dom::Tag::Tr;

        let elem_child_count = children
            .iter()
            .filter(|c| matches!(c, StyledNode::Element { .. }))
            .count()
            .max(1);

        let child_build_width = if is_flex_row || is_table_row {
            let gap_total = style.gap * (elem_child_count.saturating_sub(1)) as f32;
            ((inner_width - gap_total) / elem_child_count as f32).max(1.0)
        } else {
            inner_width
        };

        // Build child nodes
        let mut child_nodes = Vec::new();
        let mut list_counter = 0u32;

        for child in children {
            // For list items, compute and record the marker string so it can
            // be rendered as a bullet / number in the left gutter.
            let li_marker: Option<String> =
                if let StyledNode::Element { tag: child_tag, .. } = child {
                    if *child_tag == crate::dom::Tag::Li {
                        list_counter += 1;
                        Some(if *tag == crate::dom::Tag::Ol {
                            format!("{}. ", list_counter)
                        } else {
                            "\u{2022} ".to_string()
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };

            let child_id = self.build_node(child, child_build_width);

            // Attach the marker to the taffy node so pagination can render it.
            if let Some(marker) = li_marker {
                self.node_content
                    .insert(child_id, BoxContent::ListItem { marker });
            }

            child_nodes.push(child_id);
        }

        // For <img> elements, resolve Auto width/height to concrete pixel dimensions
        // using the image's intrinsic size decoded from the base64 data URI.
        // Without this, a Taffy flex container with no children and Auto dimensions
        // computes to 0×0, making the image invisible in the rendered PDF.
        let style_override: Option<crate::style::ComputedStyle> = if *tag == crate::dom::Tag::Img
            && (matches!(style.width, crate::style::Dimension::Auto)
                || matches!(style.height, crate::style::Dimension::Auto))
        {
            let src = attrs.get("src").map(|s| s.as_str()).unwrap_or("");
            resolve_img_auto_dimensions(src, style, parent_width)
        } else {
            None
        };

        let effective_style = style_override.as_ref().unwrap_or(style);
        let taffy_style = self.computed_to_taffy(effective_style, tag);
        let node = self
            .taffy
            .new_with_children(taffy_style, &child_nodes)
            .unwrap();
        self.node_styles.insert(node, effective_style.clone());

        // Handle images
        if *tag == crate::dom::Tag::Img {
            let src = attrs.get("src").cloned().unwrap_or_default();
            self.node_content.insert(node, BoxContent::Image { src });
        }

        node
    }

    fn computed_to_taffy(&self, s: &ComputedStyle, tag: &crate::dom::Tag) -> Style {
        let mut ts = Style::default();

        // -----------------------------------------------------------------
        // HTML table model: always use flex regardless of computed display.
        // -----------------------------------------------------------------
        match tag {
            crate::dom::Tag::Table => {
                ts.display = taffy::Display::Flex;
                ts.flex_direction = taffy::FlexDirection::Column;
                ts.size.width = self.dim_to_taffy(s.width);
                ts.size.height = self.dim_to_taffy(s.height);
                ts.min_size.width = taffy::Dimension::Length(0.0);
                ts.padding = Rect {
                    top: LengthPercentage::Length(s.padding_top),
                    right: LengthPercentage::Length(s.padding_right),
                    bottom: LengthPercentage::Length(s.padding_bottom),
                    left: LengthPercentage::Length(s.padding_left),
                };
                ts.margin = Rect {
                    top: LengthPercentageAuto::Length(s.margin_top),
                    right: LengthPercentageAuto::Length(s.margin_right),
                    bottom: LengthPercentageAuto::Length(s.margin_bottom),
                    left: LengthPercentageAuto::Length(s.margin_left),
                };
                return ts;
            }
            crate::dom::Tag::Tr => {
                ts.display = taffy::Display::Flex;
                ts.flex_direction = taffy::FlexDirection::Row;
                ts.align_items = Some(taffy::AlignItems::Stretch);
                ts.size.width = taffy::Dimension::Percent(1.0);
                ts.min_size.width = taffy::Dimension::Length(0.0);
                ts.margin = Rect {
                    top: LengthPercentageAuto::Length(s.margin_top),
                    right: LengthPercentageAuto::Length(s.margin_right),
                    bottom: LengthPercentageAuto::Length(s.margin_bottom),
                    left: LengthPercentageAuto::Length(s.margin_left),
                };
                return ts;
            }
            crate::dom::Tag::Td | crate::dom::Tag::Th => {
                ts.display = taffy::Display::Flex;
                ts.flex_direction = taffy::FlexDirection::Column;
                ts.flex_grow = 1.0;
                ts.flex_shrink = 1.0;
                ts.flex_basis = taffy::Dimension::Length(0.0); // equal columns
                ts.min_size.width = taffy::Dimension::Length(0.0);
                ts.padding = Rect {
                    top: LengthPercentage::Length(s.padding_top),
                    right: LengthPercentage::Length(s.padding_right),
                    bottom: LengthPercentage::Length(s.padding_bottom),
                    left: LengthPercentage::Length(s.padding_left),
                };
                ts.border = Rect {
                    top: LengthPercentage::Length(s.border_width),
                    right: LengthPercentage::Length(s.border_width),
                    bottom: LengthPercentage::Length(s.border_width),
                    left: LengthPercentage::Length(s.border_width),
                };
                return ts;
            }
            _ => {}
        }

        // Display / layout mode
        match s.display {
            style::Display::Flex => {
                ts.display = taffy::Display::Flex;
                ts.flex_direction = match s.flex_direction {
                    style::FlexDirection::Row => taffy::FlexDirection::Row,
                    style::FlexDirection::Column => taffy::FlexDirection::Column,
                };
                ts.flex_wrap = match s.flex_wrap {
                    style::FlexWrap::NoWrap => taffy::FlexWrap::NoWrap,
                    style::FlexWrap::Wrap => taffy::FlexWrap::Wrap,
                };
                ts.justify_content = Some(match s.justify_content {
                    style::JustifyContent::Start => taffy::JustifyContent::Start,
                    style::JustifyContent::End => taffy::JustifyContent::End,
                    style::JustifyContent::Center => taffy::JustifyContent::Center,
                    style::JustifyContent::SpaceBetween => taffy::JustifyContent::SpaceBetween,
                    style::JustifyContent::SpaceAround => taffy::JustifyContent::SpaceAround,
                    style::JustifyContent::SpaceEvenly => taffy::JustifyContent::SpaceEvenly,
                });
                ts.align_items = Some(match s.align_items {
                    style::AlignItems::Start => taffy::AlignItems::Start,
                    style::AlignItems::End => taffy::AlignItems::End,
                    style::AlignItems::Center => taffy::AlignItems::Center,
                    style::AlignItems::Stretch => taffy::AlignItems::Stretch,
                });
            }
            style::Display::Grid => {
                ts.display = taffy::Display::Grid;
                let cols = if !s.grid_template_columns.is_empty() {
                    s.grid_template_columns.len()
                } else {
                    1
                };
                ts.grid_template_columns = vec![taffy::TrackSizingFunction::from_flex(1.0); cols];
            }
            style::Display::Block
            | style::Display::ListItem
            | style::Display::TableRow
            | style::Display::TableCell
            | style::Display::InlineBlock => {
                // Use flex column for block-level elements (vertical stacking)
                ts.display = taffy::Display::Flex;
                ts.flex_direction = taffy::FlexDirection::Column;
            }
            style::Display::Inline => {
                ts.display = taffy::Display::Flex;
                ts.flex_direction = taffy::FlexDirection::Row;
                ts.flex_wrap = taffy::FlexWrap::Wrap;
            }
            style::Display::None => {
                ts.display = taffy::Display::None;
            }
        }

        // Sizing
        ts.size = Size {
            width: self.dim_to_taffy(s.width),
            height: self.dim_to_taffy(s.height),
        };
        // Allow flex/shrink items to compress below their natural content size
        ts.min_size = Size {
            width: if s.flex_shrink > 0.0 || s.flex_grow > 0.0 {
                taffy::Dimension::Length(0.0)
            } else {
                self.dim_to_taffy(s.min_width)
            },
            height: taffy::Dimension::Auto,
        };
        ts.max_size = Size {
            width: self.dim_to_taffy(s.max_width),
            height: taffy::Dimension::Auto,
        };

        // Flex properties
        ts.flex_grow = s.flex_grow;
        ts.flex_shrink = s.flex_shrink;

        // Margin
        ts.margin = Rect {
            top: LengthPercentageAuto::Length(s.margin_top),
            right: LengthPercentageAuto::Length(s.margin_right),
            bottom: LengthPercentageAuto::Length(s.margin_bottom),
            left: LengthPercentageAuto::Length(s.margin_left),
        };

        // Padding
        ts.padding = Rect {
            top: LengthPercentage::Length(s.padding_top),
            right: LengthPercentage::Length(s.padding_right),
            bottom: LengthPercentage::Length(s.padding_bottom),
            left: LengthPercentage::Length(s.padding_left),
        };

        // Border
        ts.border = Rect {
            top: LengthPercentage::Length(s.border_width),
            right: LengthPercentage::Length(s.border_width),
            bottom: LengthPercentage::Length(s.border_width),
            left: LengthPercentage::Length(s.border_width),
        };

        // Gap
        ts.gap = Size {
            width: LengthPercentage::Length(s.gap),
            height: LengthPercentage::Length(s.gap),
        };

        ts
    }

    fn dim_to_taffy(&self, d: crate::style::Dimension) -> taffy::Dimension {
        match d {
            crate::style::Dimension::Auto => taffy::Dimension::Auto,
            crate::style::Dimension::Px(v) => taffy::Dimension::Length(v),
            crate::style::Dimension::Percent(v) => taffy::Dimension::Percent(v / 100.0),
        }
    }

    /// Extract positioned boxes after layout computation.
    fn extract(&self, node: NodeId, offset_x: f32, offset_y: f32) -> PositionedBox {
        let layout = self.taffy.layout(node).unwrap();
        let style = self.node_styles.get(&node).cloned().unwrap_or_default();
        let content = self
            .node_content
            .get(&node)
            .cloned()
            .unwrap_or(BoxContent::None);

        let x = offset_x + layout.location.x;
        let y = offset_y + layout.location.y;

        let children: Vec<PositionedBox> = self
            .taffy
            .children(node)
            .unwrap_or_default()
            .iter()
            .map(|&child| self.extract(child, x, y))
            .collect();

        PositionedBox {
            x,
            y,
            width: layout.size.width,
            height: layout.size.height,
            page_break_before: style.page_break_before,
            page_break_after: style.page_break_after,
            page_break_inside_avoid: style.page_break_inside_avoid,
            style,
            content,
            children,
        }
    }
}

// ---------------------------------------------------------------------------
// Image intrinsic-size helper
// ---------------------------------------------------------------------------

/// Attempt to decode a base64 data-URI image and return a cloned
/// [`ComputedStyle`] with any `Auto` width/height replaced by concrete pixel
/// values derived from the image's intrinsic dimensions.
///
/// Returns `None` when the src is not a parseable base64 data URI, when image
/// decoding fails, or when both dimensions are already specified (no fix needed).
fn resolve_img_auto_dimensions(
    src: &str,
    style: &crate::style::ComputedStyle,
    parent_width: f32,
) -> Option<crate::style::ComputedStyle> {
    use base64::{engine::general_purpose::STANDARD as BASE64_STD, Engine as _};

    if !src.starts_with("data:") || !src.contains(";base64,") {
        return None;
    }
    let comma = src.find(',')?;
    let b64 = src[comma + 1..].trim();
    let bytes = BASE64_STD.decode(b64).ok()?;
    let img = ::image::load_from_memory(&bytes).ok()?;
    let (px_w, px_h) = (img.width() as f32, img.height() as f32);
    if px_w == 0.0 || px_h == 0.0 {
        return None;
    }
    let aspect = px_w / px_h;

    let known_w: Option<f32> = match style.width {
        crate::style::Dimension::Px(v) => Some(v),
        crate::style::Dimension::Percent(p) => Some(parent_width * p / 100.0),
        crate::style::Dimension::Auto => None,
    };
    let known_h: Option<f32> = match style.height {
        crate::style::Dimension::Px(v) => Some(v),
        _ => None,
    };

    let mut s = style.clone();
    match (known_w, known_h) {
        // Width known → derive height from aspect ratio.
        (Some(w), None) => s.height = crate::style::Dimension::Px((w / aspect).max(1.0)),
        // Height known → derive width from aspect ratio.
        (None, Some(h)) => s.width = crate::style::Dimension::Px((h * aspect).max(1.0)),
        // Both Auto → use intrinsic pixel dimensions at 1 px = 1 pt.
        (None, None) => {
            s.width = crate::style::Dimension::Px(px_w);
            s.height = crate::style::Dimension::Px(px_h);
        }
        // Both already resolved — nothing to fix.
        (Some(_), Some(_)) => return None,
    }
    Some(s)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute layout for a styled tree, returning a list of top-level positioned
/// boxes in document coordinates.
pub fn compute_layout(
    styled_nodes: &[StyledNode],
    page_width: f32,
    page_margin: f32,
    fonts: &FontManager,
) -> Vec<PositionedBox> {
    let content_width = page_width - 2.0 * page_margin;
    let mut builder = LayoutBuilder::new(fonts, content_width);

    // Wrap all nodes in a root flex-column container
    let mut child_ids = Vec::new();
    for node in styled_nodes {
        let id = builder.build_node(node, content_width);
        child_ids.push(id);
    }

    let root_style = Style {
        display: taffy::Display::Flex,
        flex_direction: taffy::FlexDirection::Column,
        size: Size {
            width: taffy::Dimension::Length(content_width),
            height: taffy::Dimension::Auto,
        },
        ..Default::default()
    };

    let root = builder
        .taffy
        .new_with_children(root_style, &child_ids)
        .unwrap();

    builder
        .taffy
        .compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(content_width),
                height: AvailableSpace::MaxContent,
            },
        )
        .unwrap();

    // Extract positioned boxes
    let root_box = builder.extract(root, page_margin, 0.0);
    root_box.children
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::parse_html;
    use crate::style::build_styled_tree;

    #[test]
    fn layout_simple_paragraph() {
        let html = "<p>Hello world</p>";
        let dom = parse_html(html);
        let styled = build_styled_tree(&dom, None);
        let fonts = FontManager::default();
        let boxes = compute_layout(&styled, 595.0, 40.0, &fonts);
        assert!(!boxes.is_empty(), "Should produce at least one box");
        let first = &boxes[0];
        assert!(first.width > 0.0, "Box should have width");
        assert!(first.height > 0.0, "Box should have height");
    }

    #[test]
    fn layout_flex_row() {
        let html =
            r#"<div class="flex"><div class="flex-1">A</div><div class="flex-1">B</div></div>"#;
        let dom = parse_html(html);
        let styled = build_styled_tree(&dom, None);
        let fonts = FontManager::default();
        let boxes = compute_layout(&styled, 595.0, 40.0, &fonts);
        assert!(!boxes.is_empty());
    }
}

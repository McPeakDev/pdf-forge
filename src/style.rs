//! Style resolver – maps CSS inline styles and Tailwind-like utility classes
//! to a flat [`ComputedStyle`] struct consumed by the layout engine.

use crate::dom::{DomNode, ElementNode, Tag};

/// Fully resolved style for a single element.
#[derive(Debug, Clone)]
pub struct ComputedStyle {
    // Display / layout
    pub display: Display,
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub gap: f32,

    // Grid
    pub grid_template_columns: Vec<GridTrack>,
    pub grid_template_rows: Vec<GridTrack>,

    // Sizing
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub max_width: Dimension,

    // Spacing (px)
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,

    // Border
    pub border_width: f32,
    pub border_color: Color,

    // Typography
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub font_family: String,
    pub color: Color,
    pub text_align: TextAlign,
    pub line_height: f32,
    pub text_decoration: TextDecoration,
    pub font_style: FontStyle,

    // Background
    pub background_color: Color,

    // Page break
    pub page_break_before: bool,
    pub page_break_after: bool,
    pub page_break_inside_avoid: bool,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            gap: 0.0,
            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: Dimension::Auto,
            max_width: Dimension::Auto,
            margin_top: 0.0,
            margin_right: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            border_width: 0.0,
            border_color: Color::BLACK,
            font_size: 16.0,
            font_weight: FontWeight::Normal,
            font_family: "Helvetica".to_string(),
            color: Color::BLACK,
            text_align: TextAlign::Left,
            line_height: 1.4,
            text_decoration: TextDecoration::None,
            font_style: FontStyle::Normal,
            background_color: Color::TRANSPARENT,
            page_break_before: false,
            page_break_after: false,
            page_break_inside_avoid: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Supporting enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Block,
    Flex,
    Grid,
    Inline,
    InlineBlock,
    ListItem,
    TableRow,
    TableCell,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    Start,
    End,
    Center,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Normal,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDecoration {
    None,
    Underline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dimension {
    Auto,
    Px(f32),
    Percent(f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridTrack {
    Px(f32),
    Fr(f32),
    Auto,
}

/// RGBA colour (0.0 – 1.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    pub fn is_transparent(&self) -> bool {
        self.a < 0.001
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            Some(Self { r, g, b, a: 1.0 })
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()? as f32 / 255.0;
            Some(Self { r, g, b, a: 1.0 })
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Style resolution
// ---------------------------------------------------------------------------

/// Resolve the style for an element, inheriting text properties from its parent.
pub fn resolve_style(element: &ElementNode, parent: Option<&ComputedStyle>) -> ComputedStyle {
    let mut style = base_style_for_tag(&element.tag);

    // Inherit text properties from parent
    if let Some(p) = parent {
        style.font_size = p.font_size;
        style.font_weight = p.font_weight;
        style.font_family = p.font_family.clone();
        style.color = p.color;
        style.text_align = p.text_align;
        style.line_height = p.line_height;
        style.font_style = p.font_style;
    }

    // Apply Tailwind classes
    for class in element.classes() {
        apply_tailwind_class(&mut style, class);
    }

    // Apply inline style attribute
    if let Some(inline) = element.inline_style() {
        apply_inline_style(&mut style, inline);
    }

    style
}

/// Default styles based on tag semantics.
fn base_style_for_tag(tag: &Tag) -> ComputedStyle {
    let mut s = ComputedStyle::default();
    match tag {
        Tag::H1 => {
            s.font_size = 32.0;
            s.font_weight = FontWeight::Bold;
            s.margin_top = 16.0;
            s.margin_bottom = 12.0;
        }
        Tag::H2 => {
            s.font_size = 24.0;
            s.font_weight = FontWeight::Bold;
            s.margin_top = 14.0;
            s.margin_bottom = 10.0;
        }
        Tag::H3 => {
            s.font_size = 20.0;
            s.font_weight = FontWeight::Bold;
            s.margin_top = 12.0;
            s.margin_bottom = 8.0;
        }
        Tag::P => {
            s.margin_top = 0.0;
            s.margin_bottom = 10.0;
        }
        Tag::Ul | Tag::Ol => {
            s.margin_top = 0.0;
            s.margin_bottom = 10.0;
            s.padding_left = 24.0;
        }
        Tag::Li => {
            s.display = Display::ListItem;
            s.margin_bottom = 4.0;
        }
        Tag::Table => {
            s.display = Display::Grid;
            s.border_width = 1.0;
            s.page_break_inside_avoid = false; // tables can split
        }
        Tag::Tr => {
            s.display = Display::TableRow;
        }
        Tag::Td | Tag::Th => {
            s.display = Display::TableCell;
            s.padding_top = 4.0;
            s.padding_right = 8.0;
            s.padding_bottom = 4.0;
            s.padding_left = 8.0;
            s.border_width = 1.0;
            if *tag == Tag::Th {
                s.font_weight = FontWeight::Bold;
                s.background_color = Color {
                    r: 0.93,
                    g: 0.93,
                    b: 0.93,
                    a: 1.0,
                };
            }
        }
        Tag::Span => {
            s.display = Display::Inline;
        }
        Tag::Img => {
            s.display = Display::InlineBlock;
        }
        Tag::Div | Tag::Body | Tag::Html | Tag::Head => {}
        Tag::Unknown(_) => {
            // Silently skip unrecognised elements – treat as display:none.
            s.display = Display::None;
        }
    }
    s
}

/// Apply a single Tailwind utility class.
fn apply_tailwind_class(s: &mut ComputedStyle, class: &str) {
    match class {
        // Display
        "flex" => s.display = Display::Flex,
        "grid" => s.display = Display::Grid,
        "block" => s.display = Display::Block,
        "inline" => s.display = Display::Inline,
        "inline-block" => s.display = Display::InlineBlock,
        "hidden" => s.display = Display::None,

        // Flex direction
        "flex-row" => s.flex_direction = FlexDirection::Row,
        "flex-col" => s.flex_direction = FlexDirection::Column,

        // Flex wrap
        "flex-wrap" => s.flex_wrap = FlexWrap::Wrap,
        "flex-nowrap" => s.flex_wrap = FlexWrap::NoWrap,

        // Flex grow/shrink
        "flex-grow" | "grow" => s.flex_grow = 1.0,
        "flex-shrink" | "shrink" => s.flex_shrink = 1.0,
        "flex-1" => {
            s.flex_grow = 1.0;
            s.flex_shrink = 1.0;
        }

        // Justify content
        "justify-start" => s.justify_content = JustifyContent::Start,
        "justify-end" => s.justify_content = JustifyContent::End,
        "justify-center" => s.justify_content = JustifyContent::Center,
        "justify-between" => s.justify_content = JustifyContent::SpaceBetween,
        "justify-around" => s.justify_content = JustifyContent::SpaceAround,
        "justify-evenly" => s.justify_content = JustifyContent::SpaceEvenly,

        // Align items
        "items-start" => s.align_items = AlignItems::Start,
        "items-end" => s.align_items = AlignItems::End,
        "items-center" => s.align_items = AlignItems::Center,
        "items-stretch" => s.align_items = AlignItems::Stretch,

        // Font weight
        "font-bold" => s.font_weight = FontWeight::Bold,
        "font-normal" => s.font_weight = FontWeight::Normal,

        // Font style
        "italic" => s.font_style = FontStyle::Italic,
        "not-italic" => s.font_style = FontStyle::Normal,

        // Text decoration
        "underline" => s.text_decoration = TextDecoration::Underline,
        "no-underline" => s.text_decoration = TextDecoration::None,

        // Text alignment
        "text-left" => s.text_align = TextAlign::Left,
        "text-center" => s.text_align = TextAlign::Center,
        "text-right" => s.text_align = TextAlign::Right,

        // Font sizes
        "text-xs" => s.font_size = 12.0,
        "text-sm" => s.font_size = 14.0,
        "text-base" => s.font_size = 16.0,
        "text-lg" => s.font_size = 18.0,
        "text-xl" => s.font_size = 20.0,
        "text-2xl" => s.font_size = 24.0,
        "text-3xl" => s.font_size = 30.0,
        "text-4xl" => s.font_size = 36.0,

        // Width
        "w-full" => s.width = Dimension::Percent(100.0),
        "w-auto" => s.width = Dimension::Auto,
        "w-1/2" => s.width = Dimension::Percent(50.0),
        "w-1/3" => s.width = Dimension::Percent(33.333),
        "w-2/3" => s.width = Dimension::Percent(66.666),
        "w-1/4" => s.width = Dimension::Percent(25.0),
        "w-3/4" => s.width = Dimension::Percent(75.0),

        // Page break
        "break-before" => s.page_break_before = true,
        "break-after" => s.page_break_after = true,
        "break-inside-avoid" => s.page_break_inside_avoid = true,
        // Convenience classes for explicit page breaks in templates
        "page" | "page-break" => s.page_break_after = true,

        _ => {
            // Dynamic patterns
            try_parse_spacing_class(s, class);
            try_parse_color_class(s, class);
            try_parse_gap_class(s, class);
            try_parse_grid_cols_class(s, class);
            try_parse_width_class(s, class);
            try_parse_height_class(s, class);
        }
    }
}

fn try_parse_spacing_class(s: &mut ComputedStyle, class: &str) {
    // p-{n}, px-{n}, py-{n}, pt-{n}, etc.  (1 unit = 4px)
    // m-{n}, mx-{n}, my-{n}, mt-{n}, etc.
    let parts: Vec<&str> = class.rsplitn(2, '-').collect();
    if parts.len() != 2 {
        return;
    }
    let value_str = parts[0];
    let prefix = parts[1];
    let value: f32 = match value_str.parse::<f32>() {
        Ok(v) => v * 4.0,
        Err(_) => return,
    };

    match prefix {
        "p" => {
            s.padding_top = value;
            s.padding_right = value;
            s.padding_bottom = value;
            s.padding_left = value;
        }
        "px" => {
            s.padding_left = value;
            s.padding_right = value;
        }
        "py" => {
            s.padding_top = value;
            s.padding_bottom = value;
        }
        "pt" => s.padding_top = value,
        "pr" => s.padding_right = value,
        "pb" => s.padding_bottom = value,
        "pl" => s.padding_left = value,
        "m" => {
            s.margin_top = value;
            s.margin_right = value;
            s.margin_bottom = value;
            s.margin_left = value;
        }
        "mx" => {
            s.margin_left = value;
            s.margin_right = value;
        }
        "my" => {
            s.margin_top = value;
            s.margin_bottom = value;
        }
        "mt" => s.margin_top = value,
        "mr" => s.margin_right = value,
        "mb" => s.margin_bottom = value,
        "ml" => s.margin_left = value,
        _ => {}
    }
}

fn try_parse_color_class(s: &mut ComputedStyle, class: &str) {
    // Tailwind color subset: text-{color}, bg-{color}
    let colors = [
        (
            "red-500",
            Color {
                r: 0.937,
                g: 0.267,
                b: 0.267,
                a: 1.0,
            },
        ),
        (
            "red-700",
            Color {
                r: 0.725,
                g: 0.110,
                b: 0.110,
                a: 1.0,
            },
        ),
        (
            "blue-500",
            Color {
                r: 0.231,
                g: 0.510,
                b: 0.965,
                a: 1.0,
            },
        ),
        (
            "blue-700",
            Color {
                r: 0.102,
                g: 0.306,
                b: 0.827,
                a: 1.0,
            },
        ),
        (
            "green-500",
            Color {
                r: 0.133,
                g: 0.773,
                b: 0.369,
                a: 1.0,
            },
        ),
        (
            "green-700",
            Color {
                r: 0.082,
                g: 0.533,
                b: 0.247,
                a: 1.0,
            },
        ),
        (
            "gray-100",
            Color {
                r: 0.953,
                g: 0.957,
                b: 0.961,
                a: 1.0,
            },
        ),
        (
            "gray-200",
            Color {
                r: 0.898,
                g: 0.906,
                b: 0.922,
                a: 1.0,
            },
        ),
        (
            "gray-300",
            Color {
                r: 0.831,
                g: 0.843,
                b: 0.871,
                a: 1.0,
            },
        ),
        (
            "gray-500",
            Color {
                r: 0.424,
                g: 0.447,
                b: 0.502,
                a: 1.0,
            },
        ),
        (
            "gray-700",
            Color {
                r: 0.216,
                g: 0.255,
                b: 0.318,
                a: 1.0,
            },
        ),
        (
            "gray-900",
            Color {
                r: 0.067,
                g: 0.094,
                b: 0.153,
                a: 1.0,
            },
        ),
        ("white", Color::WHITE),
        ("black", Color::BLACK),
        (
            "yellow-500",
            Color {
                r: 0.918,
                g: 0.788,
                b: 0.153,
                a: 1.0,
            },
        ),
    ];

    for (name, color) in &colors {
        if class == format!("text-{}", name) {
            s.color = *color;
            return;
        }
        if class == format!("bg-{}", name) {
            s.background_color = *color;
            return;
        }
    }

    // border-{color}
    for (name, color) in &colors {
        if class == format!("border-{}", name) {
            s.border_color = *color;
            return;
        }
    }
}

fn try_parse_gap_class(s: &mut ComputedStyle, class: &str) {
    if let Some(rest) = class.strip_prefix("gap-") {
        if let Ok(v) = rest.parse::<f32>() {
            s.gap = v * 4.0;
        }
    }
}

fn try_parse_grid_cols_class(s: &mut ComputedStyle, class: &str) {
    if let Some(rest) = class.strip_prefix("grid-cols-") {
        if let Ok(n) = rest.parse::<usize>() {
            s.grid_template_columns = vec![GridTrack::Fr(1.0); n];
        }
    }
}

fn try_parse_width_class(s: &mut ComputedStyle, class: &str) {
    if let Some(rest) = class.strip_prefix("w-") {
        if let Ok(v) = rest.parse::<f32>() {
            s.width = Dimension::Px(v * 4.0);
        }
    }
}

fn try_parse_height_class(s: &mut ComputedStyle, class: &str) {
    if let Some(rest) = class.strip_prefix("h-") {
        if let Ok(v) = rest.parse::<f32>() {
            s.height = Dimension::Px(v * 4.0);
        }
    }
}

// ---------------------------------------------------------------------------
// Inline style parsing (limited subset)
// ---------------------------------------------------------------------------

fn apply_inline_style(s: &mut ComputedStyle, style_str: &str) {
    for decl in style_str.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }
        let mut parts = decl.splitn(2, ':');
        let prop = match parts.next() {
            Some(p) => p.trim(),
            None => continue,
        };
        let val = match parts.next() {
            Some(v) => v.trim(),
            None => continue,
        };
        apply_css_property(s, prop, val);
    }
}

fn apply_css_property(s: &mut ComputedStyle, prop: &str, val: &str) {
    match prop {
        "display" => {
            s.display = match val {
                "flex" => Display::Flex,
                "grid" => Display::Grid,
                "block" => Display::Block,
                "inline" => Display::Inline,
                "inline-block" => Display::InlineBlock,
                "none" => Display::None,
                _ => s.display,
            }
        }
        "flex-direction" => {
            s.flex_direction = match val {
                "row" => FlexDirection::Row,
                "column" => FlexDirection::Column,
                _ => s.flex_direction,
            }
        }
        "font-size" => {
            if let Some(px) = parse_px(val) {
                s.font_size = px;
            }
        }
        "font-weight" => {
            s.font_weight = match val {
                "bold" | "700" | "800" | "900" => FontWeight::Bold,
                _ => FontWeight::Normal,
            }
        }
        "font-style" => {
            s.font_style = match val {
                "italic" => FontStyle::Italic,
                _ => FontStyle::Normal,
            }
        }
        "color" => {
            if let Some(c) = Color::from_hex(val) {
                s.color = c;
            }
        }
        "background-color" | "background" => {
            if let Some(c) = Color::from_hex(val) {
                s.background_color = c;
            }
        }
        "text-align" => {
            s.text_align = match val {
                "center" => TextAlign::Center,
                "right" => TextAlign::Right,
                _ => TextAlign::Left,
            }
        }
        "width" => {
            s.width = parse_dimension(val);
        }
        "height" => {
            s.height = parse_dimension(val);
        }
        "margin" => apply_shorthand_spacing(
            val,
            &mut s.margin_top,
            &mut s.margin_right,
            &mut s.margin_bottom,
            &mut s.margin_left,
        ),
        "margin-top" => {
            if let Some(px) = parse_px(val) {
                s.margin_top = px;
            }
        }
        "margin-right" => {
            if let Some(px) = parse_px(val) {
                s.margin_right = px;
            }
        }
        "margin-bottom" => {
            if let Some(px) = parse_px(val) {
                s.margin_bottom = px;
            }
        }
        "margin-left" => {
            if let Some(px) = parse_px(val) {
                s.margin_left = px;
            }
        }
        "padding" => apply_shorthand_spacing(
            val,
            &mut s.padding_top,
            &mut s.padding_right,
            &mut s.padding_bottom,
            &mut s.padding_left,
        ),
        "padding-top" => {
            if let Some(px) = parse_px(val) {
                s.padding_top = px;
            }
        }
        "padding-right" => {
            if let Some(px) = parse_px(val) {
                s.padding_right = px;
            }
        }
        "padding-bottom" => {
            if let Some(px) = parse_px(val) {
                s.padding_bottom = px;
            }
        }
        "padding-left" => {
            if let Some(px) = parse_px(val) {
                s.padding_left = px;
            }
        }
        "border-width" | "border" => {
            if let Some(px) = parse_px(val) {
                s.border_width = px;
            }
        }
        "border-color" => {
            if let Some(c) = Color::from_hex(val) {
                s.border_color = c;
            }
        }
        "line-height" => {
            if let Ok(v) = val.parse::<f32>() {
                s.line_height = v;
            } else if let Some(px) = parse_px(val) {
                s.line_height = px / s.font_size;
            }
        }
        "gap" => {
            if let Some(px) = parse_px(val) {
                s.gap = px;
            }
        }
        "break-after" => {
            s.page_break_after = val == "always" || val == "page";
        }
        "break-before" => {
            s.page_break_before = val == "always" || val == "page";
        }
        "page-break-before" => {
            s.page_break_before = val == "always" || val == "page";
        }
        "page-break-after" => {
            s.page_break_after = val == "always" || val == "page";
        }
        "page-break-inside" => {
            s.page_break_inside_avoid = val == "avoid";
        }
        _ => {}
    }
}

fn parse_px(s: &str) -> Option<f32> {
    let s = s.trim().trim_end_matches("px");
    s.parse().ok()
}

fn parse_dimension(s: &str) -> Dimension {
    let s = s.trim();
    if s == "auto" {
        Dimension::Auto
    } else if s.ends_with('%') {
        s.trim_end_matches('%')
            .parse::<f32>()
            .map(Dimension::Percent)
            .unwrap_or(Dimension::Auto)
    } else {
        parse_px(s).map(Dimension::Px).unwrap_or(Dimension::Auto)
    }
}

fn apply_shorthand_spacing(
    val: &str,
    top: &mut f32,
    right: &mut f32,
    bottom: &mut f32,
    left: &mut f32,
) {
    let parts: Vec<f32> = val.split_whitespace().filter_map(|p| parse_px(p)).collect();
    match parts.len() {
        1 => {
            *top = parts[0];
            *right = parts[0];
            *bottom = parts[0];
            *left = parts[0];
        }
        2 => {
            *top = parts[0];
            *bottom = parts[0];
            *right = parts[1];
            *left = parts[1];
        }
        4 => {
            *top = parts[0];
            *right = parts[1];
            *bottom = parts[2];
            *left = parts[3];
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Styled DOM tree
// ---------------------------------------------------------------------------

/// A DOM node annotated with its computed style.
#[derive(Debug, Clone)]
pub enum StyledNode {
    Element {
        tag: Tag,
        style: ComputedStyle,
        children: Vec<StyledNode>,
        /// Original attributes (for images src, etc.)
        attrs: std::collections::HashMap<String, String>,
    },
    Text {
        text: String,
        style: ComputedStyle,
    },
}

/// Build a styled tree from a DOM tree, resolving styles top-down.
pub fn build_styled_tree(
    nodes: &[DomNode],
    parent_style: Option<&ComputedStyle>,
) -> Vec<StyledNode> {
    let mut result = Vec::new();
    for node in nodes {
        match node {
            DomNode::Element(e) => {
                let style = resolve_style(e, parent_style);
                let children = build_styled_tree(&e.children, Some(&style));
                result.push(StyledNode::Element {
                    tag: e.tag.clone(),
                    style,
                    children,
                    attrs: e.attributes.clone(),
                });
            }
            DomNode::Text(text) => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    let mut style = parent_style.cloned().unwrap_or_default();
                    // Text nodes render inline — clear all box-model properties
                    // that must not be inherited (border, background, spacing).
                    style.border_width = 0.0;
                    style.background_color = Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    };
                    style.margin_top = 0.0;
                    style.margin_right = 0.0;
                    style.margin_bottom = 0.0;
                    style.margin_left = 0.0;
                    style.padding_top = 0.0;
                    style.padding_right = 0.0;
                    style.padding_bottom = 0.0;
                    style.padding_left = 0.0;
                    result.push(StyledNode::Text {
                        text: text.clone(),
                        style,
                    });
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailwind_padding() {
        let mut s = ComputedStyle::default();
        apply_tailwind_class(&mut s, "p-4");
        assert_eq!(s.padding_top, 16.0);
        assert_eq!(s.padding_left, 16.0);
    }

    #[test]
    fn inline_style_font_size() {
        let mut s = ComputedStyle::default();
        apply_inline_style(&mut s, "font-size: 24px; color: #ff0000");
        assert_eq!(s.font_size, 24.0);
        assert!((s.color.r - 1.0).abs() < 0.01);
    }

    #[test]
    fn color_from_hex() {
        let c = Color::from_hex("#ff8800").unwrap();
        assert!((c.r - 1.0).abs() < 0.01);
        assert!((c.g - 0.533).abs() < 0.01);
    }
}

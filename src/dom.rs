//! HTML parser – converts an HTML string into a simple DOM tree.
//!
//! We support a controlled subset of elements:
//! - Structural: div, p, h1-h3, ul, ol, li, table, tr, td, th, img
//! - Inline: span
//! - Styling via `class` and `style` attributes

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// DOM types
// ---------------------------------------------------------------------------

/// The tag name of a supported element.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Tag {
    Div,
    P,
    H1,
    H2,
    H3,
    Ul,
    Ol,
    Li,
    Table,
    Tr,
    Td,
    Th,
    Span,
    Img,
    Body,
    Html,
    Head,
    /// Catch-all for unknown tags – they are kept but treated as divs.
    Unknown(String),
}

impl Tag {
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "div" => Tag::Div,
            "p" => Tag::P,
            "h1" => Tag::H1,
            "h2" => Tag::H2,
            "h3" => Tag::H3,
            "ul" => Tag::Ul,
            "ol" => Tag::Ol,
            "li" => Tag::Li,
            "table" => Tag::Table,
            "tr" => Tag::Tr,
            "td" => Tag::Td,
            "th" => Tag::Th,
            "span" => Tag::Span,
            "img" => Tag::Img,
            "body" => Tag::Body,
            "html" => Tag::Html,
            "head" => Tag::Head,
            _ => Tag::Unknown(s.to_string()),
        }
    }

    pub fn is_block(&self) -> bool {
        matches!(
            self,
            Tag::Div
                | Tag::P
                | Tag::H1
                | Tag::H2
                | Tag::H3
                | Tag::Ul
                | Tag::Ol
                | Tag::Li
                | Tag::Table
                | Tag::Tr
                | Tag::Td
                | Tag::Th
                | Tag::Body
                | Tag::Html
                | Tag::Unknown(_)
        )
    }

    pub fn is_inline(&self) -> bool {
        matches!(self, Tag::Span)
    }

    pub fn is_table_part(&self) -> bool {
        matches!(self, Tag::Table | Tag::Tr | Tag::Td | Tag::Th)
    }
}

/// A node in our DOM tree.
#[derive(Debug, Clone)]
pub enum DomNode {
    Element(ElementNode),
    Text(String),
}

/// An element node carrying tag, attributes, and children.
#[derive(Debug, Clone)]
pub struct ElementNode {
    pub tag: Tag,
    pub attributes: HashMap<String, String>,
    pub children: Vec<DomNode>,
}

impl ElementNode {
    pub fn new(tag: Tag) -> Self {
        Self {
            tag,
            attributes: HashMap::new(),
            children: Vec::new(),
        }
    }

    pub fn classes(&self) -> Vec<&str> {
        self.attributes
            .get("class")
            .map(|c| c.split_whitespace().collect())
            .unwrap_or_default()
    }

    pub fn inline_style(&self) -> Option<&str> {
        self.attributes.get("style").map(|s| s.as_str())
    }

    pub fn src(&self) -> Option<&str> {
        self.attributes.get("src").map(|s| s.as_str())
    }
}

// ---------------------------------------------------------------------------
// Parser – simple recursive descent over HTML
// ---------------------------------------------------------------------------

/// Parse an HTML string into a list of DOM nodes.
///
/// We use a hand-written parser that handles the controlled subset. This keeps
/// dependencies minimal and avoids the complexity of a full HTML5 parser for
/// our constrained template inputs.
pub fn parse_html(html: &str) -> Vec<DomNode> {
    let mut parser = Parser::new(html);
    parser.parse_nodes()
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse_nodes(&mut self) -> Vec<DomNode> {
        let mut nodes = Vec::new();
        loop {
            self.skip_whitespace_preserve();
            if self.eof() || self.starts_with("</") {
                break;
            }
            if let Some(node) = self.parse_node() {
                nodes.push(node);
            }
        }
        nodes
    }

    fn parse_node(&mut self) -> Option<DomNode> {
        if self.starts_with("<!--") {
            self.skip_comment();
            return None;
        }
        if self.starts_with("<!") || self.starts_with("<?") {
            // Skip doctype / processing instructions
            while !self.eof() && !self.starts_with(">") {
                self.advance(1);
            }
            if !self.eof() {
                self.advance(1); // skip '>'
            }
            return None;
        }
        if self.starts_with("<") {
            Some(self.parse_element())
        } else {
            Some(self.parse_text())
        }
    }

    fn parse_text(&mut self) -> DomNode {
        let start = self.pos;
        while !self.eof() && !self.starts_with("<") {
            self.advance(1);
        }
        let text = &self.input[start..self.pos];
        DomNode::Text(decode_entities(text))
    }

    fn parse_element(&mut self) -> DomNode {
        // Consume '<'
        self.advance(1);
        let tag_name = self.parse_tag_name();
        let tag = Tag::from_str(&tag_name);
        let mut elem = ElementNode::new(tag.clone());

        // Parse attributes
        loop {
            self.skip_whitespace();
            if self.eof() || self.starts_with(">") || self.starts_with("/>") {
                break;
            }
            let (key, value) = self.parse_attribute();
            elem.attributes.insert(key, value);
        }

        // Self-closing tags
        let self_closing = tag == Tag::Img;
        if self.starts_with("/>") {
            self.advance(2);
            return DomNode::Element(elem);
        }
        if self.starts_with(">") {
            self.advance(1);
        }
        if self_closing {
            return DomNode::Element(elem);
        }

        // Parse children
        elem.children = self.parse_nodes();

        // Consume closing tag
        if self.starts_with("</") {
            self.advance(2);
            self.parse_tag_name(); // skip tag name
            self.skip_whitespace();
            if self.starts_with(">") {
                self.advance(1);
            }
        }

        DomNode::Element(elem)
    }

    fn parse_tag_name(&mut self) -> String {
        let start = self.pos;
        while !self.eof() {
            let c = self.current_char();
            if c.is_alphanumeric() || c == '-' || c == '_' {
                self.advance(1);
            } else {
                break;
            }
        }
        self.input[start..self.pos].to_string()
    }

    fn parse_attribute(&mut self) -> (String, String) {
        let key = self.parse_tag_name();
        self.skip_whitespace();
        if !self.starts_with("=") {
            return (key, String::new());
        }
        self.advance(1); // skip '='
        self.skip_whitespace();
        let value = self.parse_attr_value();
        (key, value)
    }

    fn parse_attr_value(&mut self) -> String {
        if self.starts_with("\"") {
            self.advance(1);
            let start = self.pos;
            while !self.eof() && !self.starts_with("\"") {
                self.advance(1);
            }
            let val = self.input[start..self.pos].to_string();
            if !self.eof() {
                self.advance(1);
            }
            decode_entities(&val)
        } else if self.starts_with("'") {
            self.advance(1);
            let start = self.pos;
            while !self.eof() && !self.starts_with("'") {
                self.advance(1);
            }
            let val = self.input[start..self.pos].to_string();
            if !self.eof() {
                self.advance(1);
            }
            decode_entities(&val)
        } else {
            let start = self.pos;
            while !self.eof() {
                let c = self.current_char();
                if c.is_whitespace() || c == '>' || c == '/' {
                    break;
                }
                self.advance(1);
            }
            self.input[start..self.pos].to_string()
        }
    }

    fn skip_whitespace(&mut self) {
        while !self.eof() && self.current_char().is_whitespace() {
            self.advance(1);
        }
    }

    fn skip_whitespace_preserve(&mut self) {
        // Skip runs of pure whitespace between elements.
        let saved = self.pos;
        while !self.eof() && self.current_char().is_whitespace() {
            self.advance(1);
        }
        // If we reached a tag or EOF, keep the skip. Otherwise revert.
        if !self.eof() && !self.starts_with("<") {
            self.pos = saved;
        }
    }

    fn skip_comment(&mut self) {
        self.advance(4); // skip <!--
        while !self.eof() && !self.starts_with("-->") {
            self.advance(1);
        }
        if !self.eof() {
            self.advance(3);
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn current_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap()
    }

    fn advance(&mut self, n: usize) {
        // Advance by `n` characters (not bytes).
        for _ in 0..n {
            if let Some(c) = self.input[self.pos..].chars().next() {
                self.pos += c.len_utf8();
            }
        }
    }
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", "\u{00A0}")
}

// ---------------------------------------------------------------------------
// Convenience helpers
// ---------------------------------------------------------------------------

/// Find the `<body>` element and return its children, or return all nodes if
/// no `<body>` is present.
pub fn body_children(nodes: &[DomNode]) -> Vec<DomNode> {
    for node in nodes {
        if let DomNode::Element(e) = node {
            if e.tag == Tag::Body {
                return e.children.clone();
            }
            // Recurse into <html>
            if e.tag == Tag::Html {
                let inner = body_children(&e.children);
                if !inner.is_empty() {
                    return inner;
                }
            }
        }
    }
    nodes.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_div() {
        let html = r#"<div class="flex p-4"><p>Hello</p></div>"#;
        let nodes = parse_html(html);
        assert_eq!(nodes.len(), 1);
        if let DomNode::Element(e) = &nodes[0] {
            assert_eq!(e.tag, Tag::Div);
            assert_eq!(e.classes(), vec!["flex", "p-4"]);
            assert_eq!(e.children.len(), 1);
        } else {
            panic!("Expected element");
        }
    }

    #[test]
    fn parse_self_closing_img() {
        let html = r#"<img src="logo.png" />"#;
        let nodes = parse_html(html);
        assert_eq!(nodes.len(), 1);
        if let DomNode::Element(e) = &nodes[0] {
            assert_eq!(e.tag, Tag::Img);
            assert_eq!(e.src(), Some("logo.png"));
        } else {
            panic!("Expected img element");
        }
    }

    #[test]
    fn parse_nested_spans() {
        let html = r#"<p>Hello <span class="font-bold">world</span>!</p>"#;
        let nodes = parse_html(html);
        assert_eq!(nodes.len(), 1);
        if let DomNode::Element(e) = &nodes[0] {
            assert_eq!(e.tag, Tag::P);
            assert_eq!(e.children.len(), 3); // "Hello ", <span>, "!"
        } else {
            panic!("Expected p element");
        }
    }

    #[test]
    fn parse_table() {
        let html = r#"<table><tr><th>Name</th><th>Age</th></tr><tr><td>Alice</td><td>30</td></tr></table>"#;
        let nodes = parse_html(html);
        assert_eq!(nodes.len(), 1);
        if let DomNode::Element(table) = &nodes[0] {
            assert_eq!(table.tag, Tag::Table);
            assert_eq!(table.children.len(), 2); // 2 rows
        } else {
            panic!("Expected table");
        }
    }
}

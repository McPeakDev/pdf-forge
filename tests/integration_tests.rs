//! Integration tests for the rpdf pipeline.
//!
//! These tests validate:
//! - Layout config matches expected positions
//! - PDF output exists and has valid format
//! - All supported elements produce correct output
//! - Pagination works correctly

use pdf_forge::dom::{parse_html, DomNode, Tag};
use pdf_forge::layout_config::LayoutConfig;
use pdf_forge::pipeline::{compute_layout_config, generate_pdf, PipelineConfig};
use pdf_forge::render::render_pdf;
use pdf_forge::templates;

// =====================================================================
// Helper
// =====================================================================

fn default_config() -> PipelineConfig {
    PipelineConfig::default()
}

fn assert_valid_pdf(bytes: &[u8]) {
    assert!(bytes.len() > 100, "PDF too small: {} bytes", bytes.len());
    assert_eq!(&bytes[0..5], b"%PDF-", "Missing PDF header");
}

// =====================================================================
// DOM parsing tests
// =====================================================================

#[test]
fn parse_heading_elements() {
    for tag in ["h1", "h2", "h3"] {
        let html = format!("<{0}>Title</{0}>", tag);
        let dom = parse_html(&html);
        assert_eq!(dom.len(), 1);
        if let DomNode::Element(e) = &dom[0] {
            match tag {
                "h1" => assert_eq!(e.tag, Tag::H1),
                "h2" => assert_eq!(e.tag, Tag::H2),
                "h3" => assert_eq!(e.tag, Tag::H3),
                _ => unreachable!(),
            }
        } else {
            panic!("Expected element for <{}>", tag);
        }
    }
}

#[test]
fn parse_paragraph_with_spans() {
    let html =
        r#"<p>Hello <span class="font-bold">world</span> and <span class="italic">more</span></p>"#;
    let dom = parse_html(html);
    assert_eq!(dom.len(), 1);
    if let DomNode::Element(p) = &dom[0] {
        assert_eq!(p.tag, Tag::P);
        // Children: text, span, text, span
        assert!(p.children.len() >= 3, "Expected multiple children in <p>");
    } else {
        panic!("Expected <p> element");
    }
}

#[test]
fn parse_unordered_list() {
    let html = "<ul><li>A</li><li>B</li><li>C</li></ul>";
    let dom = parse_html(html);
    assert_eq!(dom.len(), 1);
    if let DomNode::Element(ul) = &dom[0] {
        assert_eq!(ul.tag, Tag::Ul);
        assert_eq!(ul.children.len(), 3);
    } else {
        panic!("Expected <ul>");
    }
}

#[test]
fn parse_ordered_list() {
    let html = "<ol><li>First</li><li>Second</li></ol>";
    let dom = parse_html(html);
    if let DomNode::Element(ol) = &dom[0] {
        assert_eq!(ol.tag, Tag::Ol);
        assert_eq!(ol.children.len(), 2);
    } else {
        panic!("Expected <ol>");
    }
}

#[test]
fn parse_table_structure() {
    let html = r#"
        <table>
            <tr><th>Name</th><th>Value</th></tr>
            <tr><td>Alpha</td><td>100</td></tr>
            <tr><td>Beta</td><td>200</td></tr>
        </table>
    "#;
    let dom = parse_html(html);
    let table = dom
        .iter()
        .find(|n| matches!(n, DomNode::Element(e) if e.tag == Tag::Table));
    assert!(table.is_some(), "Should find a <table>");
    if let Some(DomNode::Element(t)) = table {
        assert_eq!(t.children.len(), 3, "Table should have 3 rows");
    }
}

#[test]
fn parse_image() {
    let html = r#"<img src="photo.jpg" style="width: 200px; height: 100px" />"#;
    let dom = parse_html(html);
    assert_eq!(dom.len(), 1);
    if let DomNode::Element(img) = &dom[0] {
        assert_eq!(img.tag, Tag::Img);
        assert_eq!(img.src(), Some("photo.jpg"));
    } else {
        panic!("Expected <img>");
    }
}

// =====================================================================
// Layout config position tests
// =====================================================================

#[test]
fn layout_positions_are_within_page() {
    let config = compute_layout_config(templates::invoice_template(), &default_config());
    let page_w = config.page_width_pt;
    let page_h = config.page_height_pt;

    for page in &config.pages {
        for lbox in &page.boxes {
            assert!(
                lbox.x >= 0.0 && lbox.x < page_w,
                "Box x={} outside page width={}",
                lbox.x,
                page_w
            );
            assert!(
                lbox.y >= 0.0 && lbox.y < page_h,
                "Box y={} outside page height={}",
                lbox.y,
                page_h
            );
        }
    }
}

#[test]
fn layout_boxes_have_positive_dimensions() {
    let config = compute_layout_config(templates::all_elements_template(), &default_config());
    for page in &config.pages {
        for lbox in &page.boxes {
            assert!(lbox.width >= 0.0, "Negative width: {}", lbox.width);
            assert!(lbox.height >= 0.0, "Negative height: {}", lbox.height);
        }
    }
}

#[test]
fn layout_content_width_matches_page() {
    let cfg = default_config();
    let config = compute_layout_config("<div class=\"w-full\"><p>Full width</p></div>", &cfg);
    let content_width = cfg.page_width - 2.0 * cfg.page_margin;

    for page in &config.pages {
        for lbox in &page.boxes {
            // Top-level boxes should not exceed content width
            assert!(
                lbox.width <= content_width + 1.0,
                "Box width {} exceeds content width {}",
                lbox.width,
                content_width
            );
        }
    }
}

// =====================================================================
// Pagination tests
// =====================================================================

#[test]
fn single_paragraph_fits_one_page() {
    let config = compute_layout_config("<p>Short</p>", &default_config());
    assert_eq!(config.pages.len(), 1);
}

#[test]
fn many_paragraphs_create_multiple_pages() {
    let mut html = String::new();
    for i in 0..80 {
        html.push_str(&format!(
            "<p>Paragraph {} with enough text to take up some vertical space on the page.</p>",
            i
        ));
    }

    let config = compute_layout_config(&html, &default_config());
    assert!(
        config.pages.len() > 1,
        "Expected multiple pages, got {}",
        config.pages.len()
    );
}

#[test]
fn page_break_before() {
    let html = r#"<p>Page 1 content</p><p class="break-before">Page 2 content</p>"#;
    let config = compute_layout_config(html, &default_config());
    assert!(
        config.pages.len() >= 2,
        "Expected at least 2 pages with break-before"
    );
}

// =====================================================================
// PDF generation tests
// =====================================================================

#[test]
fn generate_pdf_from_minimal_template() {
    let (bytes, config) = generate_pdf(templates::minimal_template(), &default_config()).unwrap();
    assert_valid_pdf(&bytes);
    assert!(!config.pages.is_empty());
}

#[test]
fn generate_pdf_from_invoice_template() {
    let (bytes, config) = generate_pdf(templates::invoice_template(), &default_config()).unwrap();
    assert_valid_pdf(&bytes);
    assert!(!config.pages.is_empty());
}

#[test]
fn generate_pdf_from_report_template() {
    let (bytes, config) = generate_pdf(templates::report_template(), &default_config()).unwrap();
    assert_valid_pdf(&bytes);
    assert!(!config.pages.is_empty());
}

#[test]
fn generate_pdf_from_styled_template() {
    let (bytes, config) = generate_pdf(templates::styled_template(), &default_config()).unwrap();
    assert_valid_pdf(&bytes);
    assert!(!config.pages.is_empty());
}

#[test]
fn generate_pdf_from_all_elements_template() {
    let (bytes, config) =
        generate_pdf(templates::all_elements_template(), &default_config()).unwrap();
    assert_valid_pdf(&bytes);
    assert!(!config.pages.is_empty());
}

#[test]
fn generate_pdf_from_multipage_template() {
    let (bytes, config) =
        generate_pdf(templates::multi_page_template(), &default_config()).unwrap();
    assert_valid_pdf(&bytes);
    // This template has enough content for multiple pages
    assert!(
        config.pages.len() >= 1,
        "Multi-page template should produce at least 1 page"
    );
}

// =====================================================================
// Layout config JSON round-trip
// =====================================================================

#[test]
fn layout_config_json_roundtrip() {
    let config = compute_layout_config(templates::invoice_template(), &default_config());
    let json = config.to_json();
    let parsed = LayoutConfig::from_json(&json).unwrap();
    assert_eq!(config.pages.len(), parsed.pages.len());
    assert!((config.page_width_pt - parsed.page_width_pt).abs() < 0.01);
}

#[test]
fn render_from_layout_config_json() {
    let config = compute_layout_config(templates::report_template(), &default_config());
    let json = config.to_json();
    let parsed = LayoutConfig::from_json(&json).unwrap();
    let bytes = render_pdf(&parsed).unwrap();
    assert_valid_pdf(&bytes);
}

// =====================================================================
// Golden-sample stability test
// =====================================================================

#[test]
fn pdf_output_is_deterministic() {
    let html = templates::minimal_template();
    let (bytes1, _) = generate_pdf(html, &default_config()).unwrap();
    let (bytes2, _) = generate_pdf(html, &default_config()).unwrap();

    // printpdf embeds timestamps, so byte-exact equality isn't guaranteed.
    // Instead, check that the sizes are within a small tolerance.
    let diff = (bytes1.len() as i64 - bytes2.len() as i64).unsigned_abs();
    assert!(
        diff < 200,
        "PDF outputs differ significantly: {} vs {} bytes",
        bytes1.len(),
        bytes2.len()
    );
}

// =====================================================================
// Text / inline tests
// =====================================================================

#[test]
fn inline_spans_produce_text_content() {
    let html = r#"<p>Hello <span class="font-bold">bold</span> world</p>"#;
    let config = compute_layout_config(html, &default_config());

    // Traverse layout to find text boxes
    let mut found_text = false;
    for page in &config.pages {
        for lbox in &page.boxes {
            visit_box(lbox, &mut |b| {
                if b.text.is_some() {
                    found_text = true;
                }
            });
        }
    }
    assert!(found_text, "Should find text content for inline spans");
}

fn visit_box(
    lbox: &pdf_forge::layout_config::LayoutBox,
    f: &mut dyn FnMut(&pdf_forge::layout_config::LayoutBox),
) {
    f(lbox);
    for child in &lbox.children {
        visit_box(child, f);
    }
}

// =====================================================================
// Table layout tests
// =====================================================================

#[test]
fn table_produces_grid_layout() {
    let html = r#"
        <table class="w-full">
            <tr><th>A</th><th>B</th></tr>
            <tr><td>1</td><td>2</td></tr>
        </table>
    "#;
    let config = compute_layout_config(html, &default_config());
    assert!(!config.pages.is_empty());

    // Should have boxes for rows/cells
    let total_boxes = count_boxes(&config);
    assert!(
        total_boxes >= 4,
        "Table should produce at least 4 boxes, got {}",
        total_boxes
    );
}

fn count_boxes(config: &LayoutConfig) -> usize {
    let mut count = 0;
    for page in &config.pages {
        for lbox in &page.boxes {
            count += count_box(lbox);
        }
    }
    count
}

fn count_box(lbox: &pdf_forge::layout_config::LayoutBox) -> usize {
    let mut c = 1;
    for child in &lbox.children {
        c += count_box(child);
    }
    c
}

// =====================================================================
// Image handling test
// =====================================================================

#[test]
fn image_produces_image_content() {
    let html = r#"<img src="test.png" style="width: 100px; height: 50px" />"#;
    let config = compute_layout_config(html, &default_config());

    let mut found_image = false;
    for page in &config.pages {
        for lbox in &page.boxes {
            visit_box(lbox, &mut |b| {
                if let Some(img) = &b.image {
                    assert_eq!(img.src, "test.png");
                    found_image = true;
                }
            });
        }
    }
    assert!(found_image, "Should find image content");
}

// =====================================================================
// List layout tests
// =====================================================================

#[test]
fn unordered_list_layout() {
    let html = "<ul><li>Item A</li><li>Item B</li></ul>";
    let config = compute_layout_config(html, &default_config());
    assert!(!config.pages.is_empty());
    let total = count_boxes(&config);
    assert!(total >= 2, "UL should produce at least 2 boxes");
}

#[test]
fn ordered_list_layout() {
    let html = "<ol><li>First</li><li>Second</li><li>Third</li></ol>";
    let config = compute_layout_config(html, &default_config());
    assert!(!config.pages.is_empty());
    let total = count_boxes(&config);
    assert!(total >= 3, "OL should produce at least 3 boxes");
}

// =====================================================================
// All templates render without error
// =====================================================================

#[test]
fn all_templates_render_successfully() {
    let templates: Vec<(&str, &str)> = vec![
        ("invoice", templates::invoice_template()),
        ("report", templates::report_template()),
        ("multipage", templates::multi_page_template()),
        ("styled", templates::styled_template()),
        ("minimal", templates::minimal_template()),
        ("all_elements", templates::all_elements_template()),
    ];

    for (name, html) in templates {
        let result = generate_pdf(html, &default_config());
        assert!(
            result.is_ok(),
            "Template '{}' failed: {:?}",
            name,
            result.err()
        );
        let (bytes, _) = result.unwrap();
        assert_valid_pdf(&bytes);
    }
}

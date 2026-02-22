//! PDF renderer – takes a [`LayoutConfig`] and produces PDF bytes using
//! `printpdf` (v0.8 ops-based API).

use std::collections::{HashMap, HashSet};

use base64::{engine::general_purpose::STANDARD as BASE64_STD, Engine as _};
use printpdf::*;

use crate::layout_config::*;

/// A printpdf XObject together with the pixel dimensions of the source image.
struct ImageResource {
    xobj_id: XObjectId,
    px_width: u32,
    px_height: u32,
}

/// Render a LayoutConfig into PDF bytes.
///
/// `<img>` elements whose `src` is not a base64 data URI, or whose bytes
/// cannot be decoded, are silently skipped (a `log::warn` is emitted).
pub fn render_pdf(config: &LayoutConfig) -> Result<Vec<u8>, String> {
    let page_w = Mm(config.page_width_pt * 0.352778); // pt → mm
    let page_h = Mm(config.page_height_pt * 0.352778);

    let mut doc = PdfDocument::new(&config.title);

    // ── Pre-register all images ────────────────────────────────────────────
    let mut all_srcs: HashSet<&str> = HashSet::new();
    for page_layout in &config.pages {
        for lbox in &page_layout.boxes {
            collect_image_srcs(lbox, &mut all_srcs);
        }
    }

    let mut image_resources: HashMap<String, ImageResource> = HashMap::new();
    let mut img_warnings: Vec<PdfWarnMsg> = Vec::new();

    for src in &all_srcs {
        let bytes = match parse_data_uri(src) {
            Ok(b) => b,
            Err(e) => {
                log::warn!("Skipping image — {e}");
                continue;
            }
        };

        // Decode with the `image` crate to obtain pixel dimensions.
        let dyn_img = match ::image::load_from_memory(&bytes) {
            Ok(img) => img,
            Err(e) => {
                log::warn!("Skipping image — decode error: {e}");
                continue;
            }
        };
        let (px_width, px_height) = (dyn_img.width(), dyn_img.height());

        // Register with printpdf as a reusable XObject.
        let raw = match RawImage::decode_from_bytes(&bytes, &mut img_warnings) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Skipping image — PDF encode error: {e}");
                continue;
            }
        };
        let xobj_id = doc.add_image(&raw);

        image_resources.insert(
            src.to_string(),
            ImageResource {
                xobj_id,
                px_width,
                px_height,
            },
        );
    }

    // ── Render pages ──────────────────────────────────────────────────────
    let mut pages = Vec::new();

    for page_layout in &config.pages {
        let mut ops = Vec::new();

        for lbox in &page_layout.boxes {
            render_box(&mut ops, lbox, config.page_height_pt, &image_resources);
        }

        let page = PdfPage::new(page_w, page_h, ops);
        pages.push(page);
    }

    // Ensure at least one page.
    if pages.is_empty() {
        pages.push(PdfPage::new(page_w, page_h, Vec::new()));
    }

    doc.with_pages(pages);
    let bytes = doc.save(&PdfSaveOptions::default(), &mut Vec::new());

    Ok(bytes)
}

/// Convert a UTF-8 string to raw Windows-1252 bytes then wrap in a String so
/// printpdf writes the bytes unchanged into the PDF stream (builtin fonts use
/// WinAnsiEncoding, so each glyph is one byte 0x00–0xFF).
fn to_winlatin(s: &str) -> String {
    let bytes: Vec<u8> = s
        .chars()
        .map(|c| match c {
            '\u{20AC}' => 0x80, // euro
            '\u{201A}' => 0x82, // single low-9 quote
            '\u{201E}' => 0x84, // double low-9 quote
            '\u{2026}' => 0x85, // ellipsis
            '\u{2018}' => 0x91, // left single quote
            '\u{2019}' => 0x92, // right single quote
            '\u{201C}' => 0x93, // left double quote
            '\u{201D}' => 0x94, // right double quote
            '\u{2022}' => 0x95, // bullet
            '\u{2013}' => 0x96, // en-dash
            '\u{2014}' => 0x97, // em-dash
            '\u{2122}' => 0x99, // trademark
            '\u{00A0}' => 0x20, // non-breaking space -> space
            c if (c as u32) < 256 => c as u8,
            _ => b'?',
        })
        .collect();
    // SAFETY: intentionally non-UTF-8 for 0x80-0x9F range; printpdf passes
    // these bytes straight to the PDF stream, decoded by WinAnsiEncoding.
    #[allow(unsafe_code)]
    unsafe {
        String::from_utf8_unchecked(bytes)
    }
}

/// Parse a `data:<mime>;base64,<data>` URI and return the raw decoded bytes.
///
/// Returns `Err` if `src` is not a data URI or does not use base64 encoding.
fn parse_data_uri(src: &str) -> Result<Vec<u8>, String> {
    if !src.starts_with("data:") {
        let preview = if src.len() > 80 { &src[..80] } else { src };
        return Err(format!(
            "Image src must be a base64 data URI \
             (e.g. `data:image/png;base64,...`). Got: {preview:?}"
        ));
    }
    let rest = &src["data:".len()..];
    let comma_pos = rest.find(',').ok_or_else(|| {
        "Invalid data URI: missing `,` separator between header and data".to_string()
    })?;
    let header = &rest[..comma_pos];
    if !header.contains(";base64") {
        return Err("Only base64-encoded data URIs are supported. \
             The header must contain `;base64` (e.g. `data:image/png;base64,...`)."
            .to_string());
    }
    let b64_data = rest[comma_pos + 1..].trim();
    BASE64_STD
        .decode(b64_data)
        .map_err(|e| format!("Base64 decode error: {e}"))
}

/// Recursively collect all unique `image.src` strings from a [`LayoutBox`] tree.
fn collect_image_srcs<'a>(lbox: &'a LayoutBox, srcs: &mut HashSet<&'a str>) {
    if let Some(img) = &lbox.image {
        srcs.insert(img.src.as_str());
    }
    for child in &lbox.children {
        collect_image_srcs(child, srcs);
    }
}

/// Recursively render a LayoutBox and its children into PDF ops.
fn render_box(
    ops: &mut Vec<Op>,
    lbox: &LayoutBox,
    page_height: f32,
    images: &HashMap<String, ImageResource>,
) {
    // PDF coordinate system: origin at bottom-left.
    // Our layout uses origin at top-left. Convert:
    let pdf_y = page_height - lbox.y;

    // Background
    if let Some(bg) = &lbox.background_color {
        ops.push(Op::SetFillColor {
            col: Color::Rgb(Rgb {
                r: bg[0],
                g: bg[1],
                b: bg[2],
                icc_profile: None,
            }),
        });

        // Draw filled rectangle
        let x1 = lbox.x;
        let y1 = pdf_y - lbox.height;
        let x2 = lbox.x + lbox.width;
        let y2 = pdf_y;

        ops.push(Op::DrawPolygon {
            polygon: Polygon {
                rings: vec![PolygonRing {
                    points: vec![
                        LinePoint {
                            p: Point {
                                x: Pt(x1),
                                y: Pt(y1),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt(x2),
                                y: Pt(y1),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt(x2),
                                y: Pt(y2),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt(x1),
                                y: Pt(y2),
                            },
                            bezier: false,
                        },
                    ],
                }],
                mode: PaintMode::Fill,
                winding_order: WindingOrder::NonZero,
            },
        });
    }

    // Border
    if let Some(border) = &lbox.border {
        ops.push(Op::SetOutlineColor {
            col: Color::Rgb(Rgb {
                r: border.color[0],
                g: border.color[1],
                b: border.color[2],
                icc_profile: None,
            }),
        });
        ops.push(Op::SetOutlineThickness {
            pt: Pt(border.width),
        });

        let x1 = lbox.x;
        let y1 = pdf_y - lbox.height;
        let x2 = lbox.x + lbox.width;
        let y2 = pdf_y;

        ops.push(Op::DrawLine {
            line: Line {
                points: vec![
                    LinePoint {
                        p: Point {
                            x: Pt(x1),
                            y: Pt(y2),
                        },
                        bezier: false,
                    },
                    LinePoint {
                        p: Point {
                            x: Pt(x2),
                            y: Pt(y2),
                        },
                        bezier: false,
                    },
                    LinePoint {
                        p: Point {
                            x: Pt(x2),
                            y: Pt(y1),
                        },
                        bezier: false,
                    },
                    LinePoint {
                        p: Point {
                            x: Pt(x1),
                            y: Pt(y1),
                        },
                        bezier: false,
                    },
                ],
                is_closed: true,
            },
        });
    }

    // Text
    if let Some(text) = &lbox.text {
        let font = match (text.bold, text.italic) {
            (true, true) => BuiltinFont::HelveticaBoldOblique,
            (true, false) => BuiltinFont::HelveticaBold,
            (false, true) => BuiltinFont::HelveticaOblique,
            (false, false) => BuiltinFont::Helvetica,
        };

        for tline in &text.lines {
            if tline.text.is_empty() {
                continue;
            }
            let text_x = lbox.x + tline.x_offset;
            // Baseline ≈ top of line + ascender (approx 0.75 × font_size)
            let ascender_offset = text.font_size * 0.75;
            let text_y = pdf_y - tline.y_offset - ascender_offset;

            ops.push(Op::StartTextSection);
            ops.push(Op::SetTextCursor {
                pos: Point {
                    x: Pt(text_x),
                    y: Pt(text_y),
                },
            });
            ops.push(Op::SetFontSizeBuiltinFont {
                size: Pt(text.font_size),
                font,
            });
            ops.push(Op::SetLineHeight {
                lh: Pt(text.line_height),
            });
            ops.push(Op::SetFillColor {
                col: Color::Rgb(Rgb {
                    r: text.color[0],
                    g: text.color[1],
                    b: text.color[2],
                    icc_profile: None,
                }),
            });
            ops.push(Op::WriteTextBuiltinFont {
                items: vec![TextItem::Text(to_winlatin(&tline.text))],
                font,
            });
            ops.push(Op::EndTextSection);

            // Underline
            if text.underline {
                let underline_y = text_y - text.font_size * 0.1;
                ops.push(Op::SetOutlineThickness { pt: Pt(0.5) });
                ops.push(Op::SetOutlineColor {
                    col: Color::Rgb(Rgb {
                        r: text.color[0],
                        g: text.color[1],
                        b: text.color[2],
                        icc_profile: None,
                    }),
                });
                ops.push(Op::DrawLine {
                    line: Line {
                        points: vec![
                            LinePoint {
                                p: Point {
                                    x: Pt(text_x),
                                    y: Pt(underline_y),
                                },
                                bezier: false,
                            },
                            LinePoint {
                                p: Point {
                                    x: Pt(text_x + lbox.width),
                                    y: Pt(underline_y),
                                },
                                bezier: false,
                            },
                        ],
                        is_closed: false,
                    },
                });
            }
        }

        // List marker
        if let Some(marker) = &text.list_marker {
            let marker_x = lbox.x - 16.0;
            let marker_y = pdf_y - text.font_size * 0.75;
            ops.push(Op::StartTextSection);
            ops.push(Op::SetTextCursor {
                pos: Point {
                    x: Pt(marker_x),
                    y: Pt(marker_y),
                },
            });
            ops.push(Op::SetFontSizeBuiltinFont {
                size: Pt(text.font_size),
                font: BuiltinFont::Helvetica,
            });
            ops.push(Op::SetFillColor {
                col: Color::Rgb(Rgb {
                    r: text.color[0],
                    g: text.color[1],
                    b: text.color[2],
                    icc_profile: None,
                }),
            });
            ops.push(Op::WriteTextBuiltinFont {
                items: vec![TextItem::Text(to_winlatin(marker))],
                font: BuiltinFont::Helvetica,
            });
            ops.push(Op::EndTextSection);
        }
    }

    // Image – embed from pre-registered XObject
    if let Some(img) = &lbox.image {
        if let Some(res) = images.get(&img.src) {
            // PDF origin is bottom-left; our layout origin is top-left.
            // translate_y = bottom edge of image in PDF coordinates.
            let img_bottom_y = page_height - lbox.y - img.height;

            // At dpi=72 printpdf renders 1 px = 1 pt, so
            // scale = desired_pt / px_dim.
            let scale_x = if res.px_width > 0 {
                img.width / res.px_width as f32
            } else {
                1.0
            };
            let scale_y = if res.px_height > 0 {
                img.height / res.px_height as f32
            } else {
                1.0
            };

            ops.push(Op::UseXobject {
                id: res.xobj_id.clone(),
                transform: XObjectTransform {
                    translate_x: Some(Pt(lbox.x)),
                    translate_y: Some(Pt(img_bottom_y)),
                    dpi: Some(72.0),
                    scale_x: Some(scale_x),
                    scale_y: Some(scale_y),
                    rotate: None,
                },
            });
        }
    }

    // Children
    for child in &lbox.children {
        render_box(ops, child, page_height, images);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_empty_page() {
        let config = LayoutConfig::a4();
        let bytes = render_pdf(&config).unwrap();
        assert!(bytes.len() > 100, "PDF should have content");
        // PDF magic number
        assert_eq!(&bytes[0..5], b"%PDF-");
    }
}

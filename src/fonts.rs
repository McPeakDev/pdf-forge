//! Font loading and text measurement using `ttf-parser`.
//!
//! For reproducibility we embed a default font (Liberation Sans) and measure
//! glyph advances to feed Taffy with accurate intrinsic sizes.

use std::collections::HashMap;

/// A loaded font face with metrics.
#[derive(Clone)]
pub struct FontData {
    /// Raw font bytes (kept alive for ttf-parser's zero-copy API).
    pub bytes: Vec<u8>,
    pub units_per_em: f32,
    pub ascender: f32,
    pub descender: f32,
    pub line_gap: f32,
}

/// Manages loaded fonts.
pub struct FontManager {
    fonts: HashMap<FontKey, FontData>,
    /// Fallback metrics if no font is loaded.
    default_key: FontKey,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FontKey {
    pub family: String,
    pub bold: bool,
    pub italic: bool,
}

impl FontManager {
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            default_key: FontKey {
                family: "Helvetica".to_string(),
                bold: false,
                italic: false,
            },
        }
    }

    /// Load a TTF/OTF font from bytes.
    pub fn load_font(&mut self, family: &str, bold: bool, italic: bool, bytes: Vec<u8>) -> Result<(), String> {
        let face = ttf_parser::Face::parse(&bytes, 0)
            .map_err(|e| format!("Failed to parse font: {e}"))?;

        let data = FontData {
            units_per_em: face.units_per_em() as f32,
            ascender: face.ascender() as f32,
            descender: face.descender() as f32,
            line_gap: face.line_gap() as f32,
            bytes,
        };

        let key = FontKey {
            family: family.to_string(),
            bold,
            italic,
        };

        if self.fonts.is_empty() {
            self.default_key = key.clone();
        }
        self.fonts.insert(key, data);
        Ok(())
    }

    /// Register a builtin font with synthetic metrics (for when no TTF is
    /// available). Uses Helvetica-like metrics.
    pub fn ensure_default(&mut self) {
        if self.fonts.is_empty() {
            let key = FontKey {
                family: "Helvetica".to_string(),
                bold: false,
                italic: false,
            };
            self.fonts.insert(
                key.clone(),
                FontData {
                    bytes: Vec::new(),
                    units_per_em: 1000.0,
                    ascender: 750.0,
                    descender: -250.0,
                    line_gap: 0.0,
                },
            );
            self.default_key = key;

            // Also register bold variant
            let bold_key = FontKey {
                family: "Helvetica".to_string(),
                bold: true,
                italic: false,
            };
            self.fonts.insert(
                bold_key,
                FontData {
                    bytes: Vec::new(),
                    units_per_em: 1000.0,
                    ascender: 750.0,
                    descender: -250.0,
                    line_gap: 0.0,
                },
            );
        }
    }

    /// Get font data for a key, falling back to the default.
    pub fn get(&self, key: &FontKey) -> &FontData {
        self.fonts.get(key).unwrap_or_else(|| {
            self.fonts.get(&self.default_key).expect("No fonts loaded")
        })
    }

    /// Measure the width of a string at a given font size (in px).
    /// If we have actual font bytes, we parse glyph advances. Otherwise we
    /// use an average character width heuristic (0.5 × font_size per char).
    pub fn measure_text_width(&self, text: &str, font_size: f32, bold: bool, italic: bool, family: &str) -> f32 {
        let key = FontKey {
            family: family.to_string(),
            bold,
            italic,
        };
        let data = self.get(&key);

        if data.bytes.is_empty() {
            // Heuristic: average char width ≈ 0.5 × font_size for proportional fonts.
            // Bold is ~10 % wider.
            let avg = if bold { 0.55 } else { 0.5 };
            return text.chars().count() as f32 * font_size * avg;
        }

        // Parse the font and sum horizontal advances
        if let Ok(face) = ttf_parser::Face::parse(&data.bytes, 0) {
            let scale = font_size / data.units_per_em;
            let mut width = 0.0f32;
            for ch in text.chars() {
                if let Some(gid) = face.glyph_index(ch) {
                    let advance = face.glyph_hor_advance(gid).unwrap_or(0);
                    width += advance as f32 * scale;
                } else {
                    // Fallback for missing glyph
                    width += font_size * 0.5;
                }
            }
            width
        } else {
            text.chars().count() as f32 * font_size * 0.5
        }
    }

    /// Measure the line height in px.
    pub fn line_height_px(&self, font_size: f32, line_height_factor: f32) -> f32 {
        font_size * line_height_factor
    }

    /// Get the ascender in px for the given font.
    pub fn ascender_px(&self, font_size: f32, bold: bool, italic: bool, family: &str) -> f32 {
        let key = FontKey {
            family: family.to_string(),
            bold,
            italic,
        };
        let data = self.get(&key);
        let scale = font_size / data.units_per_em;
        data.ascender * scale
    }

    /// Check if real font bytes are loaded for the default font.
    pub fn has_real_fonts(&self) -> bool {
        self.fonts
            .get(&self.default_key)
            .map(|d| !d.bytes.is_empty())
            .unwrap_or(false)
    }

    /// Get all loaded font keys.
    pub fn keys(&self) -> Vec<FontKey> {
        self.fonts.keys().cloned().collect()
    }

    /// Get font bytes for embedding in PDF.
    pub fn font_bytes(&self, key: &FontKey) -> Option<&[u8]> {
        self.fonts.get(key).and_then(|d| {
            if d.bytes.is_empty() {
                None
            } else {
                Some(d.bytes.as_slice())
            }
        })
    }
}

impl Default for FontManager {
    fn default() -> Self {
        let mut mgr = Self::new();
        mgr.ensure_default();
        mgr
    }
}

/// Word-wrap text to fit within `max_width` pixels. Returns a vec of lines.
pub fn wrap_text(
    text: &str,
    font_size: f32,
    bold: bool,
    italic: bool,
    family: &str,
    max_width: f32,
    fonts: &FontManager,
) -> Vec<String> {
    if max_width <= 0.0 || text.is_empty() {
        return vec![text.to_string()];
    }

    let mut lines: Vec<String> = Vec::new();
    // Split on existing newlines first
    for paragraph in text.split('\n') {
        let words: Vec<&str> = paragraph.split_whitespace().collect();
        if words.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current_line = String::new();
        for word in &words {
            let candidate = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };
            let w = fonts.measure_text_width(&candidate, font_size, bold, italic, family);
            if w > max_width && !current_line.is_empty() {
                lines.push(current_line);
                current_line = word.to_string();
            } else {
                current_line = candidate;
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heuristic_text_width() {
        let mgr = FontManager::default();
        let w = mgr.measure_text_width("Hello", 16.0, false, false, "Helvetica");
        // 5 chars × 16 × 0.5 = 40
        assert!((w - 40.0).abs() < 0.1);
    }

    #[test]
    fn word_wrap_basic() {
        let mgr = FontManager::default();
        let lines = wrap_text("Hello world foo bar", 16.0, false, false, "Helvetica", 60.0, &mgr);
        assert!(lines.len() >= 2, "Expected wrapping, got {:?}", lines);
    }
}

//! Rendu texte via la police bitmap 8x8 (font8x8, domaine public).

pub fn glyph(c: char) -> Option<[u8; 8]> {
    use font8x8::{BASIC_FONTS, UnicodeFonts};
    let c = if c.is_ascii() { c } else { '?' };
    BASIC_FONTS.get(c).map(|g| g)
}

pub const GLYPH_W: f32 = 8.0;
pub const GLYPH_H: f32 = 8.0;

pub fn text_width(s: &str, size: f32) -> f32 {
    let scale = size / GLYPH_H;
    let mut max_len = 0usize;
    for line in s.split('\n') {
        max_len = max_len.max(line.chars().count());
    }
    max_len as f32 * GLYPH_W * scale
}

pub fn text_height(s: &str, size: f32) -> f32 {
    let scale = size / GLYPH_H;
    s.split('\n').count() as f32 * GLYPH_H * scale
}

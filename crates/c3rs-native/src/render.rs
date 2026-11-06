//! Rendu logiciel : framebuffer ARGB, sprites avec rotation, tiling, texte.

use crate::model::Tex;
use crate::font;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Rgba {
        Rgba { r: r.clamp(0.0, 1.0), g: g.clamp(0.0, 1.0), b: b.clamp(0.0, 1.0), a: a.clamp(0.0, 1.0) }
    }
    pub fn hex(s: &str) -> Rgba {
        parse_hex_color(s).map(|c| Rgba::new(c[0], c[1], c[2], c[3])).unwrap_or(Rgba::new(1.0, 1.0, 1.0, 1.0))
    }
    pub fn to_u32(self) -> u32 {
        let r = (self.r * 255.0).round() as u32;
        let g = (self.g * 255.0).round() as u32;
        let b = (self.b * 255.0).round() as u32;
        (0xFF << 24) | (r << 16) | (g << 8) | b
    }
    pub fn from_u32(v: u32) -> Rgba {
        Rgba::new(
            ((v >> 16) & 0xFF) as f32 / 255.0,
            ((v >> 8) & 0xFF) as f32 / 255.0,
            (v & 0xFF) as f32 / 255.0,
            1.0,
        )
    }
}

pub fn parse_hex_color(s: &str) -> Option<[f32; 4]> {
    let s = s.trim().trim_start_matches('#');
    let cs: Vec<char> = s.chars().collect();
    if cs.len() < 6 {
        return None;
    }
    let hx = |cs: &[char]| -> Option<f32> {
        if cs.len() < 2 {
            return None;
        }
        let a = u8::from_str_radix(&cs[0].to_string(), 16).ok()?;
        let b = u8::from_str_radix(&cs[1].to_string(), 16).ok()?;
        Some((a * 16 + b) as f32 / 255.0)
    };
    let r = hx(&cs[0..2])?;
    let g = hx(&cs[2..4])?;
    let b = hx(&cs[4..6])?;
    let a = if cs.len() >= 8 { hx(&cs[6..8])? } else { 1.0 };
    Some([r, g, b, a])
}

#[derive(Clone)]
pub struct FrameBuf {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u32>,
}

impl FrameBuf {
    pub fn new(w: usize, h: usize) -> FrameBuf {
        FrameBuf { w, h, px: vec![0xFF000000; w * h] }
    }

    pub fn clear(&mut self, c: Rgba) {
        let v = c.to_u32();
        self.px.iter_mut().for_each(|p| *p = v);
    }

    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, c: Rgba) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let a = c.a;
        if a <= 0.0 {
            return;
        }
        let idx = (y as usize) * self.w + x as usize;
        if a >= 1.0 {
            self.px[idx] = c.to_u32();
            return;
        }
        let d = self.px[idx];
        let dr = ((d >> 16) & 0xFF) as f32 / 255.0;
        let dg = ((d >> 8) & 0xFF) as f32 / 255.0;
        let db = (d & 0xFF) as f32 / 255.0;
        let out = Rgba::new(
            c.r * a + dr * (1.0 - a),
            c.g * a + dg * (1.0 - a),
            c.b * a + db * (1.0 - a),
            1.0,
        );
        self.px[idx] = out.to_u32();
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Rgba) {
        if c.a <= 0.0 {
            return;
        }
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = (x + w).ceil() as i32;
        let y1 = (y + h).ceil() as i32;
        for yy in y0..y1 {
            for xx in x0..x1 {
                self.blend(xx, yy, c);
            }
        }
    }

    /// Blit d'une texture avec échelle + rotation autour d'un pivot (coordonnées texture normalisées ox,oy).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_tex(
        &mut self,
        tex: &Tex,
        dx: f32,
        dy: f32,
        dw: f32,
        dh: f32,
        angle_deg: f32,
        ox: f32,
        oy: f32,
        opacity: f32,
        tint: Option<Rgba>,
        mirror: bool,
    ) {
        if opacity <= 0.001 || dw <= 0.0 || dh <= 0.0 {
            return;
        }
        let rad = angle_deg.to_radians();
        let (sin, cos) = rad.sin_cos();
        // pivot en coordonnées écran
        let pivot_x = dx + ox * dw;
        let pivot_y = dy + oy * dh;
        // bbox écran du rectangle tourné
        let corners = [
            (dx, dy),
            (dx + dw, dy),
            (dx + dw, dy + dh),
            (dx, dy + dh),
        ];
        let mut minx = f32::INFINITY;
        let mut miny = f32::INFINITY;
        let mut maxx = f32::NEG_INFINITY;
        let mut maxy = f32::NEG_INFINITY;
        for (cx, cy) in corners {
            let rx = cx - dx - ox * dw;
            let ry = cy - dy - oy * dh;
            let sx = pivot_x + rx * cos - ry * sin;
            let sy = pivot_y + rx * sin + ry * cos;
            minx = minx.min(sx);
            miny = miny.min(sy);
            maxx = maxx.max(sx);
            maxy = maxy.max(sy);
        }
        for py in miny.floor() as i32..maxy.ceil() as i32 {
            for px in minx.floor() as i32..maxx.ceil() as i32 {
                // inverse : écran -> rectangle non tourné autour du pivot
                let rx = px as f32 - pivot_x;
                let ry = py as f32 - pivot_y;
                let mut ux = rx * cos + ry * sin + ox * dw;
                let uy = ry * cos - rx * sin + oy * dh;
                if mirror {
                    ux = dw - ux;
                }
                if ux < 0.0 || uy < 0.0 || ux >= dw || uy >= dh {
                    continue;
                }
                let sx = ((ux / dw) * tex.w as f32) as u32;
                let sy = ((uy / dh) * tex.h as f32) as u32;
                let idx = (sy * tex.w + sx) as usize * 4;
                let tr = tex.rgba[idx] as f32 / 255.0;
                let tg = tex.rgba[idx + 1] as f32 / 255.0;
                let tb = tex.rgba[idx + 2] as f32 / 255.0;
                let ta = tex.rgba[idx + 3] as f32 / 255.0 * opacity;
                let (fr, fg, fb) = match tint {
                    Some(t) if t.r != 1.0 || t.g != 1.0 || t.b != 1.0 => (tr * t.r, tg * t.g, tb * t.b),
                    _ => (tr, tg, tb),
                };
                self.blend(px, py, Rgba::new(fr, fg, fb, ta));
            }
        }
    }

    /// Tiling d'une texture sur une zone (Tiled Background).
    pub fn tile_tex(&mut self, tex: &Tex, x: f32, y: f32, w: f32, h: f32, opacity: f32) {
        if opacity <= 0.001 {
            return;
        }
        let x0 = x.max(0.0) as i32;
        let y0 = y.max(0.0) as i32;
        let x1 = (x + w).min(self.w as f32) as i32;
        let y1 = (y + h).min(self.h as f32) as i32;
        for py in y0..y1 {
            for px in x0..x1 {
                let sx = ((px as f32 - x) % tex.w as f32) as u32;
                let sy = ((py as f32 - y) % tex.h as f32) as u32;
                let idx = (sy * tex.w + sx) as usize * 4;
                let ta = tex.rgba[idx + 3] as f32 / 255.0 * opacity;
                self.blend(
                    px,
                    py,
                    Rgba::new(
                        tex.rgba[idx] as f32 / 255.0,
                        tex.rgba[idx + 1] as f32 / 255.0,
                        tex.rgba[idx + 2] as f32 / 255.0,
                        ta,
                    ),
                );
            }
        }
    }

    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Rgba, opacity: f32) {
        let scale = (size / font::GLYPH_H).max(0.05);
        let color = Rgba::new(color.r, color.g, color.b, color.a * opacity);
        let mut cy = y;
        for line in text.split('\n') {
            let mut cx = x;
            for ch in line.chars() {
                if let Some(g) = font::glyph(ch) {
                    for (row, byte) in g.iter().enumerate() {
                        for col in 0..8 {
                            if byte & (1 << col) != 0 {
                                let x0 = cx + col as f32 * scale;
                                let y0 = cy + row as f32 * scale;
                                let x1 = x0 + scale;
                                let y1 = y0 + scale;
                                let px0 = x0.floor() as i32;
                                let py0 = y0.floor() as i32;
                                let px1 = x1.ceil() as i32;
                                let py1 = y1.ceil() as i32;
                                for py in py0..py1 {
                                    for px in px0..px1 {
                                        self.blend(px, py, color);
                                    }
                                }
                            }
                        }
                    }
                }
                cx += font::GLYPH_W * scale;
            }
            cy += font::GLYPH_H * scale;
        }
    }
}

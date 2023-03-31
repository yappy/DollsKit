use std::io::Cursor;

use anyhow::{Context, Result};
use image::{DynamicImage, ImageOutputFormat, Rgba};
use rusttype::{point, Font, PositionedGlyph, Scale};

struct FontRenderer {
    font: Font<'static>,
}

impl FontRenderer {
    pub fn new(ttf_bin: Vec<u8>) -> Result<Self> {
        // ロードエラー時は None が返るので anyhow::Error に変換する
        let font = Font::try_from_vec(ttf_bin).context("Invalid font data")?;

        Ok(Self { font })
    }

    pub fn draw_multiline_text(&self, text: &str, scale: u32, width: u32) -> Result<Vec<u8>> {
        let scale = Scale::uniform(scale as f32);
        let vmet = self.font.v_metrics(scale);
        let glyphs_h = (vmet.ascent - vmet.descent).ceil() as u32;

        let mut lines = Vec::new();
        // 行で分解
        for orig_line in text.split('\n') {
            // この行をさらに分解する
            // 残り: orig_line[start..len]
            let mut start = 0_usize;
            // 空行
            if orig_line.is_empty() {
                lines.push(Vec::new());
            }
            while start < orig_line.len() {
                // orig_line[start..len] の先頭部分文字列で
                // 横幅が width を超えない最大のインデックスを求める
                let rest = &orig_line[start..orig_line.len()];
                // unicode 文字の開始位置
                let idxlist: Vec<_> = rest.char_indices().map(|(i, _c)| i).collect();

                let mut le = 0_usize;
                let mut ri = idxlist.len() + 1;
                while le < ri {
                    let m = le + (ri - le) / 2;
                    let m_idx = *idxlist.get(m).unwrap_or(&rest.len());
                    let (glyphs_w, _) = self.calc_line_w(
                        &rest[0..m_idx],
                        scale,
                        (glyphs_h * lines.len() as u32) as f32,
                        vmet.ascent,
                    );
                    if glyphs_w <= width {
                        le = m + 1;
                    } else {
                        ri = m;
                    }
                }
                // le = ri = 初めて横幅がwidthを超えるidxlistのインデックス
                let le_idx = *idxlist.get(le.saturating_sub(1)).unwrap_or(&rest.len());
                let (_w, glyphs) = self.calc_line_w(
                    &rest[0..le_idx],
                    scale,
                    (glyphs_h * lines.len() as u32) as f32,
                    vmet.ascent,
                );
                lines.push(glyphs);

                start += le_idx;
            }
        }

        let mut image = DynamicImage::new_rgba8(width, glyphs_h * lines.len() as u32).to_rgba8();
        for (_x, _y, pixel) in image.enumerate_pixels_mut() {
            *pixel = Rgba([0, 0, 0, 255 as u8]);
        }

        for glyphs in lines {
            for glyph in glyphs {
                if let Some(bounding_box) = glyph.pixel_bounding_box() {
                    // Draw the glyph into the image per-pixel by using the draw closure
                    glyph.draw(|x, y, v| {
                        let x = x + bounding_box.min.x as u32;
                        let y = y + bounding_box.min.y as u32;
                        let [r1, g1, b1, _a1] = image.get_pixel(x, y).0;
                        let [r2, g2, b2] = [255, 255, 255u8];
                        let r3 = (r1 as f32 * (1.0 - v) + r2 as f32 * v) as u8;
                        let g3 = (g1 as f32 * (1.0 - v) + g2 as f32 * v) as u8;
                        let b3 = (b1 as f32 * (1.0 - v) + b2 as f32 * v) as u8;
                        image.put_pixel(
                            // Offset the position by the glyph bounding box
                            x,
                            y,
                            Rgba([r3, g3, b3, 255]),
                        )
                    });
                }
            }
        }

        // Save the image to a png file
        let mut filebuf = Vec::new();
        image.write_to(&mut Cursor::new(&mut filebuf), ImageOutputFormat::Png)?;

        Ok(filebuf)
    }

    fn calc_line_w(
        &self,
        text: &str,
        scale: Scale,
        y: f32,
        height: f32,
    ) -> (u32, Vec<PositionedGlyph>) {
        let glyphs: Vec<_> = self
            .font
            .layout(text, scale, point(0.0, y + height))
            .collect();

        let min_x = glyphs
            .first()
            .map(|g| g.pixel_bounding_box().unwrap().min.x)
            .unwrap();
        let max_x = glyphs
            .last()
            .map(|g| g.pixel_bounding_box().unwrap().max.x)
            .unwrap();

        ((max_x - min_x) as u32, glyphs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    #[ignore]
    // sudo apt install fonts-ipafont
    // cargo test font -- --ignored
    fn font() -> Result<()> {
        let fname = "/usr/share/fonts/truetype/fonts-japanese-gothic.ttf";
        println!("Load font file from: {fname}");
        let ttf_bin = fs::read(fname)?;

        let r = FontRenderer::new(ttf_bin)?;

        let png = r.draw_multiline_text(
            "こんにちは。私はyappy家の管理人形です。\n\nくれぐれもよろしくお願いします。\n",
            32,
            320,
        )?;

        let fname = "font_test.png";
        println!("Write image to: {fname}");
        fs::write(fname, png)?;

        Ok(())
    }
}

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

    pub fn draw_multiline_text(
        &self,
        fgcolor: (u8, u8, u8),
        bgcolor: (u8, u8, u8),
        text: &str,
        scale: u32,
        width: u32,
    ) -> Result<Vec<u8>> {
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

        let height = glyphs_h * lines.len() as u32;
        let mut image = DynamicImage::new_rgba8(width, height).to_rgba8();
        for (_x, _y, pixel) in image.enumerate_pixels_mut() {
            *pixel = Rgba([bgcolor.0, bgcolor.1, bgcolor.2, 255_u8]);
        }

        for glyphs in lines {
            for glyph in glyphs {
                if let Some(bounding_box) = glyph.pixel_bounding_box() {
                    // Draw the glyph into the image per-pixel by using the draw closure
                    glyph.draw(|x, y, v| {
                        let x = x + bounding_box.min.x as u32;
                        let y = y + bounding_box.min.y as u32;
                        if x >= width || y >= height {
                            return;
                        }
                        let [r1, g1, b1, _a1] = image.get_pixel(x, y).0;
                        let [r2, g2, b2] = [fgcolor.0, fgcolor.1, fgcolor.2];
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
        if text.is_empty() {
            return (0, Vec::new());
        }

        let glyphs: Vec<_> = self
            .font
            .layout(text, scale, point(0.0, y + height))
            .collect();

        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        for g in glyphs.iter() {
            let (le, ri) = match g.pixel_bounding_box() {
                Some(bb) => (bb.min.x, bb.max.x),
                None => (g.position().x as i32, g.position().x as i32),
            };
            min_x = min_x.min(le);
            max_x = max_x.max(ri);
        }

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
        let text =
"役割論理（Role logic）は、形式言語の一種であり、論理学やコンピュータ科学の分野で用いられる言語です。役割論理では、対象の役割に着目し、役割に関する命題を表現します。役割論理には、役割論理の基本的な概念である「役割」、「役割変数」、「役割制約」、「役割コンストラクタ」などがあります。
役割論理では、対象の役割を表現するために、役割変数を用います。役割変数は、役割を表す変数であり、通常はRやSなどのアルファベットで表されます。役割制約は、役割変数が取りうる値に制限を加える制約のことです。たとえば、「人間は動物である」という役割制約は、「人間」の役割変数に対して、「動物」の値を許容することを意味します。
役割コンストラクタは、複数の役割を組み合わせて新しい役割を構成するための方法です。たとえば、「親子関係」という役割は、「親」と「子」の役割を組み合わせて構成されます。役割コンストラクタには、合成（composition）や逆（inverse）などがあります。
役割論理は、主に知識表現や推論に用いられます。例えば、役割論理を用いて、複数のエージェントの間で共有される知識を表現することができます。また、役割論理は、オントロジー言語の一種であるOWL（Web Ontology Language）の基盤としても用いられています。";

        let png = r.draw_multiline_text((0xff, 0xff, 0xff), (0x80, 0x00, 0x80), text, 16, 640)?;

        let fname = "font_test.png";
        println!("Write image to: {fname}");
        fs::write(fname, png)?;

        Ok(())
    }
}

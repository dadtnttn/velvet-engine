use super::canvas::Canvas;
use fontdue::Font;

impl Canvas {
    pub fn text_line(
        &mut self,
        font: &Font,
        text: &str,
        x: i32,
        baseline: i32,
        size: f32,
        color: u32,
    ) -> i32 {
        let mut pen = x as f32;
        for ch in text.chars() {
            let (metrics, bitmap) = font.rasterize(ch, size);
            let gx = pen as i32 + metrics.xmin;
            let gy = baseline - metrics.height as i32 - metrics.ymin;
            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let alpha = bitmap[row * metrics.width + col] as u32;
                    if alpha == 0 {
                        continue;
                    }
                    self.put(
                        gx + col as i32,
                        gy + row as i32,
                        (alpha << 24) | (color & 0x00ff_ffff),
                        true,
                    );
                }
            }
            pen += metrics.advance_width;
        }
        pen.round() as i32
    }
    pub fn measure_text(font: &Font, text: &str, size: f32) -> f32 {
        text.chars()
            .map(|ch| font.metrics(ch, size).advance_width)
            .sum()
    }
    #[allow(clippy::too_many_arguments)]
    pub fn text_wrapped(
        &mut self,
        font: &Font,
        text: &str,
        x: i32,
        y: i32,
        max_width: i32,
        size: f32,
        line_height: i32,
        color: u32,
    ) -> i32 {
        let mut cy = y;
        for paragraph in text.split('\n') {
            let mut line = String::new();
            for word in paragraph.split_whitespace() {
                let candidate = if line.is_empty() {
                    word.to_owned()
                } else {
                    format!("{line} {word}")
                };
                if !line.is_empty() && Self::measure_text(font, &candidate, size) > max_width as f32
                {
                    self.text_line(font, &line, x, cy + size as i32, size, color);
                    cy += line_height;
                    line = word.to_owned();
                } else {
                    line = candidate;
                }
            }
            if !line.is_empty() {
                self.text_line(font, &line, x, cy + size as i32, size, color);
                cy += line_height;
            }
        }
        cy
    }
    pub fn centered_text(
        &mut self,
        font: &Font,
        text: &str,
        cx: i32,
        baseline: i32,
        size: f32,
        color: u32,
    ) {
        let w = Self::measure_text(font, text, size).round() as i32;
        self.text_line(font, text, cx - w / 2, baseline, size, color);
    }
}

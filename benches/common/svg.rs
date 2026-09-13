//! An SVG writer over malevich's cell raster: one `<text>` run per stretch of
//! same-colored glyphs on a fixed grid, so colors survive where a terminal
//! escape stream cannot, such as a markdown file rendered by GitHub.

use malevich::{Charset, Color, ColorMode, Frame, Plot, Theme};

const CELL_W: f64 = 7.8;
const CELL_H: f64 = 16.0;
const FONT_PX: f64 = 13.0;
const BACKGROUND: &str = "#ffffff";
const FOREGROUND: &str = "#1f2328";

/// A true-color frame on the light theme, sized in cells.
pub fn frame(width: usize, height: usize) -> Frame {
    Frame {
        width,
        height,
        charset: Charset::Quadrants,
        color: ColorMode::TrueColor,
        theme: Theme::LIGHT,
    }
}

/// Renders `plot` into a `width` by `height` cell grid and writes it as SVG.
pub fn render(plot: &Plot<'_>, width: usize, height: usize) -> String {
    let raster = plot.raster(&frame(width, height));
    let (cols, rows) = (raster.width(), raster.height());
    let (w, h) = (cols as f64 * CELL_W, rows as f64 * CELL_H);
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w:.1} {h:.1}\" width=\"{w:.0}\" height=\"{h:.0}\" \
         font-family=\"ui-monospace, Menlo, 'DejaVu Sans Mono', Consolas, monospace\" font-size=\"{FONT_PX}\" shape-rendering=\"crispEdges\">\n"
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{BACKGROUND}\"/>\n"
    ));

    for row in 0..rows {
        let y = row as f64 * CELL_H;
        // Cell backgrounds first, so text paints over them.
        for col in 0..cols {
            let cell = raster.cell(col, row).unwrap();
            if cell.columns == 0 || cell.background == Color::Default {
                continue;
            }
            out.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{CELL_H}\" fill=\"{}\"/>\n",
                col as f64 * CELL_W,
                cell.columns as f64 * CELL_W,
                hex(cell.background)
            ));
        }
        // Block elements become rectangles: crisp bars in any viewer, no
        // dependence on the font's line gap. Runs of the same full-width
        // glyph in the same color merge into one rectangle.
        let mut col = 0;
        while col < cols {
            let cell = raster.cell(col, row).unwrap();
            let Some(boxes) = (cell.columns > 0)
                .then(|| block_boxes(cell.glyph))
                .flatten()
            else {
                col += 1;
                continue;
            };
            let full_width = boxes.iter().all(|&(bx, _, bw, _)| bx == 0.0 && bw == 1.0);
            let mut run = 1;
            if full_width {
                while col + run < cols {
                    let next = raster.cell(col + run, row).unwrap();
                    if next.glyph != cell.glyph || next.foreground != cell.foreground {
                        break;
                    }
                    run += 1;
                }
            }
            let (x0, y0) = (col as f64 * CELL_W, y);
            for (bx, by, bw, bh) in boxes {
                out.push_str(&format!(
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"/>\n",
                    x0 + bx * CELL_W,
                    y0 + by * CELL_H,
                    bw * CELL_W * run as f64,
                    bh * CELL_H,
                    hex(cell.foreground)
                ));
            }
            col += run;
        }
        // Then runs of remaining glyphs sharing a foreground.
        let mut col = 0;
        while col < cols {
            let cell = raster.cell(col, row).unwrap();
            if cell.columns == 0 || cell.glyph == ' ' || block_boxes(cell.glyph).is_some() {
                col += 1;
                continue;
            }
            let start = col;
            let color = cell.foreground;
            let mut text = String::new();
            let mut span = 0usize;
            while col < cols {
                let c = raster.cell(col, row).unwrap();
                if c.columns == 0 {
                    col += 1;
                    continue;
                }
                if c.foreground != color || block_boxes(c.glyph).is_some() {
                    break;
                }
                // Keep interior spaces so the run stays one text element.
                text.push(c.glyph);
                span += c.columns as usize;
                col += 1;
            }
            let trimmed_len = text.trim_end().chars().count();
            let trailing = text.chars().count() - trimmed_len;
            let text: String = text.trim_end().to_string();
            let span = span - trailing;
            out.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" fill=\"{}\" textLength=\"{:.1}\" lengthAdjust=\"spacingAndGlyphs\" xml:space=\"preserve\">{}</text>\n",
                start as f64 * CELL_W,
                y + CELL_H * 0.78,
                hex(color),
                span as f64 * CELL_W,
                escape(&text)
            ));
        }
    }
    out.push_str("</svg>\n");
    out
}

/// Filled boxes for a Block Elements glyph as `(x, y, w, h)` fractions of
/// the cell, or `None` for anything that should stay text.
fn block_boxes(glyph: char) -> Option<Vec<(f64, f64, f64, f64)>> {
    let bottom = |eighths: f64| vec![(0.0, 1.0 - eighths / 8.0, 1.0, eighths / 8.0)];
    let left = |eighths: f64| vec![(0.0, 0.0, eighths / 8.0, 1.0)];
    let (tl, tr, bl, br) = (
        (0.0, 0.0, 0.5, 0.5),
        (0.5, 0.0, 0.5, 0.5),
        (0.0, 0.5, 0.5, 0.5),
        (0.5, 0.5, 0.5, 0.5),
    );
    Some(match glyph {
        '\u{2588}' => vec![(0.0, 0.0, 1.0, 1.0)],
        '\u{2587}' => bottom(7.0),
        '\u{2586}' => bottom(6.0),
        '\u{2585}' => bottom(5.0),
        '\u{2584}' => bottom(4.0),
        '\u{2583}' => bottom(3.0),
        '\u{2582}' => bottom(2.0),
        '\u{2581}' => bottom(1.0),
        '\u{2580}' => vec![(0.0, 0.0, 1.0, 0.5)],
        '\u{2594}' => vec![(0.0, 0.0, 1.0, 0.125)],
        '\u{2589}' => left(7.0),
        '\u{258A}' => left(6.0),
        '\u{258B}' => left(5.0),
        '\u{258C}' => left(4.0),
        '\u{258D}' => left(3.0),
        '\u{258E}' => left(2.0),
        '\u{258F}' => left(1.0),
        '\u{2590}' => vec![(0.5, 0.0, 0.5, 1.0)],
        '\u{2595}' => vec![(0.875, 0.0, 0.125, 1.0)],
        '\u{2598}' => vec![tl],
        '\u{259D}' => vec![tr],
        '\u{2596}' => vec![bl],
        '\u{2597}' => vec![br],
        '\u{259A}' => vec![tl, br],
        '\u{259E}' => vec![tr, bl],
        '\u{259B}' => vec![tl, tr, bl],
        '\u{259C}' => vec![tl, tr, br],
        '\u{2599}' => vec![tl, bl, br],
        '\u{259F}' => vec![tr, bl, br],
        _ => return None,
    })
}

fn escape(text: &str) -> String {
    let mut s = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => s.push_str("&amp;"),
            '<' => s.push_str("&lt;"),
            '>' => s.push_str("&gt;"),
            c => s.push(c),
        }
    }
    s
}

/// GitHub's light-mode colors for the ANSI names, xterm's cube for the rest.
fn hex(color: Color) -> String {
    let (r, g, b) = match color {
        Color::Default | Color::Black => return FOREGROUND.into(),
        Color::Red => (0xcf, 0x22, 0x2e),
        Color::Green => (0x1a, 0x7f, 0x37),
        Color::Yellow => (0x9a, 0x67, 0x00),
        Color::Blue => (0x09, 0x69, 0xda),
        Color::Magenta => (0x82, 0x50, 0xdf),
        Color::Cyan => (0x1b, 0x7c, 0x83),
        Color::White => (0xff, 0xff, 0xff),
        Color::BrightBlack => (0x57, 0x60, 0x6a),
        Color::BrightRed => (0xff, 0x81, 0x82),
        Color::BrightGreen => (0x4a, 0xc2, 0x6b),
        Color::BrightYellow => (0xd4, 0xa7, 0x2c),
        Color::BrightBlue => (0x54, 0xae, 0xff),
        Color::BrightMagenta => (0xc2, 0x97, 0xff),
        Color::BrightCyan => (0x76, 0xe3, 0xea),
        Color::BrightWhite => (0xf6, 0xf8, 0xfa),
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Ansi256(i) => ansi256(i),
    };
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn ansi256(i: u8) -> (u8, u8, u8) {
    match i {
        0..=15 => {
            let named = [
                Color::Black,
                Color::Red,
                Color::Green,
                Color::Yellow,
                Color::Blue,
                Color::Magenta,
                Color::Cyan,
                Color::White,
                Color::BrightBlack,
                Color::BrightRed,
                Color::BrightGreen,
                Color::BrightYellow,
                Color::BrightBlue,
                Color::BrightMagenta,
                Color::BrightCyan,
                Color::BrightWhite,
            ][i as usize];
            let h = hex(named);
            let v = |k: usize| u8::from_str_radix(&h[k..k + 2], 16).unwrap();
            (v(1), v(3), v(5))
        }
        16..=231 => {
            let n = i - 16;
            let step = |k: u8| if k == 0 { 0 } else { 55 + 40 * k };
            (step(n / 36), step(n / 6 % 6), step(n % 6))
        }
        _ => {
            let v = 8 + 10 * (i - 232);
            (v, v, v)
        }
    }
}

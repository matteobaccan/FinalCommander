//! The cell buffer as an SVG.
//!
//! So that a screenshot in the documentation is drawn *by the renderer*, from the same buffer
//! the terminal would be given, rather than photographed from somebody's terminal with
//! somebody's font and somebody's colour scheme. A picture made this way cannot drift from the
//! program: it is regenerated from a test, and if the panel changes the picture changes with
//! it.
//!
//! Every character is given its own `x`, rather than a run being given a width to stretch to.
//! Two things make that necessary and neither is obvious until the picture is looked at: SVG
//! collapses runs of whitespace in text content unless told not to, which eats the spaces a
//! panel's columns are made of; and `textLength` with `lengthAdjust="spacingAndGlyphs"` makes
//! up the difference by stretching the glyphs themselves, which turns a box-drawing character
//! into a smear. With one `x` per character and `xml:space="preserve"` the grid is exact in
//! any viewer, whatever font it ends up using — and the font named here will often not be the
//! one it uses.

use crate::buffer::Buffer;
use crate::color::{index_to_rgb, Color};

/// One cell, in pixels, at the font size below. The ratio is a monospace font's own.
const CELL_W: f32 = 8.4;
const CELL_H: f32 = 18.0;
const FONT_SIZE: f32 = 14.0;
/// Where the baseline sits inside the cell.
const BASELINE: f32 = 13.6;

/// What a `Color::Default` becomes when there is no terminal to ask.
const DEFAULT_FG: (u8, u8, u8) = (0xcc, 0xcc, 0xcc);
const DEFAULT_BG: (u8, u8, u8) = (0x00, 0x00, 0x00);

/// Render `buf` as a standalone SVG document.
pub fn to_svg(buf: &Buffer) -> String {
    let (w, h) = (buf.width(), buf.height());
    let (pw, ph) = (w as f32 * CELL_W, h as f32 * CELL_H);
    let mut out = String::with_capacity(w as usize * h as usize * 24);
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {pw:.0} {ph:.0}\" \
         width=\"{pw:.0}\" height=\"{ph:.0}\" font-family=\"ui-monospace, 'DejaVu Sans Mono', \
         'Cascadia Mono', Menlo, Consolas, monospace\" font-size=\"{FONT_SIZE}\">\n"
    ));
    // The background of the whole picture, so no gap shows through between the runs — and the
    // *background* default, not the foreground one, which is a mistake that paints the whole
    // picture light grey and is obvious only when looking at it.
    out.push_str(&format!(
        "<rect width=\"{pw:.0}\" height=\"{ph:.0}\" fill=\"{}\"/>\n",
        hex(Color::Default, true)
    ));

    for y in 0..h {
        // Backgrounds first, as runs: one rectangle per stretch of the same colour rather than
        // one per cell, which for a full screen is a few dozen instead of three thousand.
        let mut x = 0;
        while x < w {
            let bg = cell_bg(buf, x, y);
            let mut run = 1;
            while x + run < w && cell_bg(buf, x + run, y) == bg {
                run += 1;
            }
            if bg != hex(Color::Default, true) {
                out.push_str(&format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{CELL_H}\" \
                     fill=\"{bg}\"/>\n",
                    x as f32 * CELL_W,
                    y as f32 * CELL_H,
                    run as f32 * CELL_W
                ));
            }
            x += run;
        }

        // Then the text, as runs of one colour and weight, each character carrying its own
        // `x`. Leading and trailing spaces are dropped from a run: they draw nothing, and a
        // full screen of them is most of the file.
        let mut x = 0;
        while x < w {
            let Some(cell) = buf.get(x, y) else {
                x += 1;
                continue;
            };
            if cell.wide_tail {
                x += 1;
                continue;
            }
            let fg = cell_fg(buf, x, y);
            let bold = cell.style.bold;
            let mut chars: Vec<(f32, char)> = Vec::new();
            while x < w {
                let Some(cell) = buf.get(x, y) else { break };
                if cell.wide_tail {
                    x += 1;
                    continue;
                }
                if cell_fg(buf, x, y) != fg || cell.style.bold != bold {
                    break;
                }
                chars.push((x as f32 * CELL_W, cell.ch));
                x += 1;
            }
            let first = chars.iter().position(|(_, c)| *c != ' ');
            let last = chars.iter().rposition(|(_, c)| *c != ' ');
            let (Some(first), Some(last)) = (first, last) else {
                continue;
            };
            let chars = &chars[first..=last];
            out.push_str(&format!(
                "<text xml:space=\"preserve\" x=\"{}\" y=\"{:.1}\" fill=\"{fg}\"{}>{}</text>\n",
                chars
                    .iter()
                    .map(|(x, _)| format!("{x:.1}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                y as f32 * CELL_H + BASELINE,
                if bold { " font-weight=\"bold\"" } else { "" },
                escape(&chars.iter().map(|(_, c)| *c).collect::<String>())
            ));
        }
    }
    out.push_str("</svg>\n");
    out
}

/// The colours a cell is actually drawn in, `reverse` applied — which is how the active
/// panel's title and FAR's selected rows are drawn, so a picture that ignored it would be
/// wrong in exactly the places that matter.
fn cell_fg(buf: &Buffer, x: u16, y: u16) -> String {
    match buf.get(x, y) {
        Some(cell) if cell.style.reverse => hex(cell.style.bg, true),
        Some(cell) => hex(cell.style.fg, false),
        None => hex(Color::Default, false),
    }
}

fn cell_bg(buf: &Buffer, x: u16, y: u16) -> String {
    match buf.get(x, y) {
        Some(cell) if cell.style.reverse => hex(cell.style.fg, false),
        Some(cell) => hex(cell.style.bg, true),
        None => hex(Color::Default, true),
    }
}

fn hex(color: Color, background: bool) -> String {
    let (r, g, b) = match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Ansi(n) | Color::Indexed(n) => index_to_rgb(n),
        Color::Default if background => DEFAULT_BG,
        Color::Default => DEFAULT_FG,
    };
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            // A control character has no glyph and no business in the file.
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Style;

    #[test]
    fn the_text_and_the_colours_are_in_the_picture() {
        let mut buf = Buffer::new(10, 2);
        let style = Style::new(Color::Rgb(0, 0, 0), Color::Rgb(0, 128, 128));
        buf.put_str(0, 0, "hello & <>", style, 10);
        let svg = to_svg(&buf);
        assert!(
            svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""),
            "{svg}"
        );
        assert!(svg.ends_with("</svg>\n"));
        // The text, with its entities escaped so the document stays well formed.
        assert!(svg.contains("hello &amp; &lt;&gt;"), "{svg}");
        // The colours, as the theme gave them.
        assert!(svg.contains("fill=\"#008080\""), "{svg}");
        assert!(svg.contains("fill=\"#000000\""), "{svg}");
        // The grid: one `x` per character, so nothing depends on the viewer's font, and the
        // spaces inside the run are kept rather than collapsed by the XML reader.
        assert!(svg.contains("xml:space=\"preserve\""), "{svg}");
        assert!(
            svg.contains("x=\"0.0 8.4 16.8 25.2 33.6 42.0 50.4 58.8 67.2 75.6\""),
            "{svg}"
        );
        assert!(!svg.contains("textLength"), "{svg}");
    }

    /// `reverse` is how the active panel's title and a selected row are drawn, so it has to
    /// reach the picture rather than being dropped on the way.
    #[test]
    fn a_reversed_cell_swaps_its_colours() {
        let mut buf = Buffer::new(4, 1);
        let mut style = Style::new(Color::Rgb(0xff, 0xff, 0x55), Color::Rgb(0, 0, 0xaa));
        style.reverse = true;
        buf.put_str(0, 0, "far", style, 4);
        let svg = to_svg(&buf);
        // The background is the foreground colour and the text the background one.
        assert!(
            svg.contains(
                "<rect x=\"0.0\" y=\"0.0\" width=\"25.2\" height=\"18\" fill=\"#ffff55\"/>"
            ),
            "{svg}"
        );
        assert!(svg.contains("fill=\"#0000aa\""), "{svg}");
    }

    #[test]
    fn an_empty_buffer_is_a_valid_document() {
        let svg = to_svg(&Buffer::new(2, 1));
        assert!(svg.contains("</svg>"));
        assert!(!svg.contains("<text"), "nothing to write: {svg}");
        // And its one rectangle is the background colour, not the text colour.
        assert!(svg.contains("fill=\"#000000\"/>"), "{svg}");
        assert!(!svg.contains("fill=\"#cccccc\"/>"), "{svg}");
    }
}

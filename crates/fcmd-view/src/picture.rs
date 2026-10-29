//! Pictures, drawn in half blocks (SPEC 05 §1.14).
//!
//! A terminal cell is about twice as tall as it is wide, so a cell split across the middle
//! with `▀` — the top half painted in the foreground colour and the bottom in the background
//! — is two pixels that are each very nearly square. That is the whole trick: a window of
//! `cols × rows` cells is a canvas of `cols × 2·rows` pixels, and a picture scaled into it
//! and written one block per cell is a picture, at half the vertical resolution one would
//! guess from the row count and none of the horizontal.
//!
//! It works in every terminal there is, which is the reason to do it this way first. The
//! graphics protocols — kitty's, iTerm2's, sixel — put real pixels on the screen and are
//! worth having, but each works in some terminals and not others, and a picture one cannot
//! see at all is worse than one made of blocks.

use std::io::Cursor;

/// What a decoded picture is while it is being looked at: straight RGBA, no stride tricks,
/// because every use of it walks the whole thing anyway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    /// What the file was: `PNG`, `JPEG`, `GIF`, `BMP`. For the status line, which is the one
    /// place a reader can tell a `.png` that is really a JPEG from one that is not.
    pub format: &'static str,
    /// Row-major RGBA, four bytes a pixel.
    pixels: Vec<u8>,
}

/// Above this a picture is not decoded. Fifty megapixels is two hundred megabytes of RGBA,
/// which is already more than a terminal has any use for: what is drawn is at most a few
/// thousand blocks, and everything above that was read only to be averaged away.
pub const MAX_PIXELS: u64 = 50_000_000;

/// Is this a file the picture view can read? Decided on the first bytes and never on the
/// name, because a name can lie and a magic number cannot.
pub fn is_picture(head: &[u8]) -> bool {
    format_of(head).is_some()
}

/// What the first bytes say the file is.
fn format_of(head: &[u8]) -> Option<&'static str> {
    match head {
        [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, ..] => Some("PNG"),
        [0xff, 0xd8, 0xff, ..] => Some("JPEG"),
        [b'G', b'I', b'F', b'8', b'7' | b'9', b'a', ..] => Some("GIF"),
        [b'B', b'M', ..] => Some("BMP"),
        _ => None,
    }
}

/// Decode a whole file into a picture.
///
/// The error is a sentence rather than a type: there is one caller, it puts the words on the
/// status line, and a reader who opened a damaged JPEG wants to be told that and not handed
/// a hex dump without a word.
pub fn read(bytes: &[u8]) -> Result<Picture, String> {
    let format = format_of(bytes).ok_or("not a picture this can read")?;
    let mut reader = image::ImageReader::new(Cursor::new(bytes));
    reader.set_format(match format {
        "PNG" => image::ImageFormat::Png,
        "JPEG" => image::ImageFormat::Jpeg,
        "GIF" => image::ImageFormat::Gif,
        _ => image::ImageFormat::Bmp,
    });
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(u32::MAX);
    limits.max_image_height = Some(u32::MAX);
    limits.max_alloc = Some(MAX_PIXELS * 4);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|e| e.to_string())?;
    let (width, height) = (decoded.width(), decoded.height());
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(format!(
            "{width}×{height} is more picture than this can hold"
        ));
    }
    Ok(Picture {
        width,
        height,
        format,
        pixels: decoded.into_rgba8().into_raw(),
    })
}

/// One cell of the drawing: the colour of its top half and of its bottom half, each already
/// composited onto the background it is drawn over.
pub type Cell = ([u8; 3], [u8; 3]);

impl Picture {
    /// How many cells the picture takes inside a window of `cols × rows`, keeping its shape.
    ///
    /// Two half-pixels to a cell, so the canvas is twice as tall in pixels as it is in rows.
    /// A picture larger than the window is scaled down to fit; one smaller than it is left
    /// alone rather than blown up. A sixteen-pixel icon enlarged to fill a terminal is a
    /// grid of coloured squares and tells one nothing; at its own size it is the icon.
    pub fn fit(&self, cols: u16, rows: u16) -> (u16, u16) {
        if cols == 0 || rows == 0 || self.width == 0 || self.height == 0 {
            return (0, 0);
        }
        let canvas_w = f64::from(cols);
        let canvas_h = f64::from(rows) * 2.0;
        let scale = (canvas_w / f64::from(self.width))
            .min(canvas_h / f64::from(self.height))
            .min(1.0);
        let w = ((f64::from(self.width) * scale).round() as u16).clamp(1, cols);
        let pixels_h = ((f64::from(self.height) * scale).round() as u16).max(1);
        // Round the pixel height up to a whole cell: half a cell cannot be drawn, and the
        // spare half is the background, which is what the rest of the window is anyway.
        let h = (pixels_h.div_ceil(2)).clamp(1, rows);
        (w, h)
    }
    /// The picture as cells, scaled to fit a window of `cols × rows` and composited over
    /// `bg`.
    ///
    /// The scaling is a box filter — each target pixel is the average of the source
    /// rectangle it covers — because the only direction that matters here is *down*, and
    /// averaging is what keeps a line of text in a screenshot readable where picking one
    /// source pixel per target would drop half the strokes.
    pub fn cells(&self, cols: u16, rows: u16, bg: [u8; 3]) -> Vec<Vec<Cell>> {
        let (w, h) = self.fit(cols, rows);
        if w == 0 || h == 0 {
            return Vec::new();
        }
        let pixel_h = usize::from(h) * 2;
        let pixel_w = usize::from(w);
        let mut out = Vec::with_capacity(usize::from(h));
        for row in 0..usize::from(h) {
            let mut line = Vec::with_capacity(pixel_w);
            for col in 0..pixel_w {
                let top = self.sample(col, row * 2, pixel_w, pixel_h, bg);
                let bottom = self.sample(col, row * 2 + 1, pixel_w, pixel_h, bg);
                line.push((top, bottom));
            }
            out.push(line);
        }
        out
    }

    /// The average colour of the source rectangle one target pixel covers, over `bg`.
    fn sample(&self, x: usize, y: usize, out_w: usize, out_h: usize, bg: [u8; 3]) -> [u8; 3] {
        let src_w = self.width as usize;
        let src_h = self.height as usize;
        let x0 = x * src_w / out_w;
        let x1 = (((x + 1) * src_w).div_ceil(out_w)).min(src_w).max(x0 + 1);
        let y0 = y * src_h / out_h;
        let y1 = (((y + 1) * src_h).div_ceil(out_h)).min(src_h).max(y0 + 1);

        // Premultiplied sums: a transparent pixel must not drag the colour of the opaque
        // ones towards whatever happens to be stored under its alpha.
        let (mut r, mut g, mut b, mut a) = (0u64, 0u64, 0u64, 0u64);
        let mut n = 0u64;
        for sy in y0..y1 {
            for sx in x0..x1 {
                let at = (sy * src_w + sx) * 4;
                let alpha = u64::from(self.pixels[at + 3]);
                r += u64::from(self.pixels[at]) * alpha;
                g += u64::from(self.pixels[at + 1]) * alpha;
                b += u64::from(self.pixels[at + 2]) * alpha;
                a += alpha;
                n += 1;
            }
        }
        if n == 0 || a == 0 {
            return bg;
        }
        let over = |sum: u64, back: u8| -> u8 {
            // The average colour where the pixels were opaque, mixed with the background in
            // the proportion the rectangle was transparent.
            let colour = sum / a;
            let cover = a; // out of n * 255
            let total = n * 255;
            ((colour * cover + u64::from(back) * (total - cover)) / total) as u8
        };
        [over(r, bg[0]), over(g, bg[1]), over(b, bg[2])]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny PNG written here, so the test does not depend on a file in the tree.
    fn png(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let mut raw = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                raw.extend_from_slice(&pixel(x, y));
            }
        }
        let buffer: image::RgbaImage =
            image::ImageBuffer::from_raw(width, height, raw).expect("the right number of bytes");
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(buffer)
            .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
            .expect("a PNG is written");
        out
    }

    #[test]
    fn a_file_is_a_picture_by_its_first_bytes_and_not_by_its_name() {
        assert!(is_picture(&png(2, 2, |_, _| [0, 0, 0, 255])));
        assert!(is_picture(b"\xff\xd8\xff\xe0 and whatever follows"));
        assert!(is_picture(b"GIF89a..."));
        assert!(is_picture(b"BM and a bitmap header"));
        assert!(!is_picture(b"plain text, which is not a picture"));
        assert!(!is_picture(b""));
        // A name that lies is not consulted at all.
        assert!(!is_picture(b"\x7fELF\x02\x01\x01"));
    }

    #[test]
    fn a_picture_keeps_its_shape_inside_the_window() {
        // Twice as wide as it is tall: in a square window it takes the full width and a
        // quarter of the rows, because a row is two pixels tall.
        let wide = read(&png(100, 50, |_, _| [255, 0, 0, 255])).unwrap();
        assert_eq!(wide.format, "PNG");
        assert_eq!((wide.width, wide.height), (100, 50));
        assert_eq!(wide.fit(80, 80), (80, 20));

        // Taller than it is wide: the height is what runs out first.
        let tall = read(&png(50, 100, |_, _| [0, 255, 0, 255])).unwrap();
        assert_eq!(tall.fit(80, 20), (20, 20));

        // Smaller than the window: left at its own size rather than blown up, because an
        // icon enlarged to fill a terminal is a grid of squares and says nothing.
        let icon = read(&png(16, 16, |_, _| [0, 0, 0, 255])).unwrap();
        assert_eq!(icon.fit(200, 60), (16, 8));

        // A window with no room in it draws nothing rather than panicking.
        assert_eq!(wide.fit(0, 10), (0, 0));
        assert!(wide.cells(0, 10, [0, 0, 0]).is_empty());
    }

    #[test]
    fn the_colours_come_out_where_they_went_in() {
        // The left half red, the right half blue.
        let bytes = png(40, 40, |x, _| {
            if x < 20 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            }
        });
        let picture = read(&bytes).unwrap();
        let cells = picture.cells(40, 20, [0, 0, 0]);
        assert_eq!(cells.len(), 20);
        assert_eq!(cells[0].len(), 40);
        let (top, bottom) = cells[0][0];
        assert_eq!(top, [255, 0, 0], "the top half of the first cell");
        assert_eq!(bottom, [255, 0, 0]);
        assert_eq!(cells[10][39].0, [0, 0, 255], "and the other side");
    }

    #[test]
    fn what_is_transparent_shows_the_background_through_it() {
        let bytes = png(10, 10, |_, _| [255, 255, 255, 0]);
        let picture = read(&bytes).unwrap();
        let cells = picture.cells(10, 5, [20, 30, 40]);
        assert_eq!(cells[0][0].0, [20, 30, 40], "nothing but background");

        // Half-transparent white over black is grey, and the sum is not dragged by the
        // colour stored under a fully transparent pixel.
        let bytes = png(10, 10, |_, _| [255, 255, 255, 128]);
        let picture = read(&bytes).unwrap();
        let grey = picture.cells(10, 5, [0, 0, 0])[0][0].0;
        assert!(
            (120..=135).contains(&grey[0]),
            "half white over black: {grey:?}"
        );
    }

    #[test]
    fn scaling_down_averages_rather_than_dropping_rows() {
        // A chequerboard one pixel square: every cell of a scaled-down drawing should come
        // out mid-grey, which is only true of an average — picking one source pixel would
        // give black or white.
        let bytes = png(64, 64, |x, y| {
            let on = (x + y) % 2 == 0;
            let v = if on { 255 } else { 0 };
            [v, v, v, 255]
        });
        let picture = read(&bytes).unwrap();
        for row in picture.cells(16, 8, [0, 0, 0]) {
            for (top, bottom) in row {
                for value in [top[0], bottom[0]] {
                    assert!((100..=155).contains(&value), "{value} is not an average");
                }
            }
        }
    }

    #[test]
    fn a_damaged_file_says_so_instead_of_being_drawn() {
        let mut bytes = png(8, 8, |_, _| [1, 2, 3, 255]);
        let len = bytes.len();
        bytes.truncate(len / 2);
        assert!(read(&bytes).is_err());
        assert!(read(b"not a picture at all").is_err());
    }
}

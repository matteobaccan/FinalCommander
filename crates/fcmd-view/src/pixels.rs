//! Pictures in real pixels (SPEC 05 §1.14a).
//!
//! Three terminals' worth of protocol put a picture on the screen as itself rather than as
//! half blocks: kitty's graphics protocol and iTerm2's inline images both take a PNG, and
//! sixel takes a palette and six rows of pixels at a time. What is made here is the *image*
//! each of them carries — a PNG, or a whole sixel string — already scaled to the cells it
//! will cover; wrapping it in the terminal's own escape sequence and deciding when to send
//! it is the terminal backend's job, which is the only place that knows what is on screen.
//!
//! Nothing here asks the terminal anything. The size of a cell in pixels is handed in, the
//! picture is scaled to that, and a wrong guess costs sharpness or a margin, never a hang.

use crate::picture::Picture;

/// How a window of `cols × rows` cells, each `cell` pixels, is filled by a picture: the cells
/// it takes and the pixels it is scaled to inside them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fit {
    pub cols: u16,
    pub rows: u16,
    /// The picture's own size once scaled, which is at most the cells' and keeps its shape.
    pub width: u32,
    pub height: u32,
}

/// Fit a picture of `width × height` pixels into `cols × rows` cells of `cell` pixels each.
///
/// The same two rules the half blocks follow: a picture larger than the window is scaled
/// down keeping its shape, and one smaller is left at its own size — which in real pixels
/// means an icon is drawn as small as it is, and that is what it looks like.
pub fn fit(width: u32, height: u32, cols: u16, rows: u16, cell: (u16, u16)) -> Option<Fit> {
    if cols == 0 || rows == 0 || width == 0 || height == 0 || cell.0 == 0 || cell.1 == 0 {
        return None;
    }
    let (cw, ch) = (f64::from(cell.0), f64::from(cell.1));
    let canvas_w = f64::from(cols) * cw;
    let canvas_h = f64::from(rows) * ch;
    let scale = (canvas_w / f64::from(width))
        .min(canvas_h / f64::from(height))
        .min(1.0);
    let w = ((f64::from(width) * scale).round() as u32).clamp(1, canvas_w as u32);
    let h = ((f64::from(height) * scale).round() as u32).clamp(1, canvas_h as u32);
    Some(Fit {
        cols: (w.div_ceil(u32::from(cell.0)) as u16).clamp(1, cols),
        rows: (h.div_ceil(u32::from(cell.1)) as u16).clamp(1, rows),
        width: w,
        height: h,
    })
}

/// The picture scaled into the cells it covers: a canvas of exactly `cols × rows` cells'
/// worth of pixels with the picture centred in it, straight RGBA.
///
/// The canvas is whole cells on purpose. kitty and iTerm2 are told how many cells the image
/// covers and stretch it to them, so an image that is not a whole number of cells would be
/// stretched out of shape by up to a cell; padded to the cells, it is drawn as it is. What
/// the padding is made of is `under`: nothing at all (transparent, for a PNG, so the
/// terminal's own background shows) or a colour, for sixel, which has no transparency worth
/// relying on.
fn canvas(picture: &Picture, fit: Fit, cell: (u16, u16), under: Option<[u8; 3]>) -> Vec<u8> {
    let w = usize::from(fit.cols) * usize::from(cell.0);
    let h = usize::from(fit.rows) * usize::from(cell.1);
    let (pw, ph) = ((fit.width as usize).min(w), (fit.height as usize).min(h));
    let (x0, y0) = ((w - pw) / 2, (h - ph) / 2);
    let blank = match under {
        Some([r, g, b]) => [r, g, b, 255],
        None => [0, 0, 0, 0],
    };
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let inside = x >= x0 && x < x0 + pw && y >= y0 && y < y0 + ph;
            let pixel = if inside {
                let [r, g, b, a] = picture.sample_rgba(x - x0, y - y0, pw, ph);
                match under {
                    Some(bg) => {
                        let mix = |c: u8, back: u8| -> u8 {
                            ((u32::from(c) * u32::from(a) + u32::from(back) * (255 - u32::from(a)))
                                / 255) as u8
                        };
                        [mix(r, bg[0]), mix(g, bg[1]), mix(b, bg[2]), 255]
                    }
                    None => [r, g, b, a],
                }
            } else {
                blank
            };
            out.extend_from_slice(&pixel);
        }
    }
    out
}

/// The picture as a PNG covering `fit`'s cells, which is what kitty and iTerm2 are sent.
///
/// Compressed quickly rather than well: the bytes go down a pipe to a terminal on the same
/// machine, or down an `ssh` connection that compresses for itself, and are thrown away the
/// moment they are drawn.
pub fn png(picture: &Picture, fit: Fit, cell: (u16, u16)) -> Result<Vec<u8>, String> {
    use image::ImageEncoder as _;
    let rgba = canvas(picture, fit, cell, None);
    let w = u32::from(fit.cols) * u32::from(cell.0);
    let h = u32::from(fit.rows) * u32::from(cell.1);
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new_with_quality(
        &mut out,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::Adaptive,
    )
    .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
    .map_err(|e| e.to_string())?;
    Ok(out)
}

/// The picture as one sixel string covering `fit`'s cells, over the colour `bg`.
pub fn sixel(picture: &Picture, fit: Fit, cell: (u16, u16), bg: [u8; 3]) -> Vec<u8> {
    let rgba = canvas(picture, fit, cell, Some(bg));
    let w = usize::from(fit.cols) * usize::from(cell.0);
    let h = usize::from(fit.rows) * usize::from(cell.1);
    let rgb: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    sixel_of(&rgb, w, h)
}

// ── sixel ───────────────────────────────────────────────────────────────────────────────────

/// The most colours one sixel image is given. A VT340 had sixteen registers; every terminal
/// that draws sixel today has at least 256, and asking for more is asking some of them to
/// drop the image.
const PALETTE: usize = 256;

/// Five bits a channel: the grid colours are counted on before a palette is chosen. Thirty-two
/// thousand cells is a table that fits in the cache and still tells two neighbouring shades
/// of a sky apart.
const BINS: usize = 1 << 15;

fn bin_of(r: u8, g: u8, b: u8) -> usize {
    (usize::from(r >> 3) << 10) | (usize::from(g >> 3) << 5) | usize::from(b >> 3)
}

/// One cell of the colour grid: how many pixels fell in it and what they added up to, so a
/// palette entry is the average of real pixels and not the corner of a cube.
#[derive(Clone, Copy, Default)]
struct Bin {
    count: u64,
    sum: [u64; 3],
}

/// Encode `w × h` RGB pixels as sixel.
///
/// The palette is chosen by median cut — the box holding the most pixels is split across its
/// widest channel at the pixel that halves it, until there are 256 boxes — and the pixels
/// are mapped to it with Floyd–Steinberg's error diffusion, which is what turns 256 colours
/// into a photograph instead of a poster. A picture that has 256 colours or fewer to begin
/// with gets exactly those and no dithering at all: a screenshot stays a screenshot.
///
/// Everything here is arithmetic on the pixels in a fixed order, so the same picture is the
/// same bytes every time.
pub fn sixel_of(rgb: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = Vec::new();
    if w == 0 || h == 0 || rgb.len() < w * h * 3 {
        return out;
    }
    let mut bins = vec![Bin::default(); BINS];
    for p in rgb[..w * h * 3].as_chunks::<3>().0 {
        let bin = &mut bins[bin_of(p[0], p[1], p[2])];
        bin.count += 1;
        for (sum, value) in bin.sum.iter_mut().zip(p) {
            *sum += u64::from(*value);
        }
    }
    let used: Vec<u16> = (0..BINS as u32)
        .filter(|&i| bins[i as usize].count > 0)
        .map(|i| i as u16)
        .collect();
    let exact = used.len() <= PALETTE;
    let boxes = if exact {
        used.iter().map(|&i| vec![i]).collect()
    } else {
        median_cut(&bins, used)
    };

    // The palette, and for every grid cell that had pixels in it, which entry it belongs to.
    let mut palette: Vec<[u8; 3]> = Vec::with_capacity(boxes.len());
    let mut lookup = vec![u16::MAX; BINS];
    for (index, members) in boxes.iter().enumerate() {
        let (mut count, mut sum) = (0u64, [0u64; 3]);
        for &m in members {
            let bin = &bins[usize::from(m)];
            count += bin.count;
            for (sum, part) in sum.iter_mut().zip(bin.sum) {
                *sum += part;
            }
            lookup[usize::from(m)] = index as u16;
        }
        let count = count.max(1);
        palette.push([
            (sum[0] / count) as u8,
            (sum[1] / count) as u8,
            (sum[2] / count) as u8,
        ]);
    }

    let indexed = if exact {
        rgb[..w * h * 3]
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| lookup[bin_of(p[0], p[1], p[2])] as u8)
            .collect()
    } else {
        dither(rgb, w, h, &palette, &mut lookup)
    };

    // `P2 = 1`: a pixel this image does not set is left as it was. Every pixel is set, so it
    // only matters to the last band, whose rows below the picture's own must not be painted.
    // The raster attributes say the size in pixels up front, at one pixel to one.
    out.extend_from_slice(format!("\x1bP0;1;0q\"1;1;{w};{h}").as_bytes());
    for (i, c) in palette.iter().enumerate() {
        // A sixel colour is three percentages, which is what a VT340 took.
        let pc = |v: u8| (u32::from(v) * 100 + 127) / 255;
        out.extend_from_slice(format!("#{i};2;{};{};{}", pc(c[0]), pc(c[1]), pc(c[2])).as_bytes());
    }
    let bands = h.div_ceil(6);
    let mut slot = vec![usize::MAX; palette.len()];
    for band in 0..bands {
        // One line of sixels per colour that appears in the band: bit `k` of a character is
        // the pixel `k` rows down.
        let mut lines: Vec<(usize, Vec<u8>)> = Vec::new();
        for k in 0..6 {
            let y = band * 6 + k;
            if y >= h {
                break;
            }
            for x in 0..w {
                let colour = usize::from(indexed[y * w + x]);
                if slot[colour] == usize::MAX {
                    slot[colour] = lines.len();
                    lines.push((colour, vec![0u8; w]));
                }
                lines[slot[colour]].1[x] |= 1 << k;
            }
        }
        // In the palette's order, so the bytes do not depend on which pixel came first.
        lines.sort_by_key(|(colour, _)| *colour);
        for (n, (colour, bits)) in lines.iter().enumerate() {
            slot[*colour] = usize::MAX;
            if n > 0 {
                // Back to the left edge of the same band, for the next colour.
                out.push(b'$');
            }
            out.extend_from_slice(format!("#{colour}").as_bytes());
            // What is blank to the right of the last pixel need not be said at all.
            let end = bits.iter().rposition(|&b| b != 0).map_or(0, |p| p + 1);
            let mut x = 0;
            while x < end {
                let run = bits[x..end].iter().take_while(|&&b| b == bits[x]).count();
                let ch = 63 + bits[x];
                if run > 3 {
                    out.extend_from_slice(format!("!{run}").as_bytes());
                    out.push(ch);
                } else {
                    out.extend(std::iter::repeat_n(ch, run));
                }
                x += run;
            }
        }
        if band + 1 < bands {
            out.push(b'-');
        }
    }
    out.extend_from_slice(b"\x1b\\");
    out
}

/// Split the colour grid into at most [`PALETTE`] boxes, each a list of grid cells.
fn median_cut(bins: &[Bin], used: Vec<u16>) -> Vec<Vec<u16>> {
    let channel = |bin: u16, c: usize| -> u16 { (bin >> (10 - 5 * c)) & 31 };
    let mut boxes = vec![used];
    while boxes.len() < PALETTE {
        // The box with the most pixels in it that can still be split.
        let Some(at) = (0..boxes.len())
            .filter(|&i| boxes[i].len() > 1)
            .max_by_key(|&i| {
                let pixels: u64 = boxes[i].iter().map(|&b| bins[usize::from(b)].count).sum();
                // The earlier box on a tie, so the order of the palette is fixed.
                (pixels, std::cmp::Reverse(i))
            })
        else {
            break;
        };
        let mut members = std::mem::take(&mut boxes[at]);
        let widest = (0..3)
            .max_by_key(|&c| {
                let lo = members.iter().map(|&b| channel(b, c)).min().unwrap_or(0);
                let hi = members.iter().map(|&b| channel(b, c)).max().unwrap_or(0);
                (hi - lo, std::cmp::Reverse(c))
            })
            .unwrap_or(0);
        members.sort_by_key(|&b| (channel(b, widest), b));
        let total: u64 = members.iter().map(|&b| bins[usize::from(b)].count).sum();
        let mut seen = 0u64;
        let mut cut = members.len() - 1;
        for (i, &b) in members.iter().enumerate() {
            seen += bins[usize::from(b)].count;
            if seen * 2 >= total {
                cut = i + 1;
                break;
            }
        }
        // Both halves keep at least one cell, or the loop would split nothing for ever.
        let cut = cut.clamp(1, members.len() - 1);
        let rest = members.split_off(cut);
        boxes[at] = members;
        boxes.push(rest);
    }
    boxes
}

/// Map every pixel to a palette entry, passing on what was lost to the pixels not yet drawn.
fn dither(rgb: &[u8], w: usize, h: usize, palette: &[[u8; 3]], lookup: &mut [u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(w * h);
    // The error owed to this row and to the next, three channels a pixel, a pixel of margin
    // at each end so the edges need no special case.
    let mut here = vec![0i32; (w + 2) * 3];
    let mut below = vec![0i32; (w + 2) * 3];
    for y in 0..h {
        for x in 0..w {
            let at = (y * w + x) * 3;
            let mut want = [0u8; 3];
            for c in 0..3 {
                // Sixteenths, as Floyd and Steinberg's weights are.
                let v = i32::from(rgb[at + c]) + here[(x + 1) * 3 + c] / 16;
                want[c] = v.clamp(0, 255) as u8;
            }
            let bin = bin_of(want[0], want[1], want[2]);
            if lookup[bin] == u16::MAX {
                // A colour no pixel had, made by the error: the nearest entry, found once.
                lookup[bin] = nearest(palette, want) as u16;
            }
            let index = lookup[bin];
            out.push(index as u8);
            let got = palette[usize::from(index)];
            for c in 0..3 {
                let err = i32::from(want[c]) - i32::from(got[c]);
                here[(x + 2) * 3 + c] += err * 7;
                below[x * 3 + c] += err * 3;
                below[(x + 1) * 3 + c] += err * 5;
                below[(x + 2) * 3 + c] += err;
            }
        }
        std::mem::swap(&mut here, &mut below);
        below.iter_mut().for_each(|v| *v = 0);
    }
    out
}

fn nearest(palette: &[[u8; 3]], want: [u8; 3]) -> usize {
    let distance = |p: &[u8; 3]| -> i32 {
        (0..3)
            .map(|c| (i32::from(p[c]) - i32::from(want[c])).pow(2))
            .sum()
    };
    (0..palette.len())
        .min_by_key(|&i| (distance(&palette[i]), i))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Picture {
        let mut raw = Vec::new();
        for y in 0..height {
            for x in 0..width {
                raw.extend_from_slice(&pixel(x, y));
            }
        }
        Picture::from_rgba(width, height, "PNG", raw).expect("the right number of bytes")
    }

    #[test]
    fn a_picture_fits_the_cells_by_their_size_in_pixels() {
        // 800×600 into 80×24 cells of 8×16: the window is 640×384 pixels, so the height is
        // what runs out, and 512 pixels across is 64 cells.
        let f = fit(800, 600, 80, 24, (8, 16)).unwrap();
        assert_eq!((f.width, f.height), (512, 384));
        assert_eq!((f.cols, f.rows), (64, 24));
        // The same picture where a cell is 10×20, as Windows Terminal counts them: 800×480.
        let f = fit(800, 600, 80, 24, (10, 20)).unwrap();
        assert_eq!((f.width, f.height), (640, 480));
        assert_eq!((f.cols, f.rows), (64, 24));
        // An icon is not blown up: sixteen pixels are two cells by one.
        let f = fit(16, 16, 80, 24, (8, 16)).unwrap();
        assert_eq!((f.cols, f.rows, f.width, f.height), (2, 1, 16, 16));
        // And a part of a cell is a whole cell, because half of one cannot be claimed.
        let f = fit(20, 20, 80, 24, (8, 16)).unwrap();
        assert_eq!((f.cols, f.rows), (3, 2));
        // No room, or a cell with no size, is no drawing rather than a division by zero.
        assert_eq!(fit(16, 16, 0, 24, (8, 16)), None);
        assert_eq!(fit(16, 16, 80, 24, (0, 16)), None);
    }

    #[test]
    fn the_png_is_whole_cells_with_the_picture_centred_on_nothing() {
        // Twenty red pixels square in cells of 8×16 take 3×2 cells, which is 24×32 pixels:
        // the picture in the middle and transparent padding round it.
        let red = picture(20, 20, |_, _| [255, 0, 0, 255]);
        let f = fit(20, 20, 80, 24, (8, 16)).unwrap();
        let bytes = png(&red, f, (8, 16)).unwrap();
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        let back = crate::picture::read(&bytes).unwrap();
        assert_eq!((back.width, back.height), (24, 32));
        assert_eq!(back.sample_rgba(0, 0, 24, 32), [0, 0, 0, 0], "padding");
        assert_eq!(back.sample_rgba(12, 16, 24, 32), [255, 0, 0, 255]);
        assert_eq!(
            back.sample_rgba(2, 6, 24, 32),
            [255, 0, 0, 255],
            "its corner"
        );
        assert_eq!(back.sample_rgba(1, 6, 24, 32), [0, 0, 0, 0]);
    }

    #[test]
    fn four_pixels_of_two_colours_are_these_sixel_bytes() {
        // Two pixels across and two down: red over blue on the left, blue over red on the
        // right. Bit 0 is the top row, so red is `@` (1) then `A` (2), and blue the reverse.
        let rgb = [255, 0, 0, 0, 0, 255, 0, 0, 255, 255, 0, 0];
        let bytes = sixel_of(&rgb, 2, 2);
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "\x1bP0;1;0q\"1;1;2;2#0;2;0;0;100#1;2;100;0;0#0A@$#1@A\x1b\\"
        );
    }

    #[test]
    fn a_run_of_the_same_sixel_is_written_once_with_its_count() {
        // Ten white pixels in a row, seven rows of them: the first band is all six bits
        // (`~`), the second the one row left over (`@`), each said once with `!10`.
        let rgb = vec![255u8; 10 * 7 * 3];
        let text = String::from_utf8(sixel_of(&rgb, 10, 7)).unwrap();
        assert_eq!(
            text,
            "\x1bP0;1;0q\"1;1;10;7#0;2;100;100;100#0!10~-#0!10@\x1b\\"
        );
    }

    #[test]
    fn what_is_blank_to_the_right_of_a_colour_is_not_written() {
        // Black then three whites: black's line stops after its one pixel.
        let rgb = [0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255];
        let text = String::from_utf8(sixel_of(&rgb, 4, 1)).unwrap();
        assert!(text.ends_with("#0@$#1?@@@\x1b\\"), "{text:?}");
    }

    #[test]
    fn a_photograph_gets_a_full_palette_and_the_same_bytes_every_time() {
        // A gradient with far more than 256 colours in it.
        let (w, h) = (96usize, 48usize);
        let mut rgb = Vec::new();
        for y in 0..h {
            for x in 0..w {
                rgb.extend_from_slice(&[
                    (x * 255 / w) as u8,
                    (y * 255 / h) as u8,
                    ((x + y) % 256) as u8,
                ]);
            }
        }
        let once = sixel_of(&rgb, w, h);
        assert_eq!(
            once,
            sixel_of(&rgb, w, h),
            "nothing in it is left to chance"
        );
        let text = String::from_utf8(once).unwrap();
        assert!(text.starts_with("\x1bP0;1;0q\"1;1;96;48#0;2;"));
        assert!(text.ends_with("\x1b\\"));
        // 256 registers defined, numbered from nought, and eight bands of six rows.
        assert!(text.contains("#255;2;"));
        assert!(!text.contains("#256"));
        assert_eq!(text.matches('-').count(), 7);
        // Nothing but the printable sixel alphabet between the introducer and the end.
        let body = &text[2..text.len() - 2];
        assert!(body.bytes().all(|b| (0x20..0x7f).contains(&b)));
    }

    #[test]
    fn sixel_composites_onto_the_window_and_pads_with_its_colour() {
        // A transparent picture over a blue window is a blue rectangle of whole cells.
        let clear = picture(8, 8, |_, _| [255, 255, 255, 0]);
        let f = fit(8, 8, 10, 10, (8, 16)).unwrap();
        let text = String::from_utf8(sixel(&clear, f, (8, 16), [0, 0, 255])).unwrap();
        assert!(
            text.starts_with("\x1bP0;1;0q\"1;1;8;16#0;2;0;0;100#0!8~-"),
            "{text:?}"
        );
        // An empty picture is no bytes at all, not a header with nothing after it.
        assert!(sixel_of(&[], 0, 0).is_empty());
    }
}

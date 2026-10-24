use std::fmt;

/// A colour as a theme declares it. The terminal backend degrades it to what the terminal
/// actually supports (see [`Color::degrade`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    /// The terminal's own default foreground/background.
    #[default]
    Default,
    /// One of the 16 ANSI colours.
    Ansi(u8),
    /// A 256-colour palette index.
    Indexed(u8),
    Rgb(u8, u8, u8),
}

/// How much colour the terminal can take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorDepth {
    Mono,
    Ansi16,
    Indexed256,
    TrueColor,
}

impl Color {
    /// Half as bright, for the shadow a dialog drops on what is behind it. FAR merges the
    /// cell with half-transparent black where it can and walks the palette down where it
    /// cannot; this does both, depending on what kind of colour it is.
    pub fn darkened(self) -> Color {
        match self {
            Color::Rgb(r, g, b) => Color::Rgb(r / 2, g / 2, b / 2),
            // 8..15 are the bright half of the ANSI palette: their dark twin is eight below.
            Color::Ansi(n) if n >= 8 => Color::Ansi(n - 8),
            Color::Ansi(0) => Color::Ansi(0),
            Color::Ansi(_) => Color::Ansi(0),
            Color::Indexed(n) => Color::Indexed(n),
            Color::Default => Color::Ansi(0),
        }
    }

    pub fn parse(s: &str) -> Option<Color> {
        let s = s.trim();
        if s.is_empty() || s.eq_ignore_ascii_case("default") || s == "-" {
            return Some(Color::Default);
        }
        if let Some(hex) = s.strip_prefix('#') {
            return match hex.len() {
                6 => Some(Color::Rgb(
                    u8::from_str_radix(&hex[0..2], 16).ok()?,
                    u8::from_str_radix(&hex[2..4], 16).ok()?,
                    u8::from_str_radix(&hex[4..6], 16).ok()?,
                )),
                3 => {
                    let d = |i: usize| -> Option<u8> {
                        let v = u8::from_str_radix(&hex[i..i + 1], 16).ok()?;
                        Some(v * 17)
                    };
                    Some(Color::Rgb(d(0)?, d(1)?, d(2)?))
                }
                _ => None,
            };
        }
        if let Some(n) = s.strip_prefix("ansi:") {
            return n
                .trim()
                .parse::<u8>()
                .ok()
                .filter(|v| *v < 16)
                .map(Color::Ansi);
        }
        if let Some(n) = s.strip_prefix("idx:") {
            return n.trim().parse::<u8>().ok().map(Color::Indexed);
        }
        // Named ANSI colours, so a theme can be written without hex.
        let named = [
            "black",
            "red",
            "green",
            "yellow",
            "blue",
            "magenta",
            "cyan",
            "white",
            "brightblack",
            "brightred",
            "brightgreen",
            "brightyellow",
            "brightblue",
            "brightmagenta",
            "brightcyan",
            "brightwhite",
        ];
        let lower = s.to_ascii_lowercase().replace([' ', '-', '_'], "");
        named
            .iter()
            .position(|n| *n == lower)
            .map(|i| Color::Ansi(i as u8))
    }

    /// Map to what the terminal can show. Never panics, never produces an invalid index.
    pub fn degrade(self, depth: ColorDepth) -> Color {
        match (self, depth) {
            (_, ColorDepth::Mono) => Color::Default,
            (Color::Rgb(r, g, b), ColorDepth::Indexed256) => Color::Indexed(rgb_to_256(r, g, b)),
            (Color::Rgb(r, g, b), ColorDepth::Ansi16) => Color::Ansi(rgb_to_ansi16(r, g, b)),
            (Color::Indexed(i), ColorDepth::Ansi16) => {
                let (r, g, b) = index_to_rgb(i);
                Color::Ansi(rgb_to_ansi16(r, g, b))
            }
            (c, _) => c,
        }
    }
}

fn rgb_to_256(r: u8, g: u8, b: u8) -> u8 {
    // Grey ramp first: it is much closer for near-grey colours than the 6x6x6 cube.
    let max = r.max(g).max(b) as i32;
    let min = r.min(g).min(b) as i32;
    if max - min < 10 {
        let level = ((r as i32 + g as i32 + b as i32) / 3) as u8;
        return match level {
            0..=3 => 16,
            248..=255 => 231,
            l => 232 + ((l as u16 - 8) * 24 / 240).min(23) as u8,
        };
    }
    let q = |v: u8| -> u16 { ((v as u16 * 5) + 127) / 255 };
    (16 + 36 * q(r) + 6 * q(g) + q(b)) as u8
}

/// The colour an index actually is, for anything that has to draw outside a terminal — the
/// SVG screenshots of `svg.rs`, and the degradation above.
pub fn index_to_rgb(i: u8) -> (u8, u8, u8) {
    match i {
        0..=15 => {
            let base = [
                (0, 0, 0),
                (170, 0, 0),
                (0, 170, 0),
                (170, 85, 0),
                (0, 0, 170),
                (170, 0, 170),
                (0, 170, 170),
                (170, 170, 170),
                (85, 85, 85),
                (255, 85, 85),
                (85, 255, 85),
                (255, 255, 85),
                (85, 85, 255),
                (255, 85, 255),
                (85, 255, 255),
                (255, 255, 255),
            ];
            base[i as usize]
        }
        16..=231 => {
            let i = i - 16;
            let steps = [0u8, 95, 135, 175, 215, 255];
            (
                steps[(i / 36) as usize],
                steps[((i % 36) / 6) as usize],
                steps[(i % 6) as usize],
            )
        }
        _ => {
            let l = 8 + (i as u16 - 232) * 10;
            let l = l.min(255) as u8;
            (l, l, l)
        }
    }
}

fn rgb_to_ansi16(r: u8, g: u8, b: u8) -> u8 {
    let bright = r.max(g).max(b) > 150;
    let bit = |v: u8| u8::from(v > 100);
    let idx = bit(r) | (bit(g) << 1) | (bit(b) << 2);
    // ANSI order is black, red, green, yellow, blue, magenta, cyan, white.
    let ansi = match idx {
        0b000 => 0,
        0b001 => 1,
        0b010 => 2,
        0b011 => 3,
        0b100 => 4,
        0b101 => 5,
        0b110 => 6,
        _ => 7,
    };
    if bright {
        ansi + 8
    } else {
        ansi
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Color::Default => write!(f, "default"),
            Color::Ansi(n) => write!(f, "ansi:{n}"),
            Color::Indexed(n) => write!(f, "idx:{n}"),
            Color::Rgb(r, g, b) => write!(f, "#{r:02x}{g:02x}{b:02x}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_accepted_form() {
        assert_eq!(Color::parse("#ff8000"), Some(Color::Rgb(255, 128, 0)));
        assert_eq!(Color::parse("#fff"), Some(Color::Rgb(255, 255, 255)));
        assert_eq!(Color::parse("ansi:4"), Some(Color::Ansi(4)));
        assert_eq!(Color::parse("idx:208"), Some(Color::Indexed(208)));
        assert_eq!(Color::parse("brightCyan"), Some(Color::Ansi(14)));
        assert_eq!(Color::parse("default"), Some(Color::Default));
        assert_eq!(Color::parse("#12345"), None);
        assert_eq!(Color::parse("ansi:99"), None);
    }

    #[test]
    fn display_round_trips() {
        for c in [
            Color::Default,
            Color::Ansi(9),
            Color::Indexed(200),
            Color::Rgb(1, 2, 3),
        ] {
            assert_eq!(Color::parse(&c.to_string()), Some(c));
        }
    }

    #[test]
    fn degradation_stays_in_range_and_keeps_greys_grey() {
        let green = Color::Rgb(0, 255, 0);
        assert_eq!(green.degrade(ColorDepth::TrueColor), green);
        assert_eq!(green.degrade(ColorDepth::Ansi16), Color::Ansi(10));
        assert_eq!(green.degrade(ColorDepth::Mono), Color::Default);
        match Color::Rgb(128, 128, 128).degrade(ColorDepth::Indexed256) {
            Color::Indexed(i) => assert!((232..=255).contains(&i), "grey became {i}"),
            other => panic!("expected an indexed colour, got {other}"),
        }
        for v in 0..=255u8 {
            if let Color::Indexed(i) = Color::Rgb(v, v / 2, 255 - v).degrade(ColorDepth::Indexed256)
            {
                assert!(i >= 16);
            }
        }
    }
}

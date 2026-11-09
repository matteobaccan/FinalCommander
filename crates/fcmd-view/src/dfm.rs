//! Delphi's binary forms, read as the text Delphi itself shows for them.
//!
//! A `.dfm` (and `.fmx`, `.lfm`, `.xfm`) is either text — `object Form1: TForm1 … end` — or the
//! same thing written by `TWriter` in binary, starting with `TPF0`. The binary kind is what
//! older projects keep and what every Delphi program carries in its resources, one per form,
//! and outside the IDE nobody can read it. This turns it back into the text form, the way
//! *View as Text* does: the objects nested, their properties `Name = value`, collections in
//! `< … >`, lists in `( … )`, sets in `[ … ]` and binary data in `{ … }` as hex.
//!
//! The grammar is the one `TReader` reads (Classes.pas): a component is an optional filer
//! prefix, the class name, the object name, its properties up to an empty name, and its
//! children up to a zero byte. Values carry their own type byte.

use std::fmt::Write as _;

/// The signature a binary form starts with.
pub const SIGNATURE: &[u8; 4] = b"TPF0";

/// Is this a binary form?
pub fn is_binary_form(bytes: &[u8]) -> bool {
    bytes.starts_with(SIGNATURE)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not a form Delphi could read: {}", self.0)
    }
}

impl std::error::Error for Error {}

/// The text form of a binary one.
pub fn to_text(bytes: &[u8]) -> Result<String, Error> {
    if !is_binary_form(bytes) {
        return Err(Error("no TPF0 signature".into()));
    }
    let mut r = Reader {
        bytes,
        at: SIGNATURE.len(),
    };
    let mut out = String::new();
    r.component(&mut out, 0)?;
    Ok(out)
}

/// The class of the form, without reading the rest: `TForm1` for `object Form1: TForm1`.
pub fn class_name(bytes: &[u8]) -> Option<String> {
    let mut r = Reader {
        bytes,
        at: SIGNATURE.len(),
    };
    if !is_binary_form(bytes) {
        return None;
    }
    r.prefix().ok()?;
    r.short_string().ok()
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Result<u8, Error> {
        let b = *self
            .bytes
            .get(self.at)
            .ok_or_else(|| Error(format!("cut short at byte {}", self.at)))?;
        self.at += 1;
        Ok(b)
    }

    fn peek(&self) -> Result<u8, Error> {
        self.bytes
            .get(self.at)
            .copied()
            .ok_or_else(|| Error(format!("cut short at byte {}", self.at)))
    }

    fn take(&mut self, n: usize) -> Result<&[u8], Error> {
        let end = self
            .at
            .checked_add(n)
            .filter(|e| *e <= self.bytes.len())
            .ok_or_else(|| Error(format!("{n} bytes wanted at {}", self.at)))?;
        let s = &self.bytes[self.at..end];
        self.at = end;
        Ok(s)
    }

    fn i16(&mut self) -> Result<i16, Error> {
        Ok(i16::from_le_bytes(self.take(2)?.try_into().expect("two")))
    }

    fn i32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().expect("four")))
    }

    fn i64(&mut self) -> Result<i64, Error> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().expect("eight")))
    }

    fn len32(&mut self) -> Result<usize, Error> {
        let n = self.i32()?;
        usize::try_from(n).map_err(|_| Error(format!("a length of {n}")))
    }

    fn short_string(&mut self) -> Result<String, Error> {
        let n = self.byte()? as usize;
        Ok(ansi(self.take(n)?))
    }

    /// `ffInherited`, `ffChildPos`, `ffInline`: a byte `0xF?` before the class name, with the
    /// child's position after it when it says so.
    fn prefix(&mut self) -> Result<u8, Error> {
        let b = self.peek()?;
        if b & 0xF0 != 0xF0 {
            return Ok(0);
        }
        self.at += 1;
        let flags = b & 0x0F;
        if flags & 2 != 0 {
            // The position is an integer value with its own type byte.
            let kind = self.byte()?;
            self.integer(kind)?;
        }
        Ok(flags)
    }

    fn integer(&mut self, kind: u8) -> Result<i64, Error> {
        Ok(match kind {
            2 => self.byte()? as i8 as i64,
            3 => self.i16()? as i64,
            4 => self.i32()? as i64,
            19 => self.i64()?,
            other => return Err(Error(format!("an integer of type {other}"))),
        })
    }

    fn component(&mut self, out: &mut String, depth: usize) -> Result<(), Error> {
        let flags = self.prefix()?;
        let class = self.short_string()?;
        let name = self.short_string()?;
        let pad = "  ".repeat(depth);
        let word = if flags & 1 != 0 {
            "inherited"
        } else if flags & 4 != 0 {
            "inline"
        } else {
            "object"
        };
        if name.is_empty() {
            let _ = writeln!(out, "{pad}{word} {class}");
        } else {
            let _ = writeln!(out, "{pad}{word} {name}: {class}");
        }
        self.properties(out, depth + 1)?;
        // The children, up to a zero byte.
        while self.peek()? != 0 {
            self.component(out, depth + 1)?;
        }
        self.at += 1;
        let _ = writeln!(out, "{pad}end");
        Ok(())
    }

    fn properties(&mut self, out: &mut String, depth: usize) -> Result<(), Error> {
        let pad = "  ".repeat(depth);
        loop {
            let name = self.short_string()?;
            if name.is_empty() {
                return Ok(());
            }
            let _ = write!(out, "{pad}{name} = ");
            self.value(out, depth)?;
            out.push('\n');
        }
    }

    fn value(&mut self, out: &mut String, depth: usize) -> Result<(), Error> {
        let kind = self.byte()?;
        let pad = "  ".repeat(depth + 1);
        match kind {
            0 => out.push_str("nil"),
            // vaList: values up to vaNull.
            1 => {
                out.push('(');
                while self.peek()? != 0 {
                    let _ = write!(out, "\n{pad}");
                    self.value(out, depth + 1)?;
                }
                self.at += 1;
                out.push(')');
            }
            2..=4 | 19 => {
                let n = self.integer(kind)?;
                let _ = write!(out, "{n}");
            }
            5 => {
                let raw: [u8; 10] = self.take(10)?.try_into().expect("ten");
                out.push_str(&number(extended(raw)));
            }
            6 | 7 => {
                let s = self.short_string()?;
                if kind == 7 {
                    out.push_str(&s);
                } else {
                    out.push_str(&quoted(&s));
                }
            }
            8 => out.push_str("False"),
            9 => out.push_str("True"),
            // vaBinary: hex, 32 bytes a line, as Delphi writes it.
            10 => {
                let n = self.len32()?;
                let data = self.take(n)?;
                out.push('{');
                for line in data.chunks(32) {
                    let _ = write!(out, "\n{pad}");
                    for b in line {
                        let _ = write!(out, "{b:02X}");
                    }
                }
                out.push('}');
            }
            // vaSet: names up to an empty one.
            11 => {
                let mut items = Vec::new();
                loop {
                    let s = self.short_string()?;
                    if s.is_empty() {
                        break;
                    }
                    items.push(s);
                }
                let _ = write!(out, "[{}]", items.join(", "));
            }
            12 => {
                let n = self.len32()?;
                let s = ansi(self.take(n)?);
                out.push_str(&quoted(&s));
            }
            13 => out.push_str("nil"),
            // vaCollection: items, each an optional order and a property list, up to vaNull.
            14 => {
                out.push('<');
                while self.peek()? != 0 {
                    let mark = self.peek()?;
                    if matches!(mark, 2..=4) {
                        self.at += 1;
                        let n = self.integer(mark)?;
                        let _ = write!(out, "\n{pad}item [{n}]");
                    } else {
                        let _ = write!(out, "\n{pad}item");
                    }
                    // vaList opens the item's properties.
                    if self.byte()? != 1 {
                        return Err(Error(format!("a collection item at {}", self.at)));
                    }
                    out.push('\n');
                    self.properties(out, depth + 2)?;
                    let _ = write!(out, "{pad}end");
                }
                self.at += 1;
                out.push('>');
            }
            15 => {
                let f = f32::from_le_bytes(self.take(4)?.try_into().expect("four"));
                out.push_str(&number(f as f64));
            }
            16 => {
                let c = self.i64()?;
                out.push_str(&number(c as f64 / 10_000.0));
            }
            17 | 21 => {
                let f = f64::from_le_bytes(self.take(8)?.try_into().expect("eight"));
                out.push_str(&number(f));
            }
            18 => {
                let n = self.len32()?;
                let raw = self.take(
                    n.checked_mul(2)
                        .ok_or_else(|| Error("a long string".into()))?,
                )?;
                let units: Vec<u16> = raw
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_le_bytes(*c))
                    .collect();
                out.push_str(&quoted(&String::from_utf16_lossy(&units)));
            }
            20 => {
                let n = self.len32()?;
                let s = String::from_utf8_lossy(self.take(n)?).into_owned();
                out.push_str(&quoted(&s));
            }
            other => return Err(Error(format!("a value of type {other} at {}", self.at - 1))),
        }
        Ok(())
    }
}

/// A short string in the form is in the code page the form was saved in, which for every
/// Delphi before 2009 is the Windows one. Latin-1 keeps every byte as a character.
fn ansi(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|b| *b as char).collect(),
    }
}

/// `'it''s'`, as Delphi writes a string: the quote doubled, and the characters a form file
/// cannot hold literally as `#nn`.
fn quoted(s: &str) -> String {
    let mut out = String::new();
    let mut open = false;
    for ch in s.chars() {
        if (ch as u32) < 32 {
            if open {
                out.push('\'');
                open = false;
            }
            let _ = write!(out, "#{}", ch as u32);
        } else {
            if !open {
                out.push('\'');
                open = true;
            }
            if ch == '\'' {
                out.push('\'');
            }
            out.push(ch);
        }
    }
    if open {
        out.push('\'');
    }
    if out.is_empty() {
        out.push_str("''");
    }
    out
}

fn number(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{f:.1}")
    } else {
        format!("{f}")
    }
}

/// An 80-bit x87 extended, which is how Delphi stores a `Double` property in a form.
fn extended(raw: [u8; 10]) -> f64 {
    let mantissa = u64::from_le_bytes(raw[..8].try_into().expect("eight"));
    let se = u16::from_le_bytes([raw[8], raw[9]]);
    let sign = if se & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = (se & 0x7FFF) as i32;
    if exp == 0 && mantissa == 0 {
        return 0.0 * sign;
    }
    sign * (mantissa as f64) * 2f64.powi(exp - 16383 - 63)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ss(out: &mut Vec<u8>, s: &str) {
        out.push(s.len() as u8);
        out.extend_from_slice(s.as_bytes());
    }

    /// A small form as `TWriter` writes it: a form with a caption, a size, a font style set,
    /// a button inside it, a string with a quote and a collection.
    fn sample() -> Vec<u8> {
        let mut b = b"TPF0".to_vec();
        ss(&mut b, "TForm1");
        ss(&mut b, "Form1");
        ss(&mut b, "Left");
        b.push(3);
        b.extend_from_slice(&300i16.to_le_bytes());
        ss(&mut b, "Caption");
        b.push(6);
        ss(&mut b, "It's mine");
        ss(&mut b, "Font.Style");
        b.push(11);
        ss(&mut b, "fsBold");
        ss(&mut b, "fsItalic");
        b.push(0);
        ss(&mut b, "Visible");
        b.push(9);
        ss(&mut b, "Columns");
        b.push(14);
        b.push(1);
        ss(&mut b, "Width");
        b.push(2);
        b.push(64);
        b.push(0);
        b.push(0);
        b.push(0); // end of the form's properties
                   // A child, inherited, with a WideString caption.
        b.push(0xF1);
        ss(&mut b, "TButton");
        ss(&mut b, "OK");
        ss(&mut b, "Caption");
        b.push(18);
        b.extend_from_slice(&2i32.to_le_bytes());
        for u in "Sì".encode_utf16() {
            b.extend_from_slice(&u.to_le_bytes());
        }
        b.push(0); // end of the button's properties
        b.push(0); // no children of the button
        b.push(0); // no more children of the form
        b
    }

    #[test]
    fn a_binary_form_reads_as_the_text_delphi_shows() {
        let bytes = sample();
        assert!(is_binary_form(&bytes));
        assert_eq!(class_name(&bytes).as_deref(), Some("TForm1"));
        let text = to_text(&bytes).unwrap();
        assert_eq!(
            text,
            "object Form1: TForm1\n  Left = 300\n  Caption = 'It''s mine'\n  Font.Style = [fsBold, fsItalic]\n  Visible = True\n  Columns = <\n    item\n      Width = 64\n    end>\n  inherited OK: TButton\n    Caption = 'Sì'\n  end\nend\n"
        );
    }

    /// `F3` on a binary `.dfm` opens on its text; `F4` still reaches the bytes.
    #[test]
    fn the_viewer_opens_a_binary_form_as_its_text() {
        let src = crate::source::BytesSource::new(sample());
        let mut v = crate::view::Viewer::new(Box::new(src), fcmd_core::VPath::local("Unit1.dfm"));
        assert_eq!(v.derived.as_deref(), Some("form"));
        let rows = v.rows(80, 5);
        assert_eq!(rows[0].text, "object Form1: TForm1");
        // And a program with a form inside: the form at its offset.
        let mut exe = b"MZ not really a program ".to_vec();
        let at = exe.len() as u64;
        exe.extend_from_slice(&sample());
        let mut v = crate::view::Viewer::new(
            Box::new(crate::source::BytesSource::new(exe)),
            fcmd_core::VPath::local("Project1.exe"),
        );
        assert!(v.derived.is_none());
        assert!(!v.form_at(0), "not a form there");
        assert!(v.form_at(at));
        assert_eq!(v.rows(80, 5)[0].text, "object Form1: TForm1");
    }

    #[test]
    fn a_form_cut_short_or_not_a_form_says_so() {
        let bytes = sample();
        assert!(to_text(&bytes[..bytes.len() - 3]).is_err());
        assert!(to_text(b"object Form1: TForm1\nend\n").is_err());
        assert_eq!(quoted("a\r\nb"), "'a'#13#10'b'");
        assert_eq!(extended([0, 0, 0, 0, 0, 0, 0, 0x80, 0xFF, 0x3F]), 1.0);
    }
}

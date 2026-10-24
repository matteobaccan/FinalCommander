//! The calculator (`docs/02-features.md` §5b, Hiew's own): one line in, one number out, in
//! every base a person patching a binary needs to see it in.
//!
//! Sixty-four bits, signed, with the operators C uses and the literals every disassembler
//! prints: `0x1f`, `1Fh`, `0b1010`, `0o17`, `31`, and `'A'` for a character. Everything is
//! evaluated in 128 bits and then narrowed, so an overflow is a result that wrapped rather
//! than a panic.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CalcError {
    #[error("nothing to work out")]
    Empty,
    #[error("{0}: not a number")]
    NotANumber(String),
    #[error("expected {0}")]
    Expected(&'static str),
    #[error("{0} is not an operator here")]
    Unexpected(char),
    #[error("divide by zero")]
    DivideByZero,
}

/// What one line of the calculator came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Answer(pub i64);

impl Answer {
    /// The same number as a disassembler, a dialog and a shell would each want it.
    pub fn lines(&self) -> [String; 5] {
        let v = self.0;
        let u = v as u64;
        [
            format!("{v}"),
            format!("0x{u:X}"),
            format!("0b{u:b}"),
            format!("0o{u:o}"),
            Self::as_characters(u),
        ]
    }

    /// The bytes of the value, low one first, as characters — which is how a four-byte
    /// signature in a header is read back.
    fn as_characters(value: u64) -> String {
        let bytes = value.to_le_bytes();
        let last = bytes
            .iter()
            .rposition(|b| *b != 0)
            .map(|i| i + 1)
            .unwrap_or(1);
        let shown: String = bytes[..last]
            .iter()
            .map(|b| {
                if (0x20..0x7f).contains(b) {
                    *b as char
                } else {
                    '.'
                }
            })
            .collect();
        format!("'{shown}'")
    }
}

impl fmt::Display for Answer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [dec, hex, ..] = self.lines();
        write!(f, "{dec}  {hex}")
    }
}

/// Work out one expression.
pub fn evaluate(text: &str) -> Result<Answer, CalcError> {
    let mut parser = Parser {
        chars: text.chars().collect(),
        at: 0,
    };
    parser.skip_spaces();
    if parser.at >= parser.chars.len() {
        return Err(CalcError::Empty);
    }
    let value = parser.expression(0)?;
    parser.skip_spaces();
    if parser.at < parser.chars.len() {
        return Err(CalcError::Unexpected(parser.chars[parser.at]));
    }
    Ok(Answer(value as i64))
}

struct Parser {
    chars: Vec<char>,
    at: usize,
}

impl Parser {
    fn skip_spaces(&mut self) {
        while self.at < self.chars.len() && self.chars[self.at].is_whitespace() {
            self.at += 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.skip_spaces();
        self.chars.get(self.at).copied()
    }

    /// Precedence climbing: the same table C has, so an expression copied out of a header
    /// means here what it means there.
    fn expression(&mut self, least: u8) -> Result<i128, CalcError> {
        let mut left = self.unary()?;
        loop {
            let Some((op, power, width)) = self.operator() else {
                break;
            };
            if power < least {
                break;
            }
            self.at += width;
            let right = self.expression(power + 1)?;
            left = match op {
                Op::Add => left.wrapping_add(right),
                Op::Sub => left.wrapping_sub(right),
                Op::Mul => left.wrapping_mul(right),
                Op::Div => {
                    if right == 0 {
                        return Err(CalcError::DivideByZero);
                    }
                    left.wrapping_div(right)
                }
                Op::Rem => {
                    if right == 0 {
                        return Err(CalcError::DivideByZero);
                    }
                    left.wrapping_rem(right)
                }
                Op::And => left & right,
                Op::Or => left | right,
                Op::Xor => left ^ right,
                // A shift of more than sixty-four is zero, not a panic.
                Op::Shl => {
                    if (0..64).contains(&right) {
                        ((left as i64).wrapping_shl(right as u32)) as i128
                    } else {
                        0
                    }
                }
                Op::Shr => {
                    if (0..64).contains(&right) {
                        ((left as i64).wrapping_shr(right as u32)) as i128
                    } else {
                        0
                    }
                }
            };
        }
        Ok(left)
    }

    fn operator(&mut self) -> Option<(Op, u8, usize)> {
        self.skip_spaces();
        let c = *self.chars.get(self.at)?;
        let next = self.chars.get(self.at + 1).copied();
        Some(match (c, next) {
            ('<', Some('<')) => (Op::Shl, 4, 2),
            ('>', Some('>')) => (Op::Shr, 4, 2),
            ('|', _) => (Op::Or, 1, 1),
            ('^', _) => (Op::Xor, 2, 1),
            ('&', _) => (Op::And, 3, 1),
            ('+', _) => (Op::Add, 5, 1),
            ('-', _) => (Op::Sub, 5, 1),
            ('*', _) => (Op::Mul, 6, 1),
            ('/', _) => (Op::Div, 6, 1),
            ('%', _) => (Op::Rem, 6, 1),
            _ => return None,
        })
    }

    fn unary(&mut self) -> Result<i128, CalcError> {
        match self.peek() {
            Some('-') => {
                self.at += 1;
                Ok(self.unary()?.wrapping_neg())
            }
            Some('+') => {
                self.at += 1;
                self.unary()
            }
            Some('~') => {
                self.at += 1;
                Ok(!(self.unary()? as i64) as i128)
            }
            Some('(') => {
                self.at += 1;
                let inside = self.expression(0)?;
                match self.peek() {
                    Some(')') => {
                        self.at += 1;
                        Ok(inside)
                    }
                    _ => Err(CalcError::Expected(")")),
                }
            }
            Some('\'') => self.character(),
            Some(_) => self.number(),
            None => Err(CalcError::Expected("a number")),
        }
    }

    /// `'A'`, or `'PE\0\0'`-style runs: the bytes, low one first, as a disassembler reads a
    /// signature.
    fn character(&mut self) -> Result<i128, CalcError> {
        self.at += 1; // the opening quote
        let mut value: u64 = 0;
        let mut shift = 0;
        while let Some(c) = self.chars.get(self.at).copied() {
            self.at += 1;
            if c == '\'' {
                return Ok(value as i128);
            }
            if shift < 64 {
                value |= ((c as u32 & 0xff) as u64) << shift;
                shift += 8;
            }
        }
        Err(CalcError::Expected("a closing quote"))
    }

    fn number(&mut self) -> Result<i128, CalcError> {
        self.skip_spaces();
        let start = self.at;
        while self
            .chars
            .get(self.at)
            .map(|c| c.is_ascii_alphanumeric() || *c == '_')
            .unwrap_or(false)
        {
            self.at += 1;
        }
        let text: String = self.chars[start..self.at].iter().collect();
        if text.is_empty() {
            return match self.chars.get(self.at) {
                Some(c) => Err(CalcError::Unexpected(*c)),
                None => Err(CalcError::Expected("a number")),
            };
        }
        parse_number(&text).ok_or(CalcError::NotANumber(text))
    }
}

#[derive(Debug, Clone, Copy)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

/// Every spelling of a number a binary tool prints: `0x1f`, `1Fh`, `0b1010`, `0o17`, `017`
/// is *not* octal (that trap belongs to C, not to a calculator a person types into), and
/// underscores anywhere.
fn parse_number(text: &str) -> Option<i128> {
    let clean: String = text.chars().filter(|c| *c != '_').collect();
    let lower = clean.to_ascii_lowercase();
    let parse = |digits: &str, radix: u32| -> Option<i128> {
        if digits.is_empty() {
            return None;
        }
        u64::from_str_radix(digits, radix).ok().map(|v| v as i128)
    };
    if let Some(rest) = lower.strip_prefix("0x") {
        return parse(rest, 16);
    }
    if let Some(rest) = lower.strip_prefix("0b") {
        return parse(rest, 2);
    }
    if let Some(rest) = lower.strip_prefix("0o") {
        return parse(rest, 8);
    }
    // `1Fh` and `1Fb`: the suffix forms a disassembler prints.
    if let Some(rest) = lower.strip_suffix('h') {
        return parse(rest, 16);
    }
    if lower.len() > 1 {
        if let Some(rest) = lower.strip_suffix('b') {
            if rest.chars().all(|c| c == '0' || c == '1') {
                return parse(rest, 2);
            }
        }
    }
    parse(&lower, 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str) -> i64 {
        evaluate(text).unwrap_or_else(|e| panic!("{text}: {e}")).0
    }

    #[test]
    fn the_numbers_a_binary_tool_prints_all_parse() {
        assert_eq!(value("31"), 31);
        assert_eq!(value("0x1f"), 31);
        assert_eq!(value("1Fh"), 31);
        assert_eq!(value("0b11111"), 31);
        assert_eq!(value("11111b"), 31);
        assert_eq!(value("0o37"), 31);
        assert_eq!(value("0xdead_beef"), 0xdeadbeef);
        // A leading zero is not octal here: nobody typing into a calculator means that.
        assert_eq!(value("017"), 17);
        assert_eq!(value("'A'"), 65);
        assert_eq!(value("'PE'"), 0x4550);
    }

    #[test]
    fn the_operators_mean_what_they_mean_in_c() {
        assert_eq!(value("2 + 3 * 4"), 14);
        assert_eq!(value("(2 + 3) * 4"), 20);
        assert_eq!(value("1 << 10"), 1024);
        assert_eq!(value("0xff & 0x0f"), 0x0f);
        assert_eq!(value("0xf0 | 0x0f"), 0xff);
        assert_eq!(value("0xff ^ 0x0f"), 0xf0);
        assert_eq!(value("~0"), -1);
        assert_eq!(value("-5 + 2"), -3);
        assert_eq!(value("17 % 5"), 2);
        assert_eq!(value("100 / 7"), 14);
        // Bitwise binds looser than arithmetic, as in C.
        assert_eq!(value("1 | 2 + 4"), 7);
    }

    #[test]
    fn an_offset_sum_is_the_thing_this_is_for() {
        // The question a hex editor asks all day: where does this field live?
        assert_eq!(value("0x400 + 0x1c"), 0x41c);
        assert_eq!(value("(0x41c - 0x400) / 4"), 7);
        assert_eq!(
            value("0x7fffffffffffffff + 1"),
            i64::MIN,
            "it wraps, it does not panic"
        );
    }

    #[test]
    fn an_answer_is_shown_in_every_base_at_once() {
        let answer = evaluate("0x4550").unwrap();
        let [dec, hex, bin, oct, chars] = answer.lines();
        assert_eq!(dec, "17744");
        assert_eq!(hex, "0x4550");
        assert_eq!(bin, "0b100010101010000");
        assert_eq!(oct, "0o42520");
        assert_eq!(
            chars, "'PE'",
            "the bytes low one first, as a signature reads"
        );

        assert_eq!(evaluate("65").unwrap().lines()[4], "'A'");
        assert_eq!(evaluate("0").unwrap().lines()[4], "'.'");
    }

    #[test]
    fn what_is_wrong_says_what_is_wrong() {
        assert_eq!(evaluate(""), Err(CalcError::Empty));
        assert_eq!(evaluate("   "), Err(CalcError::Empty));
        assert_eq!(evaluate("1 / 0"), Err(CalcError::DivideByZero));
        assert_eq!(evaluate("1 % 0"), Err(CalcError::DivideByZero));
        assert_eq!(evaluate("(1 + 2"), Err(CalcError::Expected(")")));
        assert!(matches!(evaluate("0xzz"), Err(CalcError::NotANumber(_))));
        assert!(matches!(
            evaluate("1 2"),
            Err(CalcError::NotANumber(_)) | Err(CalcError::Unexpected(_))
        ));
        assert!(matches!(evaluate("@"), Err(CalcError::Unexpected('@'))));
    }
}

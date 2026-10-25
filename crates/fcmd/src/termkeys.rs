//! A key press as the bytes a terminal would have sent.
//!
//! The program on the other end of a pseudo-terminal does not receive key presses; it reads
//! bytes, and works out what was pressed from them. That mapping is a convention — xterm's,
//! as everything since has copied it — and this is our side of it: `Up` is `ESC [ A`,
//! `Ctrl+C` is byte 3, `Alt+x` is `ESC x`, and a letter is itself.
//!
//! It is a function from a key to bytes with no state, which is the whole reason it is in a
//! file of its own: every one of these can be checked by reading the table it comes from.

use fcmd_cmd::{Key, KeyCode, Mods};

/// The bytes a terminal sends for this key, or `None` for a key it has no spelling for.
pub fn bytes_for(key: Key) -> Option<Vec<u8>> {
    let ctrl = key.mods.contains(Mods::CTRL);
    let alt = key.mods.contains(Mods::ALT);
    let shift = key.mods.contains(Mods::SHIFT);

    let body: Vec<u8> = match key.code {
        KeyCode::Char(c) => {
            if ctrl {
                // The control characters: `Ctrl+A` is 1 and `Ctrl+Z` is 26, which is the
                // letter's position in the alphabet, and the handful below `@` and above `Z`
                // that complete the block.
                let byte = match c.to_ascii_lowercase() {
                    'a'..='z' => (c.to_ascii_lowercase() as u8) - b'a' + 1,
                    '@' | ' ' => 0,
                    '[' => 27,
                    '\\' => 28,
                    ']' => 29,
                    '^' => 30,
                    '_' | '/' => 31,
                    '?' => 127,
                    _ => return None,
                };
                vec![byte]
            } else {
                let mut out = [0u8; 4];
                c.encode_utf8(&mut out).as_bytes().to_vec()
            }
        }
        KeyCode::Enter => vec![b'\r'],
        // `Shift+Tab` is its own sequence rather than a tab with a modifier byte, and it
        // arrives both ways: as `BackTab` from a terminal that distinguishes them, and as a
        // shifted `Tab` from one that does not.
        KeyCode::BackTab => return Some(b"\x1b[Z".to_vec()),
        KeyCode::Tab if shift => return Some(b"\x1b[Z".to_vec()),
        KeyCode::Tab => vec![b'\t'],
        // The one key everybody disagrees about. `DEL` is what a terminal with
        // `stty erase ^?` expects, which is every Linux and macOS default; a program that
        // wants `BS` asks for it through `stty`, and gets it from the shell it is under.
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        // The arrows and the navigation block. `ESC [ 1 ; m X` is the modified form, where
        // `m` is 1 plus the modifier bits — shift 1, alt 2, control 4 — which is the
        // encoding every terminal has used since xterm's.
        KeyCode::Up => return Some(arrow(b'A', shift, alt, ctrl)),
        KeyCode::Down => return Some(arrow(b'B', shift, alt, ctrl)),
        KeyCode::Right => return Some(arrow(b'C', shift, alt, ctrl)),
        KeyCode::Left => return Some(arrow(b'D', shift, alt, ctrl)),
        KeyCode::Home => return Some(arrow(b'H', shift, alt, ctrl)),
        KeyCode::End => return Some(arrow(b'F', shift, alt, ctrl)),
        KeyCode::Insert => return Some(tilde(2, shift, alt, ctrl)),
        KeyCode::Delete => return Some(tilde(3, shift, alt, ctrl)),
        KeyCode::PageUp => return Some(tilde(5, shift, alt, ctrl)),
        KeyCode::PageDown => return Some(tilde(6, shift, alt, ctrl)),
        KeyCode::F(n) => return function_key(n, shift, alt, ctrl),
    };

    // `Alt` is an `ESC` in front of whatever the key would otherwise send, which is how a
    // terminal with no modifier byte to spare has always carried it.
    if alt {
        let mut out = vec![0x1b];
        out.extend_from_slice(&body);
        return Some(out);
    }
    Some(body)
}

/// 1 plus the modifier bits, as the `;m` of a modified sequence.
fn modifier(shift: bool, alt: bool, ctrl: bool) -> u8 {
    1 + u8::from(shift) + 2 * u8::from(alt) + 4 * u8::from(ctrl)
}

/// `ESC [ A` plain, `ESC [ 1 ; 5 A` with a modifier.
fn arrow(last: u8, shift: bool, alt: bool, ctrl: bool) -> Vec<u8> {
    let m = modifier(shift, alt, ctrl);
    if m == 1 {
        vec![0x1b, b'[', last]
    } else {
        format!("\x1b[1;{m}{}", last as char).into_bytes()
    }
}

/// `ESC [ n ~`, the form the navigation keys and the later function keys take.
fn tilde(n: u8, shift: bool, alt: bool, ctrl: bool) -> Vec<u8> {
    let m = modifier(shift, alt, ctrl);
    if m == 1 {
        format!("\x1b[{n}~").into_bytes()
    } else {
        format!("\x1b[{n};{m}~").into_bytes()
    }
}

/// The function keys, which are two different shapes: `F1`–`F4` are `ESC O P`..`S`, and the
/// rest are `ESC [ n ~` with a number that is not the key's own.
fn function_key(n: u8, shift: bool, alt: bool, ctrl: bool) -> Option<Vec<u8>> {
    let m = modifier(shift, alt, ctrl);
    match n {
        1..=4 => {
            let last = b"PQRS"[(n - 1) as usize];
            Some(if m == 1 {
                vec![0x1b, b'O', last]
            } else {
                format!("\x1b[1;{m}{}", last as char).into_bytes()
            })
        }
        // 5 is 15, and the numbers skip 16, 22, 23 and 27 — a table rather than a formula,
        // because the real one has gaps in it.
        5..=12 => {
            let code = [15, 17, 18, 19, 20, 21, 23, 24][(n - 5) as usize];
            Some(tilde(code, shift, alt, ctrl))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent(key: &str) -> Vec<u8> {
        bytes_for(key.parse().expect("a key")).unwrap_or_default()
    }

    #[test]
    fn a_letter_is_itself_and_a_control_letter_is_its_place_in_the_alphabet() {
        assert_eq!(sent("a"), b"a");
        // A key name of one letter carries no shift, so an upper-case letter is built rather
        // than parsed: `"Z".parse()` is the `z` key.
        assert_eq!(bytes_for(Key::char('Z')).unwrap(), b"Z");
        assert_eq!(sent("Ctrl+a"), vec![1]);
        assert_eq!(sent("Ctrl+c"), vec![3], "the one everybody knows");
        assert_eq!(sent("Ctrl+d"), vec![4], "end of input");
        assert_eq!(sent("Ctrl+z"), vec![26]);
        // The ones that complete the control block.
        assert_eq!(sent("Ctrl+["), vec![27]);
        assert_eq!(sent("Ctrl+\\"), vec![28]);
    }

    #[test]
    fn a_character_outside_ascii_goes_as_its_utf8() {
        assert_eq!(sent("è"), "è".as_bytes());
        assert_eq!(sent("漢"), "漢".as_bytes());
    }

    #[test]
    fn alt_puts_an_escape_in_front() {
        assert_eq!(sent("Alt+b"), b"\x1bb", "a word back, in every shell");
        assert_eq!(sent("Alt+f"), b"\x1bf");
        assert_eq!(sent("Alt+Enter"), b"\x1b\r");
    }

    #[test]
    fn the_keys_with_no_character_have_the_sequences_everything_expects() {
        assert_eq!(sent("Enter"), b"\r");
        assert_eq!(sent("Tab"), b"\t");
        assert_eq!(sent("Shift+Tab"), b"\x1b[Z");
        assert_eq!(sent("Esc"), b"\x1b");
        // DEL, not BS: what `stty erase ^?` expects, which is every modern default.
        assert_eq!(sent("Backspace"), vec![0x7f]);
    }

    #[test]
    fn the_arrows_are_plain_until_a_modifier_is_held() {
        assert_eq!(sent("Up"), b"\x1b[A");
        assert_eq!(sent("Down"), b"\x1b[B");
        assert_eq!(sent("Right"), b"\x1b[C");
        assert_eq!(sent("Left"), b"\x1b[D");
        assert_eq!(sent("Home"), b"\x1b[H");
        assert_eq!(sent("End"), b"\x1b[F");
        // 1 plus the bits: shift 1, alt 2, control 4.
        assert_eq!(sent("Shift+Up"), b"\x1b[1;2A");
        assert_eq!(sent("Alt+Up"), b"\x1b[1;3A");
        assert_eq!(sent("Ctrl+Up"), b"\x1b[1;5A");
        assert_eq!(sent("Ctrl+Right"), b"\x1b[1;5C", "a word right, in a shell");
    }

    #[test]
    fn the_navigation_block_is_the_tilde_form() {
        assert_eq!(sent("Ins"), b"\x1b[2~");
        assert_eq!(sent("Del"), b"\x1b[3~");
        assert_eq!(sent("PageUp"), b"\x1b[5~");
        assert_eq!(sent("PageDown"), b"\x1b[6~");
        assert_eq!(sent("Ctrl+Del"), b"\x1b[3;5~");
    }

    #[test]
    fn the_function_keys_are_two_shapes_with_gaps_in_the_numbers() {
        assert_eq!(sent("F1"), b"\x1bOP");
        assert_eq!(sent("F4"), b"\x1bOS");
        assert_eq!(sent("F5"), b"\x1b[15~");
        // 16 is skipped, which is why this is a table and not a sum.
        assert_eq!(sent("F6"), b"\x1b[17~");
        assert_eq!(sent("F10"), b"\x1b[21~");
        assert_eq!(sent("F12"), b"\x1b[24~");
        assert_eq!(sent("Shift+F1"), b"\x1b[1;2P");
    }

    #[test]
    fn a_key_with_no_spelling_sends_nothing_rather_than_something_wrong() {
        // `Ctrl+1` is not a control character, and inventing one would type a digit into a
        // program that asked for a key.
        assert_eq!(bytes_for(Key::new(KeyCode::Char('1'), Mods::CTRL)), None);
        assert_eq!(bytes_for(Key::plain(KeyCode::F(20))), None);
    }
}

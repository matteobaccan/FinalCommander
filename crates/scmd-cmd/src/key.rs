use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Mods(u8);

impl Mods {
    pub const NONE: Mods = Mods(0);
    pub const SHIFT: Mods = Mods(1 << 0);
    pub const CTRL: Mods = Mods(1 << 1);
    pub const ALT: Mods = Mods(1 << 2);

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    pub const fn bits(self) -> u8 {
        self.0
    }
}

impl std::ops::BitOr for Mods {
    type Output = Mods;

    fn bitor(self, rhs: Mods) -> Mods {
        Mods(self.0 | rhs.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KeyCode {
    Char(char),
    F(u8),
    Enter,
    Esc,
    Tab,
    BackTab,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key {
    pub code: KeyCode,
    pub mods: Mods,
}

impl Key {
    pub const fn new(code: KeyCode, mods: Mods) -> Self {
        Key { code, mods }
    }

    pub const fn plain(code: KeyCode) -> Self {
        Key {
            code,
            mods: Mods::NONE,
        }
    }

    pub fn char(c: char) -> Self {
        Key::plain(KeyCode::Char(c))
    }

    /// Keymaps are written in lowercase: `Shift+A` and `A` must resolve to the same binding.
    pub fn normalized(self) -> Key {
        match self.code {
            // Terminals are inconsistent about reporting Shift for letters, so case is
            // folded and the modifiers are left exactly as they came in.
            KeyCode::Char(c) if c.is_uppercase() => Key {
                code: KeyCode::Char(c.to_lowercase().next().unwrap_or(c)),
                mods: self.mods,
            },
            _ => self,
        }
    }

    /// A key that types a character into an input line: no Ctrl/Alt, any Shift.
    pub fn as_typed_char(self) -> Option<char> {
        match self.code {
            KeyCode::Char(c)
                if !self.mods.contains(Mods::CTRL) && !self.mods.contains(Mods::ALT) =>
            {
                Some(c)
            }
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum KeyParseError {
    #[error("empty key")]
    Empty,
    #[error("unknown modifier `{0}`")]
    UnknownModifier(String),
    #[error("unknown key `{0}`")]
    UnknownKey(String),
}

impl FromStr for Key {
    type Err = KeyParseError;

    /// `"F10"`, `"Ctrl+Shift+F3"`, `"Alt+1"`, `"Ctrl+PgUp"`, `"+"`, `"Space"`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(KeyParseError::Empty);
        }
        // A trailing `+` is the key itself rather than a separator, which is why the split
        // happens from the right: `+`, `Ctrl++`, `Alt+Shift++`. Reading it from the left
        // instead loses the modifiers, and a `Ctrl++` that quietly means `+` is a binding
        // that fires on the wrong key.
        let (modifiers, name) = match s.strip_suffix('+') {
            Some("") => ("", "+"),
            Some(rest) => match rest.strip_suffix('+') {
                Some(mods) => (mods, "+"),
                // `Ctrl+` names a modifier and no key.
                None => return Err(KeyParseError::UnknownKey(s.to_string())),
            },
            None => match s.rsplit_once('+') {
                Some((mods, name)) => (mods, name),
                None => ("", s),
            },
        };

        let mut mods = Mods::NONE;
        for m in modifiers.split('+').filter(|m| !m.trim().is_empty()) {
            match m.trim().to_ascii_lowercase().as_str() {
                "ctrl" | "c" => mods.insert(Mods::CTRL),
                "shift" | "s" => mods.insert(Mods::SHIFT),
                "alt" | "a" | "meta" => mods.insert(Mods::ALT),
                other => return Err(KeyParseError::UnknownModifier(other.to_string())),
            }
        }

        let lower = name.trim().to_ascii_lowercase();
        let code = match lower.as_str() {
            "enter" | "return" => KeyCode::Enter,
            "esc" | "escape" => KeyCode::Esc,
            "tab" => KeyCode::Tab,
            "backtab" => KeyCode::BackTab,
            "backspace" | "bs" => KeyCode::Backspace,
            "del" | "delete" => KeyCode::Delete,
            "ins" | "insert" => KeyCode::Insert,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pgup" | "pageup" => KeyCode::PageUp,
            "pgdn" | "pagedown" => KeyCode::PageDown,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "space" => KeyCode::Char(' '),
            _ => {
                // `F1`..`F24` are function keys; a bare `F` is the letter.
                let fkey = lower
                    .strip_prefix('f')
                    .filter(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()));
                if let Some(num) = fkey {
                    match num.parse::<u8>() {
                        Ok(n) if (1..=24).contains(&n) => KeyCode::F(n),
                        _ => return Err(KeyParseError::UnknownKey(name.to_string())),
                    }
                } else {
                    let mut chars = name.trim().chars();
                    match (chars.next(), chars.next()) {
                        (Some(c), None) => KeyCode::Char(c.to_ascii_lowercase()),
                        _ => return Err(KeyParseError::UnknownKey(name.to_string())),
                    }
                }
            }
        };
        Ok(Key { code, mods })
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mods.contains(Mods::CTRL) {
            write!(f, "Ctrl+")?;
        }
        if self.mods.contains(Mods::ALT) {
            write!(f, "Alt+")?;
        }
        if self.mods.contains(Mods::SHIFT) {
            write!(f, "Shift+")?;
        }
        match self.code {
            KeyCode::Char(' ') => write!(f, "Space"),
            KeyCode::Char(c) => write!(f, "{}", c.to_uppercase()),
            KeyCode::F(n) => write!(f, "F{n}"),
            KeyCode::Enter => write!(f, "Enter"),
            KeyCode::Esc => write!(f, "Esc"),
            KeyCode::Tab => write!(f, "Tab"),
            KeyCode::BackTab => write!(f, "BackTab"),
            KeyCode::Backspace => write!(f, "Backspace"),
            KeyCode::Delete => write!(f, "Del"),
            KeyCode::Insert => write!(f, "Ins"),
            KeyCode::Home => write!(f, "Home"),
            KeyCode::End => write!(f, "End"),
            KeyCode::PageUp => write!(f, "PgUp"),
            KeyCode::PageDown => write!(f, "PgDn"),
            KeyCode::Up => write!(f, "Up"),
            KeyCode::Down => write!(f, "Down"),
            KeyCode::Left => write!(f, "Left"),
            KeyCode::Right => write!(f, "Right"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_function_keys_with_modifiers() {
        let k: Key = "Ctrl+Shift+F3".parse().unwrap();
        assert_eq!(k.code, KeyCode::F(3));
        assert!(k.mods.contains(Mods::CTRL) && k.mods.contains(Mods::SHIFT));
        assert!(!k.mods.contains(Mods::ALT));
    }

    #[test]
    fn parses_names_and_chars_case_insensitively() {
        assert_eq!("tab".parse::<Key>().unwrap(), Key::plain(KeyCode::Tab));
        assert_eq!(
            "PgDn".parse::<Key>().unwrap(),
            Key::plain(KeyCode::PageDown)
        );
        assert_eq!("A".parse::<Key>().unwrap(), Key::char('a'));
        assert_eq!("Space".parse::<Key>().unwrap(), Key::char(' '));
        assert_eq!("+".parse::<Key>().unwrap(), Key::char('+'));
    }

    #[test]
    fn a_bare_f_is_the_letter_not_a_function_key() {
        assert_eq!("Alt+F".parse::<Key>().unwrap().code, KeyCode::Char('f'));
        assert_eq!("F1".parse::<Key>().unwrap().code, KeyCode::F(1));
        assert_eq!("f12".parse::<Key>().unwrap().code, KeyCode::F(12));
    }

    #[test]
    fn rejects_nonsense() {
        assert!("Hyper+F1".parse::<Key>().is_err());
        assert!("F99".parse::<Key>().is_err());
        assert!("".parse::<Key>().is_err());
    }

    #[test]
    fn the_plus_key_keeps_its_modifiers() {
        // FAR's selection matrix is built on the grey plus and minus with every modifier, so
        // `Ctrl++` has to survive the parser as plus *with* control.
        let plus: Key = "+".parse().unwrap();
        assert_eq!(plus, Key::char('+'));
        let ctrl: Key = "Ctrl++".parse().unwrap();
        assert_eq!(ctrl, Key::new(KeyCode::Char('+'), Mods::CTRL));
        assert_ne!(ctrl, plus, "the modifier was dropped");
        let both: Key = "Alt+Shift++".parse().unwrap();
        assert_eq!(both, Key::new(KeyCode::Char('+'), Mods::ALT | Mods::SHIFT));
        assert_eq!(ctrl.to_string(), "Ctrl++");
        assert_eq!(ctrl.to_string().parse::<Key>().unwrap(), ctrl);
        // A modifier with no key is a mistake in a keymap, not the plus key.
        assert!("Ctrl+".parse::<Key>().is_err());
    }

    #[test]
    fn display_round_trips() {
        for s in ["F10", "Ctrl+R", "Alt+F7", "Ctrl+Shift+F3", "Ins", "PgUp"] {
            let k: Key = s.parse().unwrap();
            assert_eq!(k.to_string().parse::<Key>().unwrap(), k, "{s}");
        }
    }
}

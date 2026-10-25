//! Percent-encoding, by the rule the signature uses rather than the one a URL library picks.
//!
//! SigV4 asks for RFC 3986 encoding of everything but the unreserved set — the letters, the
//! digits, and `-._~`. A library that leaves `+`, `=`, `:` or `@` alone because they are legal
//! somewhere in a URL produces a request the server will reject, with no clue as to why, so
//! the rule is written out here and used for both the URL and the canonical form.

/// Encode a key for the path part of a URL: everything but the unreserved set and the `/` that
/// separates the segments.
pub fn encode_path(text: &str) -> String {
    encode(text, b"/")
}

/// Encode a name or a value for the query string: `/` included, since there it is data.
pub fn encode_query(text: &str) -> String {
    encode(text, b"")
}

fn encode(text: &str, extra: &[u8]) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for byte in text.as_bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'.' | b'_' | b'~')
            || extra.contains(byte)
        {
            out.push(*byte as char);
        } else {
            out.push('%');
            out.push(
                char::from_digit((byte >> 4) as u32, 16)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
            out.push(
                char::from_digit((byte & 0xf) as u32, 16)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
        }
    }
    out
}

/// Undo it. A listing asked for with `encoding-type=url` comes back with every key encoded,
/// which is the only way a key holding a control character, a newline or an ampersand survives
/// the XML it arrives in.
///
/// A stray `%` or a half-written escape is left as it stands rather than dropped: it is not
/// this layer's business to decide that a key is malformed.
pub fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The query string a request carries: the pairs sorted and encoded, which is both what goes
/// on the wire and what gets signed. One string, so the two cannot drift apart.
pub fn query_string(pairs: &[(&str, String)]) -> String {
    let mut encoded: Vec<(String, String)> = pairs
        .iter()
        .map(|(k, v)| (encode_query(k), encode_query(v)))
        .collect();
    encoded.sort();
    encoded
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_encoded_once_and_keeps_its_slashes() {
        assert_eq!(encode_path("a/b.txt"), "a/b.txt");
        assert_eq!(
            encode_path("folder name/ünïcode & special+chars.txt"),
            "folder%20name/%C3%BCn%C3%AFcode%20%26%20special%2Bchars.txt"
        );
        // Already-encoded text is not spared: the caller holds keys, not URLs.
        assert_eq!(encode_path("a%2Fb"), "a%252Fb");
        assert_eq!(encode_path("~-._"), "~-._");
        assert_eq!(encode_path("a\nb\tc"), "a%0Ab%09c");
        assert_eq!(encode_path("ключ"), "%D0%BA%D0%BB%D1%8E%D1%87");
    }

    #[test]
    fn a_query_value_has_no_slashes_to_keep() {
        assert_eq!(encode_query("2024/08/"), "2024%2F08%2F");
        assert_eq!(
            encode_query("1ueGcxLPRx1Tr/XYExHnhbYLgveDs2J/wm36Hy4vbOwM="),
            "1ueGcxLPRx1Tr%2FXYExHnhbYLgveDs2J%2Fwm36Hy4vbOwM%3D"
        );
    }

    #[test]
    fn the_query_string_is_sorted_the_way_the_signature_wants() {
        let q = query_string(&[
            ("list-type", "2".into()),
            ("prefix", "2024/08/".into()),
            ("delimiter", "/".into()),
            ("encoding-type", "url".into()),
            ("max-keys", "1000".into()),
        ]);
        assert_eq!(
            q,
            "delimiter=%2F&encoding-type=url&list-type=2&max-keys=1000&prefix=2024%2F08%2F"
        );
    }

    #[test]
    fn what_was_encoded_comes_back() {
        for text in [
            "a/b.txt",
            "folder name/ünïcode & special+chars.txt",
            "ключ/файл.txt",
            "a\nb",
            "100%",
            "",
        ] {
            assert_eq!(decode(&encode_path(text)), text, "{text:?}");
            assert_eq!(decode(&encode_query(text)), text, "{text:?}");
        }
    }

    #[test]
    fn a_broken_escape_is_left_alone_rather_than_eaten() {
        assert_eq!(decode("100%"), "100%");
        assert_eq!(decode("50%z0"), "50%z0");
        assert_eq!(decode("a%2"), "a%2");
        assert_eq!(decode("a%2Fb%"), "a/b%");
    }
}

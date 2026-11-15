//! What a mail message is made of: who sent it, to whom, about what, and the tree of its
//! parts — for an `.eml` here, and for an Outlook `.msg` in `ole.rs`, which draws its rows
//! with the same function (SPEC 05 §1.8).
//!
//! The tree is the one `structure.txt` shows inside the message opened as a folder: the
//! reader in `fcmd-arc` builds it once and both places show it, so they cannot disagree.

use fcmd_arc::mail::{self, Node};

use crate::structure::{Bytes, Row, Structure};

/// The most of a message read for its tree. One with larger attachments than this shows the
/// parts that begin inside the window and says it stopped.
const LIMIT: u64 = 64 << 20;

pub(crate) fn read(src: &mut dyn Bytes, head: &[u8], len: u64) -> Option<Structure> {
    if !mail::looks_like_message(head) {
        return None;
    }
    let bytes = src.at(0, len.min(LIMIT) as usize);
    let headline = mail::headline(&bytes);
    let tree = mail::structure(&bytes);
    Some(shape("eml", "Mail message", &headline, &tree, len > LIMIT))
}

/// The pane of a message, whichever format it came in.
pub(crate) fn shape(
    format: &'static str,
    kind: &str,
    headline: &[(&'static str, String)],
    tree: &Node,
    truncated: bool,
) -> Structure {
    let mut rows = Vec::new();
    if !headline.is_empty() {
        rows.push(Row::group("Message"));
        for (name, value) in headline {
            rows.push(Row::field(
                name.to_ascii_lowercase(),
                String::new(),
                value.clone(),
            ));
        }
    }
    rows.push(Row::group("Structure"));
    for row in mail::tree_rows(tree)
        .into_iter()
        .take(crate::structure::MAX_ENTRIES)
    {
        rows.push(Row::entry(
            format!("  {}", row.label),
            row.detail,
            row.offset,
        ));
    }
    let subject = headline
        .iter()
        .find(|(name, _)| *name == "Subject")
        .map(|(_, v)| format!(" · {v}"))
        .unwrap_or_default();
    Structure {
        format,
        summary: format!("{kind}{subject}"),
        rows,
        symbols: Vec::new(),
        maps: Vec::new(),
        clr: None,
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use crate::structure::{self, RowKind};

    const MAIL: &str = "From: Matteo <m@example.com>\r\n\
To: anna@example.com\r\n\
Subject: =?utf-8?Q?Caff=C3=A8?=\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"XX\"\r\n\
\r\n\
--XX\r\n\
Content-Type: text/plain\r\n\
\r\n\
Ciao.\r\n\
--XX\r\n\
Content-Type: application/pdf; name=\"invoice.pdf\"\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
JVBERi0xLjQK\r\n\
--XX--\r\n";

    #[test]
    fn a_message_shows_who_wrote_it_and_the_tree_of_its_parts() {
        let s = structure::read(&mut MAIL.as_bytes()).expect("a message");
        assert_eq!(s.format, "eml");
        assert_eq!(s.summary, "Mail message · Caffè");
        let text: Vec<String> = s
            .rows
            .iter()
            .map(|r| format!("{}|{}", r.label, r.value))
            .collect();
        assert_eq!(
            text,
            [
                "Message|",
                "from|Matteo <m@example.com>",
                "to|anna@example.com",
                "subject|Caffè",
                "Structure|",
                "  multipart/mixed|",
                "  ├ text/plain|5 B",
                "  └ application/pdf|base64  9 B  \"invoice.pdf\"",
            ]
        );
        // `Enter` on a part goes to where it starts in the file.
        let pdf = s.rows.last().unwrap();
        assert_eq!(pdf.kind, RowKind::Entry);
        let at = pdf.offset.expect("a place in the file") as usize;
        assert!(MAIL[at..].starts_with("Content-Type: application/pdf"));
    }

    #[test]
    fn a_text_that_is_not_mail_has_no_pane() {
        assert!(structure::read(&mut &b"Dear all,\n\nSubject: nothing\n"[..]).is_none());
        assert!(structure::read(&mut &b"title: Notes\nauthor: me\n\nbody\n"[..]).is_none());
    }
}

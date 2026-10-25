use std::path::Path;

/// The container formats this crate reads. The list is deliberately short: the formats a
/// commander meets every day, each with a pure-Rust decoder (SPEC 04 §3 names the rest, and
/// `docs/BACKLOG.md` says when they come).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Zip,
    SevenZ,
    Tar,
    TarGz,
    TarBz2,
    TarXz,
    TarZst,
}

impl Format {
    /// Every format, for the places that offer a choice.
    pub const ALL: [Format; 7] = [
        Format::Zip,
        Format::SevenZ,
        Format::Tar,
        Format::TarGz,
        Format::TarBz2,
        Format::TarXz,
        Format::TarZst,
    ];

    /// What it is called in a dialog or a panel frame.
    pub fn label(self) -> &'static str {
        match self {
            Format::Zip => "zip",
            Format::SevenZ => "7z",
            Format::Tar => "tar",
            Format::TarGz => "tar.gz",
            Format::TarBz2 => "tar.bz2",
            Format::TarXz => "tar.xz",
            Format::TarZst => "tar.zst",
        }
    }

    /// The extension we give a new archive of this format.
    pub fn extension(self) -> &'static str {
        match self {
            Format::Zip => "zip",
            Format::SevenZ => "7z",
            Format::Tar => "tar",
            Format::TarGz => "tar.gz",
            Format::TarBz2 => "tar.bz2",
            Format::TarXz => "tar.xz",
            Format::TarZst => "tar.zst",
        }
    }

    /// Is this one of the tar family, rather than a zip or a 7z?
    pub fn is_tar(self) -> bool {
        matches!(
            self,
            Format::Tar | Format::TarGz | Format::TarBz2 | Format::TarXz | Format::TarZst
        )
    }

    /// Can one entry be read without reading the ones before it? A zip has a directory at the
    /// end and a plain tar has fixed-size headers, so both can seek; a compressed tar is one
    /// stream from start to finish, and reading the last entry means decompressing all of it.
    ///
    /// A 7z says no for the same reason as a compressed tar, and for one more: its usual
    /// setting is *solid*, where several files share one compressed block, so the bytes of one
    /// of them cannot be found without decoding what is in front of them. Its index, on the
    /// other hand, is a header read in one go — which is why listing a 7z is quick and reading
    /// the last file in one is not.
    pub fn seekable(self) -> bool {
        matches!(self, Format::Zip | Format::Tar)
    }

    /// The format a *name* suggests. Longest suffix first, so `.tar.gz` is not read as `.gz`.
    pub fn of_name(name: &str) -> Option<Format> {
        let lower = name.to_ascii_lowercase();
        const BY_SUFFIX: [(&str, Format); 13] = [
            (".tar.gz", Format::TarGz),
            (".tgz", Format::TarGz),
            (".tar.bz2", Format::TarBz2),
            (".tbz2", Format::TarBz2),
            (".tbz", Format::TarBz2),
            (".tar.xz", Format::TarXz),
            (".txz", Format::TarXz),
            (".tar.zst", Format::TarZst),
            (".tzst", Format::TarZst),
            (".tar", Format::Tar),
            (".7z", Format::SevenZ),
            (".zip", Format::Zip),
            (".jar", Format::Zip),
        ];
        BY_SUFFIX
            .iter()
            .find(|(suffix, _)| lower.ends_with(suffix))
            .map(|(_, format)| *format)
    }

    pub fn of_path(path: &Path) -> Option<Format> {
        Format::of_name(&path.file_name()?.to_string_lossy())
    }

    /// The format the *bytes* say it is. A `.docx` is a zip and a tarball someone renamed is
    /// still a tarball, so what the file starts with wins over what it is called — except for
    /// the compressed tars, where the magic says "gzip" and only the name says "tar inside".
    pub fn of_bytes(head: &[u8], name: &str) -> Option<Format> {
        let by_name = Format::of_name(name);
        if head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06") {
            return Some(Format::Zip);
        }
        if head.starts_with(&[0x1f, 0x8b]) {
            return Some(Format::TarGz);
        }
        if head.starts_with(b"BZh") {
            return Some(Format::TarBz2);
        }
        if head.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0x00]) {
            return Some(Format::TarXz);
        }
        // 7z's own signature, tested after xz's: xz's starts with one byte and then the same
        // two letters, so the order of these two is what keeps them apart.
        if head.starts_with(&[b'7', b'z', 0xbc, 0xaf, 0x27, 0x1c]) {
            return Some(Format::SevenZ);
        }
        if head.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
            return Some(Format::TarZst);
        }
        // A tar has no magic at the start: the word `ustar` sits 257 bytes in.
        if head.len() > 262 && (&head[257..262] == b"ustar") {
            return Some(Format::Tar);
        }
        by_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_names_its_format_longest_suffix_first() {
        assert_eq!(Format::of_name("src.tar.gz"), Some(Format::TarGz));
        assert_eq!(Format::of_name("src.TGZ"), Some(Format::TarGz));
        assert_eq!(Format::of_name("src.tar"), Some(Format::Tar));
        assert_eq!(Format::of_name("app.jar"), Some(Format::Zip));
        assert_eq!(Format::of_name("backup.7Z"), Some(Format::SevenZ));
        assert_eq!(Format::of_name("notes.txt"), None);
        // `.gz` on its own is a compressed file, not an archive of several.
        assert_eq!(Format::of_name("notes.txt.gz"), None);
    }

    #[test]
    fn the_bytes_win_over_the_name() {
        // A zip called something else is still a zip.
        assert_eq!(
            Format::of_bytes(b"PK\x03\x04rest", "book.epub"),
            Some(Format::Zip)
        );
        // And a name with no magic behind it is still worth something.
        assert_eq!(Format::of_bytes(b"", "src.tar.xz"), Some(Format::TarXz));
        assert_eq!(Format::of_bytes(b"not an archive", "notes.txt"), None);

        assert_eq!(
            Format::of_bytes(
                &[b'7', b'z', 0xbc, 0xaf, 0x27, 0x1c, 0x00, 0x04],
                "whatever"
            ),
            Some(Format::SevenZ)
        );

        let mut tar = vec![0u8; 512];
        tar[257..262].copy_from_slice(b"ustar");
        assert_eq!(Format::of_bytes(&tar, "whatever"), Some(Format::Tar));
    }

    #[test]
    fn only_the_two_that_can_be_read_out_of_order_say_they_can() {
        for f in Format::ALL {
            assert_eq!(
                f.seekable(),
                matches!(f, Format::Zip | Format::Tar),
                "{}",
                f.label()
            );
        }
    }
}

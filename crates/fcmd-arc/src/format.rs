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
    /// The Unix archive: `.a`, and the outer layer of a `.deb`. Read only — see
    /// [`Format::writable`].
    Ar,
    /// `cpio`, in its newc, crc and odc headers: an initramfs, and what an `rpm` carries.
    /// Read only, like `ar`.
    Cpio,
    /// A Red Hat package: a header, and a compressed newc cpio of the files it installs.
    /// Read only — `rpmbuild` makes them, and a package written without its header would be
    /// a file that looks like one and is not.
    Rpm,
    /// A single compressed stream: `notes.txt.gz` and nothing else in it.
    ///
    /// These four are the same codecs as the compressed tars above, without the tar: one
    /// file went in, one file comes out, and there is no index, no name and no directory
    /// anywhere in the format. A commander still has to let somebody *in* — the thing a
    /// reader wants from a `.gz` is to look at what is inside it — so they are archives of
    /// one entry, which is what FAR, Midnight Commander and Total Commander all present.
    /// Read only for now: see [`Format::writable`] and [`Format::holds_many`].
    Gz,
    Bz2,
    Xz,
    Zst,
}

impl Format {
    /// Every format this crate knows, read or write.
    pub const ALL: [Format; 14] = [
        Format::Zip,
        Format::SevenZ,
        Format::Tar,
        Format::TarGz,
        Format::TarBz2,
        Format::TarXz,
        Format::TarZst,
        Format::Ar,
        Format::Cpio,
        Format::Rpm,
        Format::Gz,
        Format::Bz2,
        Format::Xz,
        Format::Zst,
    ];

    /// The ones a dialog may offer to *create*.
    ///
    /// Not every format this crate reads is one it writes, and a list of formats offered for a
    /// new archive has to be the writable ones or it is a promise it cannot keep. The one
    /// place that offers a choice reads this; [`Format::ALL`] is for the places that have to
    /// cover every format, such as a test that walks them.
    pub fn writable_formats() -> Vec<Format> {
        Format::ALL.into_iter().filter(|f| f.writable()).collect()
    }

    /// Can an archive of this format be created and written into?
    ///
    /// `ar` cannot, and deliberately: nothing makes a `.a` or a `.deb` by hand any more — `ar`
    /// and `dpkg-deb` do, and a `.deb` written without its `control.tar` would be a file that
    /// looks like a package and is not one. Reading them is what a commander is for.
    ///
    /// The four single streams cannot either, for now: compressing one file into a `.gz` is a
    /// useful thing to offer and a different piece of work from reading one, because every
    /// dialog and every rebuild in this crate assumes a format that can hold a second entry.
    pub fn writable(self) -> bool {
        !matches!(
            self,
            Format::Ar
                | Format::Cpio
                | Format::Rpm
                | Format::Gz
                | Format::Bz2
                | Format::Xz
                | Format::Zst
        )
    }

    /// Is this a single compressed stream — one file in, one file out, no index?
    ///
    /// The question every other part of the crate asks differently: there is no second entry
    /// to add, no entry to delete that is not the whole file, and no directory at all.
    pub fn is_single(self) -> bool {
        matches!(self, Format::Gz | Format::Bz2 | Format::Xz | Format::Zst)
    }

    /// Can this format hold more than one file?
    pub fn holds_many(self) -> bool {
        !self.is_single()
    }

    /// The same codec with a tar inside it. `.gz` and `.tar.gz` are the same bytes at the
    /// front; what differs is only whether a tar comes out, which is a question about the
    /// contents and not about the name.
    pub fn with_tar_inside(self) -> Option<Format> {
        Some(match self {
            Format::Gz => Format::TarGz,
            Format::Bz2 => Format::TarBz2,
            Format::Xz => Format::TarXz,
            Format::Zst => Format::TarZst,
            _ => return None,
        })
    }

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
            Format::Ar => "ar",
            Format::Cpio => "cpio",
            Format::Rpm => "rpm",
            Format::Gz => "gz",
            Format::Bz2 => "bz2",
            Format::Xz => "xz",
            Format::Zst => "zst",
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
            Format::Ar => "a",
            Format::Cpio => "cpio",
            Format::Rpm => "rpm",
            Format::Gz => "gz",
            Format::Bz2 => "bz2",
            Format::Xz => "xz",
            Format::Zst => "zst",
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
    /// An `ar` says it too, and more simply than a tar: a header of sixty ASCII bytes, the
    /// member's bytes after it, no compression anywhere. So does a `cpio`. An `rpm` does not:
    /// its cpio is compressed, and is a compressed tar's case again.
    pub fn seekable(self) -> bool {
        matches!(self, Format::Zip | Format::Tar | Format::Ar | Format::Cpio)
    }

    /// The format a *name* suggests. Longest suffix first, so `.tar.gz` is not read as `.gz`.
    ///
    /// This is what decides whether `Enter` on a file goes *into* it (`ArcFs::looks_like
    /// _archive`), so it is also a list of what a reader is invited to look inside. Once one
    /// is inside, [`Format::of_bytes`] takes over and a renamed archive opens anyway — the
    /// names here only decide what is *offered*.
    ///
    /// Which is why the zip-based formats below stop where they do. A `.apk`, a `.whl`, a
    /// `.nupkg` is a thing one opens to look at what is in it; a `.docx`, a `.epub`, a `.cbz`
    /// is a thing one opens to *read*, and there the file association is the more useful
    /// answer. Both are zips and both open from the inside; the line is about which one
    /// `Enter` should mean, and it is a line that can be moved.
    ///
    /// `.crx` is not on the list for a different reason: a Chrome extension is a zip with a
    /// `Cr24` header in front of it, so it is not one.
    pub fn of_name(name: &str) -> Option<Format> {
        let lower = name.to_ascii_lowercase();
        const BY_SUFFIX: [(&str, Format); 36] = [
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
            // A zip under another name, for the packages one opens to look inside.
            (".jar", Format::Zip),
            (".war", Format::Zip),
            (".ear", Format::Zip),
            (".aar", Format::Zip),
            (".apk", Format::Zip),
            (".aab", Format::Zip),
            (".ipa", Format::Zip),
            (".whl", Format::Zip),
            (".nupkg", Format::Zip),
            (".vsix", Format::Zip),
            (".xpi", Format::Zip),
            // The Unix archive, and the Debian package that is one.
            (".a", Format::Ar),
            (".ar", Format::Ar),
            (".deb", Format::Ar),
            (".udeb", Format::Ar),
            (".ddeb", Format::Ar),
            (".cpio", Format::Cpio),
            (".rpm", Format::Rpm),
            // One compressed file, no tar. Last, so every `.tar.*` and `.t?z` above wins:
            // the list is searched in order and `.tar.gz` ends with `.gz` as well.
            (".gz", Format::Gz),
            (".bz2", Format::Bz2),
            (".bz", Format::Bz2),
            (".xz", Format::Xz),
            (".zst", Format::Zst),
            (".zstd", Format::Zst),
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
        // A compression magic says which codec, and nothing whatever about what is inside.
        // The name is what distinguishes `src.tar.gz` from `notes.txt.gz`; when the name says
        // neither, the single stream is the honest answer here, and `Archive::open_with`
        // settles it properly by decompressing the first block and looking for a tar header —
        // which it can do because it has the file, and this function has only its first bytes.
        if head.starts_with(&[0x1f, 0x8b]) {
            return Some(tarred(by_name, Format::Gz));
        }
        if head.starts_with(b"BZh") {
            return Some(tarred(by_name, Format::Bz2));
        }
        if head.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0x00]) {
            return Some(tarred(by_name, Format::Xz));
        }
        // 7z's own signature, tested after xz's: xz's starts with one byte and then the same
        // two letters, so the order of these two is what keeps them apart.
        if head.starts_with(&[b'7', b'z', 0xbc, 0xaf, 0x27, 0x1c]) {
            return Some(Format::SevenZ);
        }
        if head.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
            return Some(tarred(by_name, Format::Zst));
        }
        if head.starts_with(b"!<arch>\n") {
            return Some(Format::Ar);
        }
        if let Some(format) = crate::cpio::of_bytes(head) {
            return Some(format);
        }
        // A tar has no magic at the start: the word `ustar` sits 257 bytes in.
        if head.len() > 262 && (&head[257..262] == b"ustar") {
            return Some(Format::Tar);
        }
        by_name
    }
}

/// Which of the two readings of a compression magic the name supports: the tar one if the
/// name said so, the single stream otherwise.
fn tarred(by_name: Option<Format>, single: Format) -> Format {
    match (by_name, single.with_tar_inside()) {
        (Some(named), Some(tar)) if named == tar => tar,
        _ => single,
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
        // `.gz` on its own is one compressed file, which is an archive of one entry.
        assert_eq!(Format::of_name("notes.txt.gz"), Some(Format::Gz));
        assert_eq!(Format::of_name("dump.sql.BZ2"), Some(Format::Bz2));
        assert_eq!(Format::of_name("core.xz"), Some(Format::Xz));
        assert_eq!(Format::of_name("blob.zstd"), Some(Format::Zst));
        // And every tar suffix still wins over the bare one it ends with.
        for (name, want) in [
            ("src.tar.gz", Format::TarGz),
            ("src.tgz", Format::TarGz),
            ("src.tar.bz2", Format::TarBz2),
            ("src.tar.xz", Format::TarXz),
            ("src.tar.zst", Format::TarZst),
        ] {
            assert_eq!(Format::of_name(name), Some(want), "{name}");
        }
    }

    #[test]
    fn the_magic_names_the_codec_and_the_name_says_whether_a_tar_is_inside() {
        let gz = [0x1f, 0x8b, 0x08, 0x00];
        assert_eq!(Format::of_bytes(&gz, "src.tar.gz"), Some(Format::TarGz));
        assert_eq!(Format::of_bytes(&gz, "notes.txt.gz"), Some(Format::Gz));
        // A name that says nothing reads as the single stream here; the archive settles it.
        assert_eq!(Format::of_bytes(&gz, "backup"), Some(Format::Gz));
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
    fn a_single_stream_holds_one_entry_and_the_rest_hold_many() {
        for f in Format::ALL {
            assert_eq!(
                f.is_single(),
                matches!(f, Format::Gz | Format::Bz2 | Format::Xz | Format::Zst),
                "{}",
                f.label()
            );
            assert_eq!(f.holds_many(), !f.is_single(), "{}", f.label());
            // A single stream is never a tar, whatever codec it shares with one.
            assert!(!(f.is_single() && f.is_tar()), "{}", f.label());
            // Each one names the same codec with a tar in it, and the others name none.
            assert_eq!(
                f.with_tar_inside().is_some(),
                f.is_single(),
                "{}",
                f.label()
            );
        }
    }

    #[test]
    fn only_the_ones_that_can_be_read_out_of_order_say_they_can() {
        for f in Format::ALL {
            assert_eq!(
                f.seekable(),
                matches!(f, Format::Zip | Format::Tar | Format::Ar | Format::Cpio),
                "{}",
                f.label()
            );
        }
    }
}

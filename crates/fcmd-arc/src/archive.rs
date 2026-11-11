use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use fcmd_core::{CancelToken, Entry, EntryFlags, EntryId, EntryKind, VPath};

use crate::nest::{self, Fetch, Nest};
use crate::{ArcError, Format, Limits, Result};

/// One entry of an archive, as the index holds it.
#[derive(Debug, Clone)]
pub struct ArcEntry {
    /// Where it sits inside the archive: `/`-separated, always starting with `/`, never ending
    /// with one. The root itself is `/`.
    pub path: String,
    pub size: u64,
    /// What it takes up in the archive. `None` for a format that does not say per entry.
    pub packed: Option<u64>,
    pub mtime: Option<SystemTime>,
    /// Unix permission bits, when the archive carries them.
    pub mode: Option<u32>,
    pub kind: EntryKind,
    pub link_target: Option<String>,
    /// Where to find it again: the index for a zip, the byte offset for a plain tar. Neither
    /// is meaningful for a compressed tar, which is read from the start every time. An rpm
    /// keeps the member's inode here instead, which is what finds a hard link's bytes.
    pub(crate) at: Option<u64>,
}

/// What [`Archive::verify`] found: how much was read, and what did not come out whole.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
    pub members: u64,
    pub bytes: u64,
    /// The entries that failed, each with the reason it gave.
    pub bad: Vec<(String, String)>,
}

impl Report {
    pub fn is_whole(&self) -> bool {
        self.bad.is_empty()
    }
}

impl ArcEntry {
    /// The entry's own name, without the directories above it.
    pub fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or("")
    }

    /// The directory it lives in: `/` for an entry at the root.
    pub fn parent(&self) -> &str {
        match self.path.rfind('/') {
            Some(0) | None => "/",
            Some(cut) => &self.path[..cut],
        }
    }

    fn to_entry(&self) -> Entry {
        let mut flags = EntryFlags::empty();
        if self.name().starts_with('.') {
            flags.insert(EntryFlags::HIDDEN);
        }
        if let Some(mode) = self.mode {
            if mode & 0o111 != 0 && self.kind == EntryKind::File {
                flags.insert(EntryFlags::EXECUTABLE);
            }
        }
        Entry {
            name: OsString::from(self.name()),
            kind: self.kind,
            size: self.size,
            mtime: self.mtime,
            // An archive carries one timestamp per entry, and it is the modification time.
            btime: None,
            atime: None,
            flags,
            // An archive has no inodes, so the path inside it is the identity — which is
            // stable across a re-read and across sorting, which is what the panel needs.
            id: EntryId::Name(hash(&self.path)),
            link_target: self.link_target.as_deref().map(OsString::from),
        }
    }
}

fn hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

/// An archive, read once and then answered from the index.
///
/// Opening builds the whole index: a zip has one at the end, and a tar has to be walked. That
/// walk is the price of the first `Enter`, and a [`Cache`] keeps it so going in and out of an
/// archive does not pay it again.
#[derive(Debug)]
pub struct Archive {
    /// The format this crate read it as, or `None` when a program on the machine read it
    /// instead — `tool` then says which (`docs/05-multiarc.md` §9).
    format: Option<Format>,
    /// The table that read this, when one did. Kept because getting an entry out runs the
    /// program again: an archive this crate cannot parse cannot be seeked into either.
    tool: Option<Arc<crate::ext::Tool>>,
    /// And how to run it, which the cache owns and lends: one place in the program spawns a
    /// child, and a test puts its own in here.
    runner: Option<Arc<dyn crate::ext::Run>>,
    source: PathBuf,
    /// Size and modification time when it was read, so the cache can tell a stale index.
    stamp: (u64, Option<SystemTime>),
    entries: Vec<ArcEntry>,
    /// Entry index by path, and the children of each directory, both built once.
    by_path: HashMap<String, usize>,
    children: HashMap<String, Vec<usize>>,
    /// The password this was opened with, where it wanted one. Kept because reading an entry
    /// opens the archive again: a 7z holds its files in blocks and every one of them is
    /// encrypted with the key the header was. Never written anywhere.
    key: Option<String>,
    /// Why this crate cannot read every entry it lists, when that is so: a cabinet packed
    /// with a compressor that is not here, or one of a set. The index is still good — and
    /// the cache hands such an archive to a program that reads all of it, where there is one.
    beyond: Option<String>,
}

impl Archive {
    /// Read the index of the archive at `path`.
    pub fn open(path: &Path, limits: &Limits, cancel: &CancelToken) -> Result<Archive> {
        Archive::open_with(path, limits, cancel, None)
    }

    /// The same, with the password an encrypted archive wants.
    ///
    /// A 7z can encrypt its own header, so the password is needed to read the list of names
    /// and not only the bytes: without one such an archive answers [`ArcError::NeedsPassword`]
    /// rather than looking broken.
    pub fn open_with(
        path: &Path,
        limits: &Limits,
        cancel: &CancelToken,
        key: Option<&str>,
    ) -> Result<Archive> {
        let name = path.display().to_string();
        let meta = std::fs::metadata(path).map_err(|e| ArcError::Io {
            path: name.clone(),
            source: e,
        })?;
        let mut head = [0u8; 264];
        let read = {
            let mut file = File::open(path).map_err(|e| ArcError::Io {
                path: name.clone(),
                source: e,
            })?;
            read_up_to(&mut file, &mut head).map_err(|e| ArcError::Io {
                path: name.clone(),
                source: e,
            })?
        };
        let leaf = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let format =
            Format::of_bytes(&head[..read], &leaf).ok_or(ArcError::Unknown(name.clone()))?;
        // A gzip magic says gzip and no more. `Format::of_bytes` has only the first bytes to
        // go on, so it answers from the name and leaves the real question here, where the
        // file is: decompress the front of the stream and look for the word a tar header
        // carries 257 bytes in. A tarball somebody renamed `backup.gz` is still a tarball,
        // which is the promise the zip and tar magics keep on their own.
        let format = match format.with_tar_inside() {
            Some(tar) if tar_inside(path, format) => tar,
            _ => format,
        };

        let mut beyond = None;
        let entries = match format {
            Format::Cab => {
                let found = crate::cab::index(path, &name, limits, cancel)?;
                beyond = found.beyond;
                found.entries
            }
            Format::Zip => index_zip(path, &name, limits, cancel)?,
            Format::SevenZ => index_sevenz(path, &name, limits, cancel, key)?,
            Format::Ar => index_ar(path, &name, limits, cancel)?,
            Format::Cpio => crate::cpio::index_file(path, &name, limits, cancel)?,
            Format::Rpm => crate::cpio::index_rpm(path, &name, limits, cancel)?,
            Format::Sqlite => crate::sqlite::index(path, &name, limits, cancel)?,
            Format::Json => crate::json::index(path, &name, limits, cancel)?,
            Format::Eml => crate::mail::index_eml(path, &name, limits, cancel)?,
            Format::Mbox => crate::mail::index_mbox(path, &name, limits, cancel)?,
            f if f.is_single() => index_single(path, &name, &leaf, format, limits, cancel)?,
            _ => index_tar(path, &name, format, limits, cancel)?,
        };

        let packed = meta.len();
        let total: u64 = entries.iter().map(|e| e.size).sum();
        if total > limits.total_bytes {
            return Err(ArcError::Refused {
                path: name,
                reason: format!(
                    "it unpacks to {total} bytes, over the limit of {}",
                    limits.total_bytes
                ),
            });
        }
        // The ratio guard only means anything once the archive is big enough for the division
        // to say something: a 200-byte zip of a 1 MiB file of zeroes is not an attack.
        if packed > 4096 && limits.ratio != u64::MAX && total / packed.max(1) > limits.ratio {
            return Err(ArcError::Refused {
                path: name,
                reason: format!(
                    "it unpacks to {}× its own size, over the limit of {}×",
                    total / packed.max(1),
                    limits.ratio
                ),
            });
        }

        let mut archive = Archive {
            format: Some(format),
            tool: None,
            runner: None,
            source: path.to_path_buf(),
            stamp: (meta.len(), meta.modified().ok()),
            entries,
            by_path: HashMap::new(),
            children: HashMap::new(),
            key: key.map(str::to_string),
            beyond,
        };
        archive.build_tree();
        Ok(archive)
    }

    /// Read an archive this crate cannot parse, by asking a program that can.
    ///
    /// The table says how (`docs/05-multiarc.md`): `List` runs, its output goes through the
    /// table's own grammar, and what comes back becomes entries of the same shape a zip's
    /// are — so the panel, the search inside archives and `Ctrl+A` on an entry need nothing
    /// new. Read only, by the rule in §9.
    pub fn open_external(
        path: &Path,
        tool: Arc<crate::ext::Tool>,
        runner: Arc<dyn crate::ext::Run>,
        limits: &Limits,
        cancel: &CancelToken,
    ) -> Result<Archive> {
        let name = path.display().to_string();
        let meta = std::fs::metadata(path).map_err(|e| ArcError::Io {
            path: name.clone(),
            source: e,
        })?;
        let rows = tool
            .list(path, runner.as_ref(), cancel)
            .map_err(|e| match e {
                crate::ext::ExtError::Cancelled => ArcError::Cancelled,
                other => ArcError::External {
                    path: name.clone(),
                    reason: other.to_string(),
                },
            })?;
        if rows.len() > limits.entries {
            return Err(ArcError::Refused {
                path: name,
                reason: format!(
                    "it holds {} entries, over the limit of {}",
                    rows.len(),
                    limits.entries
                ),
            });
        }
        let mut entries = Vec::with_capacity(rows.len());
        let mut total: u64 = 0;
        for row in rows {
            let kind = if row.is_dir {
                EntryKind::Dir
            } else {
                EntryKind::File
            };
            total = total.saturating_add(row.size.unwrap_or(0));
            entries.push(ArcEntry {
                path: normalise(&row.name),
                size: row.size.unwrap_or(0),
                packed: row.packed,
                mtime: row.mtime,
                // These listings print a mode as letters, not bits, and guessing bits from
                // letters would be inventing: `Ctrl+A` shows nothing rather than a wrong 644.
                mode: None,
                kind,
                link_target: None,
                at: None,
            });
        }
        if total > limits.total_bytes {
            return Err(ArcError::Refused {
                path: name,
                reason: format!(
                    "it unpacks to {total} bytes, over the limit of {}",
                    limits.total_bytes
                ),
            });
        }
        let mut archive = Archive {
            format: None,
            tool: Some(tool),
            runner: Some(runner),
            source: path.to_path_buf(),
            stamp: (meta.len(), meta.modified().ok()),
            entries,
            by_path: HashMap::new(),
            children: HashMap::new(),
            key: None,
            beyond: None,
        };
        archive.build_tree();
        Ok(archive)
    }

    /// These entries out of an archive a program read, in one run of the program, into `into`
    /// — where each lands at the path it has inside the archive.
    ///
    /// One run for the whole set is the point: a program asked for a file at a time starts
    /// the archive again for each, and in a solid `.rar` that means unpacking everything in
    /// front of it every time. `into` must exist, and is the caller's to clean up: a
    /// cancelled run leaves in it whatever the program had written when it was killed.
    pub fn unpack_through_tool(
        &self,
        names: &[String],
        into: &Path,
        cancel: &CancelToken,
    ) -> Result<()> {
        let (Some(tool), Some(runner)) = (&self.tool, &self.runner) else {
            return Err(ArcError::External {
                path: self.name(),
                reason: "no program is attached to this archive".to_string(),
            });
        };
        tool.extract(&self.source, names, into, runner.as_ref(), cancel)
            .map_err(|e| match e {
                crate::ext::ExtError::Cancelled => ArcError::Cancelled,
                other => ArcError::External {
                    path: self.name(),
                    reason: other.to_string(),
                },
            })
    }

    /// One entry out of an archive a program read: the program runs again, into a temporary
    /// folder, and the file it wrote is poured into `out`.
    ///
    /// A whole copy of the entry reaches the disk first, which is what asking a program for
    /// one file costs: none of them will write a single member to a pipe in a way that is the
    /// same across tools, and the alternative — parsing the format ourselves — is the thing
    /// this exists not to do.
    fn copy_out_through_tool(
        &self,
        inner: &str,
        out: &mut dyn Write,
        cancel: &CancelToken,
        progress: &mut dyn FnMut(u64),
    ) -> Result<u64> {
        let temp = scratch()?;
        let outcome = (|| -> Result<u64> {
            self.unpack_through_tool(std::slice::from_ref(&inner.to_string()), &temp, cancel)?;
            // It landed under the temporary folder at the path it has inside the archive.
            let landed = temp.join(inner.trim_start_matches('/'));
            let mut file = File::open(&landed).map_err(|e| ArcError::External {
                path: self.name(),
                reason: format!("`{}` wrote nothing for {inner}: {e}", self.label()),
            })?;
            copy(&mut file, out, cancel, progress)
        })();
        // Whatever happened, including a cancellation that left the program's half-written
        // files behind: this folder is ours and nothing outside here knows its name.
        let _ = std::fs::remove_dir_all(&temp);
        outcome
    }

    /// The directories an archive does not bother to list are still directories: a tar of
    /// `a/b/c.txt` with no entry for `a/b` must still show `a` and then `b`.
    fn build_tree(&mut self) {
        let mut synthetic: Vec<String> = Vec::new();
        {
            let known: std::collections::HashSet<&str> =
                self.entries.iter().map(|e| e.path.as_str()).collect();
            for entry in &self.entries {
                let mut dir = entry.parent();
                while dir != "/" {
                    if !known.contains(dir) && !synthetic.iter().any(|s| s == dir) {
                        synthetic.push(dir.to_string());
                    }
                    dir = match dir.rfind('/') {
                        Some(0) | None => "/",
                        Some(cut) => &dir[..cut],
                    };
                }
            }
        }
        for path in synthetic {
            self.entries.push(ArcEntry {
                path,
                size: 0,
                packed: None,
                mtime: None,
                mode: None,
                kind: EntryKind::Dir,
                link_target: None,
                at: None,
            });
        }
        for (i, entry) in self.entries.iter().enumerate() {
            self.by_path.insert(entry.path.clone(), i);
            self.children
                .entry(entry.parent().to_string())
                .or_default()
                .push(i);
        }
    }

    /// The format this crate read it as. `None` for an archive a program on the machine read:
    /// there is no `Format` for a `.rar`, because this crate cannot parse one.
    pub fn format(&self) -> Option<Format> {
        self.format
    }

    /// What the frame calls it: the format's own name, or the table's.
    pub fn label(&self) -> &str {
        match (&self.format, &self.tool) {
            (Some(format), _) => format.label(),
            (None, Some(tool)) => &tool.name,
            (None, None) => "archive",
        }
    }

    /// Why some of what this lists cannot be read here, when that is so — see the field.
    pub fn beyond(&self) -> Option<&str> {
        self.beyond.as_deref()
    }

    /// The table that reads this, when a program on the machine does.
    pub fn tool(&self) -> Option<&Arc<crate::ext::Tool>> {
        self.tool.as_ref()
    }

    pub fn source(&self) -> &Path {
        &self.source
    }

    pub fn entries(&self) -> &[ArcEntry] {
        &self.entries
    }

    /// Files, directories and the total unpacked size, for the panel's footer.
    pub fn totals(&self) -> (u64, u64, u64) {
        let mut files = 0;
        let mut dirs = 0;
        let mut bytes = 0;
        for entry in &self.entries {
            if entry.kind == EntryKind::Dir {
                dirs += 1;
            } else {
                files += 1;
                bytes += entry.size;
            }
        }
        (files, dirs, bytes)
    }

    /// Is the index still the one for the file on disk?
    pub fn is_current(&self) -> bool {
        match std::fs::metadata(&self.source) {
            Ok(meta) => self.stamp == (meta.len(), meta.modified().ok()),
            Err(_) => false,
        }
    }

    /// One entry as a panel row, by its path inside the archive.
    pub fn row(&self, inner: &str) -> Option<Entry> {
        self.entry(inner).map(|e| e.to_entry())
    }

    pub fn entry(&self, inner: &str) -> Option<&ArcEntry> {
        let inner = normalise(inner);
        self.by_path.get(&inner).map(|i| &self.entries[*i])
    }

    /// What is immediately inside `dir`, as panel rows. The `..` row is the panel's business,
    /// not ours.
    pub fn list(&self, dir: &str) -> Vec<Entry> {
        let dir = normalise(dir);
        let mut out: Vec<Entry> = self
            .children
            .get(&dir)
            .map(|kids| kids.iter().map(|i| self.entries[*i].to_entry()).collect())
            .unwrap_or_default();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// Is this a directory inside the archive (so `Enter` goes into it)?
    pub fn is_dir(&self, inner: &str) -> bool {
        let inner = normalise(inner);
        inner == "/"
            || self
                .entry(&inner)
                .map(|e| e.kind == EntryKind::Dir)
                .unwrap_or(false)
    }

    /// Every file at or under `inner`, in archive order — what an extraction of a directory,
    /// or of the whole archive, has to write.
    pub fn under(&self, inner: &str) -> Vec<&ArcEntry> {
        let inner = normalise(inner);
        self.entries
            .iter()
            .filter(|e| {
                if inner == "/" {
                    return true;
                }
                e.path == inner
                    || (e.path.starts_with(&inner)
                        && e.path.as_bytes().get(inner.len()) == Some(&b'/'))
            })
            .collect()
    }

    /// The bytes of one entry. The viewer and the editor want them in memory; an extraction
    /// goes through [`Archive::extract`] instead and never holds a whole file.
    pub fn read(&self, inner: &str, cancel: &CancelToken) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        self.copy_out(inner, &mut out, cancel, &mut |_| {})?;
        Ok(out)
    }

    /// Write one entry to `dst`, reporting the bytes written as they go.
    pub fn extract(
        &self,
        inner: &str,
        dst: &Path,
        cancel: &CancelToken,
        progress: &mut dyn FnMut(u64),
    ) -> Result<u64> {
        let name = dst.display().to_string();
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ArcError::Io {
                path: name.clone(),
                source: e,
            })?;
        }
        let file = File::create(dst).map_err(|e| ArcError::Io {
            path: name.clone(),
            source: e,
        })?;
        let mut file = std::io::BufWriter::new(file);
        let written = self.copy_out(inner, &mut file, cancel, progress)?;
        file.flush().map_err(|e| ArcError::Io {
            path: name,
            source: e,
        })?;
        Ok(written)
    }

    /// Read every entry and say which ones do not come out whole.
    ///
    /// This is what *test archive* means: the bytes are decompressed, checked by whatever
    /// the format checks them with and thrown away. What a format checks them with is: a
    /// zip's and a 7z's CRC-32 per entry; gzip's CRC-32 and length, bzip2's CRC per block,
    /// xz's own check (CRC-32, CRC-64 or SHA-256) and zstd's checksum when the frame carries
    /// one, for a compressed stream and so for every member of a tarball inside it; a tar
    /// header's own checksum, which says the walk is still on the headers; and the framing
    /// of everything else — an entry that ends before its size says it should is not whole.
    /// What comes back is the count, the size, and the entries that failed with the reason
    /// each one gave.
    ///
    /// `progress` hears the entry being read, how many have been started and how many
    /// bytes have come out so far — inside an entry too, since one entry can be most of an
    /// archive.
    ///
    /// A compressed tar is one stream, so it is walked once instead of being reopened per
    /// entry: testing a thousand-member `.tar.gz` the other way would read the archive a
    /// thousand times. An archive a program reads is handed to that program's own `Test`,
    /// once ([`Archive::verify_through_tool`]).
    pub fn verify(
        &self,
        cancel: &CancelToken,
        progress: &mut dyn FnMut(&str, u64, u64),
    ) -> Result<Report> {
        let mut report = Report::default();
        if self.format.is_none() {
            return self.verify_through_tool(cancel, progress);
        }
        // One entry read to nowhere, counted as it goes. Its failure is the entry's and
        // goes in the report; only a cancel is the caller's.
        fn read_one(
            report: &mut Report,
            path: String,
            bytes: &mut dyn Read,
            cancel: &CancelToken,
            progress: &mut dyn FnMut(&str, u64, u64),
        ) -> Result<()> {
            report.members += 1;
            let (members, before) = (report.members, report.bytes);
            progress(&path, members, before);
            let mut read_so_far = 0;
            let outcome = copy(bytes, &mut io::sink(), cancel, &mut |n| {
                read_so_far = n;
                progress(&path, members, before + n);
            });
            match outcome {
                Ok(read) => report.bytes += read,
                Err(ArcError::Cancelled) => return Err(ArcError::Cancelled),
                Err(e) => {
                    // What came out before it broke was still read.
                    report.bytes += read_so_far;
                    report.bad.push((path, reason_of(e)));
                }
            }
            Ok(())
        }
        if self.format == Some(Format::SevenZ) {
            // One pass over the whole archive rather than one read per entry: a 7z block holds
            // several files together, so asking for them one at a time would decode the block
            // once per file in it. The decoder checks each entry's CRC as it goes, which is
            // what makes reading it to nowhere a test.
            let mut reader = sevenz_reader(&self.source, self.key.as_deref())?;
            let mut stopped = None;
            let outcome = reader.for_each_entries(|entry, bytes| {
                if cancel.is_cancelled() {
                    stopped = Some(ArcError::Cancelled);
                    return Ok(false);
                }
                if entry.is_directory {
                    return Ok(true);
                }
                let path = normalise(&entry.name);
                if let Err(e) = read_one(&mut report, path, bytes, cancel, progress) {
                    stopped = Some(e);
                    return Ok(false);
                }
                Ok(true)
            });
            if let Some(e) = stopped {
                return Err(e);
            }
            // A failure here is the archive's framing or its header, not one entry's bytes:
            // the entry-level failures are in the report above.
            outcome.map_err(|e| self.sevenz_error(e))?;
            return Ok(report);
        }
        if self.format == Some(Format::Cab) {
            // One pass over each folder, for a solid block's reason: a folder is one stream
            // and its files are slices of it, so reading them one at a time would unpack
            // the folder once per file. Each block's checksum is checked as it is read.
            let mut by_folder: std::collections::BTreeMap<usize, Vec<(u64, &ArcEntry)>> =
                Default::default();
            for entry in self.entries.iter().filter(|e| e.kind != EntryKind::Dir) {
                match entry.at {
                    Some(at) => {
                        let (folder, offset) = crate::cab::folder_of(at);
                        by_folder.entry(folder).or_default().push((offset, entry));
                    }
                    // In another cabinet of the set: listed, and said to be untested.
                    None => {
                        report.members += 1;
                        report
                            .bad
                            .push((entry.path.clone(), self.beyond.clone().unwrap_or_default()));
                    }
                }
            }
            for (folder, mut files) in by_folder {
                files.sort_by_key(|(offset, _)| *offset);
                let at = (folder as u64) << 32;
                // Why nothing more can come out of this folder, once that is so: a
                // compressor that is not here, or a block that failed — after which an
                // MSZIP stream has lost the history every later block leans on.
                let (mut reader, mut lost) =
                    match crate::cab::folder(&self.source, &self.name(), at) {
                        Ok((reader, _)) => (Some(reader), None),
                        Err(ArcError::Refused { reason, .. }) => {
                            (None, Some(self.beyond.clone().unwrap_or(reason)))
                        }
                        Err(e) => (None, Some(reason_of(e))),
                    };
                let mut position = 0u64;
                for (offset, entry) in files {
                    if cancel.is_cancelled() {
                        return Err(ArcError::Cancelled);
                    }
                    let (Some(stream), None) = (reader.as_mut(), &lost) else {
                        report.members += 1;
                        report
                            .bad
                            .push((entry.path.clone(), lost.clone().unwrap_or_default()));
                        continue;
                    };
                    // Two files over the same bytes is something a cabinet may say, and
                    // going back is not something a stream can do: that one is read on
                    // its own.
                    if offset < position {
                        let path = entry.path.clone();
                        report.members += 1;
                        let (members, before) = (report.members, report.bytes);
                        progress(&path, members, before);
                        match self.copy_out(&path, &mut io::sink(), cancel, &mut |n| {
                            progress(&path, members, before + n)
                        }) {
                            Ok(bytes) => report.bytes += bytes,
                            Err(ArcError::Cancelled) => return Err(ArcError::Cancelled),
                            Err(e) => report.bad.push((path, reason_of(e))),
                        }
                        continue;
                    }
                    let gap = offset - position;
                    match copy(
                        &mut (&mut *stream).take(gap),
                        &mut io::sink(),
                        cancel,
                        &mut |_| {},
                    ) {
                        Ok(skipped) if skipped == gap => {}
                        Ok(_) => {
                            lost = Some("the cabinet's data ends before this entry".to_string())
                        }
                        Err(ArcError::Cancelled) => return Err(ArcError::Cancelled),
                        Err(e) => lost = Some(reason_of(e)),
                    }
                    if let Some(reason) = &lost {
                        report.members += 1;
                        report.bad.push((entry.path.clone(), reason.clone()));
                        continue;
                    }
                    let failed_before = report.bad.len();
                    let mut slice = crate::cab::Exactly {
                        inner: &mut *stream,
                        left: entry.size,
                    };
                    read_one(
                        &mut report,
                        entry.path.clone(),
                        &mut slice,
                        cancel,
                        progress,
                    )?;
                    position = offset + entry.size;
                    if report.bad.len() > failed_before {
                        lost = report.bad.last().map(|(_, reason)| reason.clone());
                    }
                }
            }
            return Ok(report);
        }
        if self.format == Some(Format::Rpm) {
            // One walk of the payload, for the reason a compressed tar gets one: reading the
            // entries one at a time would decompress the payload once for every file in it.
            let name = self.name();
            crate::cpio::walk(
                crate::cpio::rpm_stream(&self.source)?,
                &name,
                &Limits::unlimited(),
                cancel,
                |member, bytes| {
                    if member.kind() != EntryKind::File || member.size == 0 {
                        return Ok(true);
                    }
                    let path = normalise(&member.name);
                    read_one(&mut report, path, bytes, cancel, progress)?;
                    Ok(true)
                },
            )?;
            return Ok(report);
        }
        if self.format.is_some_and(|f| f.is_tar() && !f.seekable()) {
            let format = self.format.expect("just asked");
            let mut stream = crate::tar::Reader::new(decoder(&self.source, format)?);
            loop {
                let item = match stream.next() {
                    Ok(Some(item)) => item,
                    Ok(None) => break,
                    // The stream gave out between two members: a wrong header checksum, or
                    // the compressor's own check failing on the padding. Nothing after this
                    // can be found, and the members before it were read whole — so it is
                    // one line of the report, against the archive, rather than a test that
                    // says nothing about the entries it did read.
                    Err(e) => {
                        report.bad.push((String::new(), e.to_string()));
                        break;
                    }
                };
                if cancel.is_cancelled() {
                    return Err(ArcError::Cancelled);
                }
                if item.kind() == crate::tar::Kind::Dir {
                    continue;
                }
                let path = tar_name(&item.name);
                read_one(&mut report, path, &mut stream, cancel, progress)?;
            }
            return Ok(report);
        }
        for entry in &self.entries {
            if cancel.is_cancelled() {
                return Err(ArcError::Cancelled);
            }
            if entry.kind == EntryKind::Dir {
                continue;
            }
            let path = entry.path.clone();
            report.members += 1;
            let (members, before) = (report.members, report.bytes);
            progress(&path, members, before);
            let mut read_so_far = 0;
            let outcome = self.copy_out(&path, &mut io::sink(), cancel, &mut |n| {
                read_so_far = n;
                progress(&path, members, before + n);
            });
            match outcome {
                Ok(bytes) => report.bytes += bytes,
                Err(ArcError::Cancelled) => return Err(ArcError::Cancelled),
                Err(e) => {
                    report.bytes += read_so_far;
                    report.bad.push((path, reason_of(e)));
                }
            }
        }
        Ok(report)
    }

    /// The test of an archive a program reads: that program's own `Test`, run once.
    ///
    /// The names go in a list file as they do for an extraction, and what the program
    /// printed about them comes back as the report: an entry it named in a complaint is a
    /// bad entry with that line as its reason, and a complaint that names none is against
    /// the archive (an empty name). A table with no `Test` line extracts everything into a
    /// temporary folder instead and throws it away — every one of these programs checks
    /// what it extracts — which can only say that the archive failed, not where.
    ///
    /// The program is one opaque run, so `progress` is told once, at the start.
    fn verify_through_tool(
        &self,
        cancel: &CancelToken,
        progress: &mut dyn FnMut(&str, u64, u64),
    ) -> Result<Report> {
        let (Some(tool), Some(runner)) = (&self.tool, &self.runner) else {
            return Err(ArcError::External {
                path: self.name(),
                reason: "no program is attached to this archive".to_string(),
            });
        };
        let files: Vec<&ArcEntry> = self
            .entries
            .iter()
            .filter(|entry| entry.kind != EntryKind::Dir)
            .collect();
        let names: Vec<String> = files.iter().map(|entry| entry.path.clone()).collect();
        let mut report = Report {
            members: files.len() as u64,
            bytes: files.iter().map(|entry| entry.size).sum(),
            bad: Vec::new(),
        };
        progress("", 0, 0);
        let temp = scratch()?;
        let outcome = if tool.can(crate::ext::Op::Test) {
            tool.test(&self.source, &names, &temp, runner.as_ref(), cancel)
        } else {
            tool.extract(&self.source, &names, &temp, runner.as_ref(), cancel)
                .map(|()| Vec::new())
        };
        let _ = std::fs::remove_dir_all(&temp);
        match outcome {
            Ok(bad) => {
                report.bad = bad
                    .into_iter()
                    .map(|(name, why)| match name.is_empty() {
                        true => (name, why),
                        false => (normalise(&name), why),
                    })
                    .collect();
                Ok(report)
            }
            Err(crate::ext::ExtError::Cancelled) => Err(ArcError::Cancelled),
            // It ran and said no, with nothing a test could use: that is the verdict.
            Err(crate::ext::ExtError::Failed { .. }) if !tool.can(crate::ext::Op::Test) => {
                report.bad.push((
                    String::new(),
                    format!("`{}` could not extract it whole", tool.program),
                ));
                Ok(report)
            }
            Err(other) => Err(ArcError::External {
                path: self.name(),
                reason: other.to_string(),
            }),
        }
    }

    fn copy_out(
        &self,
        inner: &str,
        out: &mut dyn Write,
        cancel: &CancelToken,
        progress: &mut dyn FnMut(u64),
    ) -> Result<u64> {
        let inner = normalise(inner);
        let entry = self.entry(&inner).ok_or_else(|| ArcError::NoEntry {
            path: self.name(),
            entry: inner.clone(),
        })?;
        if entry.kind == EntryKind::Dir {
            return Err(ArcError::NoEntry {
                path: self.name(),
                entry: format!("{inner} is a directory"),
            });
        }
        let Some(format) = self.format else {
            // Nothing in this crate can parse it, so there is nothing to seek into: the
            // program that listed it runs again and hands the entry over.
            return self.copy_out_through_tool(&inner, out, cancel, progress);
        };
        match format {
            Format::Eml => {
                let n = crate::mail::copy_out_eml(&self.source, &self.name(), &inner, out)?;
                progress(n);
                Ok(n)
            }
            Format::Mbox => {
                let n =
                    crate::mail::copy_out_mbox(&self.source, &self.name(), entry.at, &inner, out)?;
                progress(n);
                Ok(n)
            }
            Format::Json => {
                let n = crate::json::copy_out(&self.source, &self.name(), &inner, out)?;
                progress(n);
                Ok(n)
            }
            Format::Sqlite => {
                let n = crate::sqlite::copy_out(&self.source, &self.name(), &inner, out, cancel)?;
                progress(n);
                Ok(n)
            }
            Format::Zip => {
                let file = File::open(&self.source).map_err(|e| self.io(e))?;
                let mut zip =
                    zip::ZipArchive::new(BufReader::new(file)).map_err(|e| self.broken(e))?;
                let index = entry.at.unwrap_or(0) as usize;
                let mut item = zip.by_index(index).map_err(|e| self.broken(e))?;
                copy(&mut item, out, cancel, progress)
            }
            Format::Cab => {
                // Listed, and not readable here: the reason the index gave is the answer,
                // rather than a folder number.
                let refused = || ArcError::Refused {
                    path: self.name(),
                    reason: self
                        .beyond
                        .clone()
                        .unwrap_or_else(|| "this entry cannot be read here".to_string()),
                };
                let at = entry.at.ok_or_else(refused)?;
                let (mut folder, offset) = crate::cab::folder(&self.source, &self.name(), at)
                    .map_err(|e| match e {
                        ArcError::Refused { .. } => refused(),
                        other => other,
                    })?;
                // A folder is one stream and the entry is a slice of it: what is in front
                // is unpacked and dropped, which is what a solid archive costs.
                let skipped = copy(
                    &mut (&mut folder).take(offset),
                    &mut io::sink(),
                    cancel,
                    &mut |_| {},
                )?;
                if skipped != offset {
                    return Err(self.broken("the cabinet's data ends before this entry starts"));
                }
                let mut slice = crate::cab::Exactly {
                    inner: folder,
                    left: entry.size,
                };
                copy(&mut slice, out, cancel, progress)
            }
            Format::Rpm => {
                // The payload is one compressed stream, walked up to the member: a compressed
                // tar's case. The limits are the ones the index was built under, which this
                // archive has already passed.
                crate::cpio::rpm_entry(
                    &self.source,
                    &inner,
                    entry.at,
                    &Limits::unlimited(),
                    cancel,
                    |bytes| copy(bytes, out, cancel, progress),
                )?
                .ok_or_else(|| ArcError::NoEntry {
                    path: self.name(),
                    entry: inner.clone(),
                })
            }
            Format::Tar | Format::Ar | Format::Cpio => {
                // A plain tar, an `ar` and a `cpio` all say where each entry's bytes start, and neither
                // compresses them, so one seek is enough.
                let mut file = File::open(&self.source).map_err(|e| self.io(e))?;
                let at = entry.at.ok_or_else(|| ArcError::Broken {
                    path: self.name(),
                    reason: "the index does not say where this entry is".to_string(),
                })?;
                file.seek(SeekFrom::Start(at)).map_err(|e| self.io(e))?;
                let mut window = file.take(entry.size);
                copy(&mut window, out, cancel, progress)
            }
            // A 7z is read through its own reader, which walks the blocks in order and hands
            // each entry's bytes to a closure. For a solid archive that is the only way there
            // is: the files in a block are one compressed stream between them.
            Format::SevenZ => {
                let mut reader = sevenz_reader(&self.source, self.key.as_deref())?;
                let mut result: Option<Result<u64>> = None;
                let outcome = reader.for_each_entries(|entry, bytes| {
                    if cancel.is_cancelled() {
                        result = Some(Err(ArcError::Cancelled));
                        return Ok(false);
                    }
                    if entry.is_directory || normalise(&entry.name) != inner {
                        // Skipping an entry means *reading past* it. In a solid block the
                        // files are one stream between them, so leaving one unread would
                        // leave the stream in the middle of it and the next file would come
                        // out as rubble — which the CRC then reports as a broken archive.
                        io::copy(bytes, &mut io::sink())?;
                        return Ok(true);
                    }
                    result = Some(copy(bytes, out, cancel, progress));
                    Ok(false)
                });
                match result {
                    Some(found) => found,
                    None => {
                        outcome.map_err(|e| self.sevenz_error(e))?;
                        Err(ArcError::NoEntry {
                            path: self.name(),
                            entry: inner,
                        })
                    }
                }
            }
            // A single compressed stream *is* the entry: there is no header to walk past and
            // nothing to look for. Everything that came out is the file.
            f if f.is_single() => {
                let mut stream = decoder(&self.source, f)?;
                copy(&mut stream, out, cancel, progress)
            }
            // A compressed tar is one stream: the only way to the last entry is through all
            // of it. That is the format's doing, not ours, and it is why `seekable` says no.
            _ => {
                let mut stream = crate::tar::Reader::new(decoder(&self.source, format)?);
                while let Some(item) = stream.next().map_err(|e| self.broken(e))? {
                    if tar_name(&item.name) == inner {
                        return copy(&mut stream, out, cancel, progress);
                    }
                    if cancel.is_cancelled() {
                        return Err(ArcError::Cancelled);
                    }
                }
                Err(ArcError::NoEntry {
                    path: self.name(),
                    entry: inner,
                })
            }
        }
    }

    fn name(&self) -> String {
        self.source.display().to_string()
    }

    fn io(&self, source: io::Error) -> ArcError {
        ArcError::Io {
            path: self.name(),
            source,
        }
    }

    fn broken(&self, reason: impl std::fmt::Display) -> ArcError {
        ArcError::Broken {
            path: self.name(),
            reason: reason.to_string(),
        }
    }

    fn sevenz_error(&self, e: sevenz_rust2::Error) -> ArcError {
        sevenz_error(&self.source, e)
    }
}

/// A 7z reader over the file at `path`, with no password.
///
/// Opening is the header read, so this is also where an encrypted archive is found out: a 7z
/// can encrypt its own header, in which case even the list of names needs the password.
pub(crate) fn sevenz_reader(
    path: &Path,
    key: Option<&str>,
) -> Result<sevenz_rust2::ArchiveReader<BufReader<File>>> {
    let file = File::open(path).map_err(|e| ArcError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let password = match key {
        Some(key) => sevenz_rust2::Password::from(key),
        None => sevenz_rust2::Password::empty(),
    };
    sevenz_rust2::ArchiveReader::new(BufReader::with_capacity(64 * 1024, file), password)
        .map_err(|e| sevenz_error(path, e))
}

/// A 7z error in our words. The two that have an answer the user can act on — a password, and
/// a codec we did not build in — are said as themselves; the rest are a broken archive.
pub(crate) fn sevenz_error(path: &Path, e: sevenz_rust2::Error) -> ArcError {
    let name = path.display().to_string();
    match e {
        sevenz_rust2::Error::PasswordRequired | sevenz_rust2::Error::MaybeBadPassword(_) => {
            ArcError::NeedsPassword { path: name }
        }
        sevenz_rust2::Error::UnsupportedCompressionMethod(method) => ArcError::Refused {
            path: name,
            reason: format!("it is packed with {method}, which this build cannot read"),
        },
        sevenz_rust2::Error::Unsupported(what) => ArcError::Refused {
            path: name,
            reason: what.to_string(),
        },
        sevenz_rust2::Error::MaxMemLimited { max_kb, actaul_kb } => ArcError::Refused {
            path: name,
            reason: format!("unpacking it wants {actaul_kb} KiB of memory, over {max_kb} KiB"),
        },
        sevenz_rust2::Error::Io(source, _) | sevenz_rust2::Error::FileOpen(source, _) => {
            ArcError::Io { path: name, source }
        }
        other => ArcError::Broken {
            path: name,
            reason: other.to_string(),
        },
    }
}

/// Why an entry did not come out whole, for a report that already names the entry: the
/// reason alone, without the archive's path that an error carries for a message of its own.
fn reason_of(error: ArcError) -> String {
    match error {
        ArcError::Io { source, .. } => source.to_string(),
        ArcError::Broken { reason, .. }
        | ArcError::Refused { reason, .. }
        | ArcError::External { reason, .. } => reason,
        other => other.to_string(),
    }
}

fn copy(
    from: &mut dyn Read,
    to: &mut dyn Write,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(u64),
) -> Result<u64> {
    let mut buf = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        let read = from.read(&mut buf).map_err(|e| ArcError::Io {
            path: "the archive".to_string(),
            source: e,
        })?;
        if read == 0 {
            return Ok(total);
        }
        to.write_all(&buf[..read]).map_err(|e| ArcError::Io {
            path: "the destination".to_string(),
            source: e,
        })?;
        total += read as u64;
        progress(total);
    }
}

/// A zstd stream that fails at its end when the checksum in the frame is not the one of
/// the bytes that came out.
///
/// The decoder computes the checksum as it goes and keeps the one the frame carries, and
/// compares neither: a `.zst` with a flipped bit inside a literal run decodes without a
/// word. A frame written without a checksum has nothing to compare and passes, as it does
/// for `zstd -t`.
struct ZstdChecked<R: Read> {
    inner: ruzstd::decoding::StreamingDecoder<R, ruzstd::decoding::FrameDecoder>,
    checked: bool,
}

impl<R: Read> Read for ZstdChecked<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buf)?;
        if read == 0 && !buf.is_empty() && !self.checked {
            self.checked = true;
            let written = self.inner.decoder.get_checksum_from_data();
            let computed = self.inner.decoder.get_calculated_checksum();
            if let (Some(written), Some(computed)) = (written, computed) {
                if written != computed {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "the zstd checksum does not match what was unpacked",
                    ));
                }
            }
        }
        Ok(read)
    }
}

/// Read as much as fits, returning how much that was. `read` is allowed to stop early and a
/// short archive is shorter than the buffer, so neither is an error.
fn read_up_to(file: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match file.read(&mut buf[filled..])? {
            0 => break,
            n => filled += n,
        }
    }
    Ok(filled)
}

/// The decompressor for a compressed stream, as a plain reader. Whether a tar comes out of
/// it is the caller's business: the codec is the same either way.
pub(crate) fn decoder(path: &Path, format: Format) -> Result<Box<dyn Read>> {
    let name = path.display().to_string();
    let file = File::open(path).map_err(|e| ArcError::Io {
        path: name.clone(),
        source: e,
    })?;
    let file = BufReader::with_capacity(64 * 1024, file);
    Ok(match format {
        Format::Tar => Box::new(file),
        Format::TarGz | Format::Gz => Box::new(flate2::read::GzDecoder::new(file)),
        Format::TarBz2 | Format::Bz2 => Box::new(bzip2::read::BzDecoder::new(file)),
        Format::TarXz | Format::Xz => Box::new(lzma_rust2::XzReader::new(file, true)),
        Format::TarZst | Format::Zst => Box::new(ZstdChecked {
            inner: ruzstd::decoding::StreamingDecoder::new(file).map_err(|e| ArcError::Broken {
                path: name,
                reason: e.to_string(),
            })?,
            checked: false,
        }),
        Format::Zip
        | Format::SevenZ
        | Format::Ar
        | Format::Cpio
        | Format::Rpm
        | Format::Cab
        | Format::Sqlite
        | Format::Json
        | Format::Eml
        | Format::Mbox => {
            return Err(ArcError::Broken {
                path: name,
                reason: format!("a {} is not a stream", format.label()),
            })
        }
    })
}

/// The first bytes of a file, for deciding what it is. A file that cannot be read answers with
/// nothing, because the question "what format is this" is not the place to report that.
fn head_of(path: &Path) -> Option<Vec<u8>> {
    let mut file = File::open(path).ok()?;
    let mut head = [0u8; 264];
    let read = read_up_to(&mut file, &mut head).ok()?;
    Some(head[..read].to_vec())
}

/// A folder of our own for a program to write into, in the same place and under the same
/// convention `nest` uses: one directory per process, so a crash leaves something a person
/// can recognise and delete rather than scattered files with invented names.
///
/// The caller removes it. There is no guard type doing it on drop because the one caller
/// wants it gone on every path, cancellation included, and says so once.
fn scratch() -> Result<PathBuf> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir()
        .join(format!("fcmd-{}", std::process::id()))
        .join("ext")
        .join(n.to_string());
    std::fs::create_dir_all(&dir).map_err(|e| ArcError::Io {
        path: dir.display().to_string(),
        source: e,
    })?;
    Ok(dir)
}

/// `/`-separated, leading slash, no trailing slash, `.` and `..` resolved. An entry whose name
/// climbs out of the archive is pinned at the root instead: a zip that says `../../etc/passwd`
/// is a zip-slip attack, and the answer is to keep it inside, not to trust it.
pub(crate) fn normalise(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    if out.is_empty() {
        return "/".to_string();
    }
    format!("/{}", out.join("/"))
}

pub(crate) fn tar_name(name: &str) -> String {
    normalise(name)
}

/// Does a tar come out of this single compressed stream?
///
/// The first block of it, decompressed, with the word a tar header carries 257 bytes in. A
/// stream that cannot be opened or read answers no: this is a question asked to improve an
/// answer we already have, so it never turns into an error of its own.
fn tar_inside(path: &Path, format: Format) -> bool {
    let Ok(mut stream) = decoder(path, format) else {
        return false;
    };
    let mut head = [0u8; 264];
    let Ok(read) = read_up_to(&mut stream, &mut head) else {
        return false;
    };
    read > 262 && &head[257..262] == b"ustar"
}

/// The index of a single compressed stream: one entry, which is the whole of it.
///
/// The size is counted rather than read off the file. gzip does keep the uncompressed length
/// in its last four bytes, but modulo 2^32 and for the last member only, while the decoder
/// here reads the first — so a file of several members would be listed with one member's
/// bytes under another member's length. The other three keep no length at all. One pass
/// through the decoder is the only answer that is right for all four, and it is a pass that
/// is about to be made anyway: nothing opens a `.gz` except to look at what is inside it.
fn index_single(
    path: &Path,
    name: &str,
    leaf: &str,
    format: Format,
    limits: &Limits,
    cancel: &CancelToken,
) -> Result<Vec<ArcEntry>> {
    let mut stream = decoder(path, format)?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut size: u64 = 0;
    loop {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(read) => {
                size += read as u64;
                // The same ceiling `Archive::open_with` applies to every other format,
                // applied while counting: a stream that does not end must not be read to the
                // end of the disk to find out how long it is.
                if size > limits.total_bytes {
                    return Err(ArcError::Refused {
                        path: name.to_string(),
                        reason: format!(
                            "it unpacks to more than the limit of {} bytes",
                            limits.total_bytes
                        ),
                    });
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => {
                return Err(ArcError::Broken {
                    path: name.to_string(),
                    reason: e.to_string(),
                })
            }
        }
    }
    let (inner, mtime) = single_entry(path, leaf, format);
    Ok(vec![ArcEntry {
        path: normalise(&inner),
        size,
        packed: std::fs::metadata(path).ok().map(|m| m.len()),
        mtime,
        mode: None,
        kind: EntryKind::File,
        link_target: None,
        at: None,
    }])
}

/// What the one entry is called, and when it was last written.
///
/// gzip is the only one of the four that carries either. Its header has an optional name —
/// the one the file had before it was compressed, which is better than anything that could be
/// worked out from the outside — and a modification time. Everything else is named by taking
/// the compression suffix off the file's own name.
fn single_entry(path: &Path, leaf: &str, format: Format) -> (String, Option<SystemTime>) {
    let stripped = without_suffix(leaf, format);
    if format != Format::Gz {
        return (stripped, None);
    }
    let (stored, mtime) = gzip_header(path);
    (stored.unwrap_or(stripped), mtime)
}

/// The file's name without the suffix that says how it was compressed.
///
/// A name that does not say keeps its own: a row called exactly what the file is called is
/// odd to look at, and it is still the truth, which inventing a suffix would not be.
fn without_suffix(leaf: &str, format: Format) -> String {
    let lower = leaf.to_ascii_lowercase();
    for suffix in [".gz", ".bz2", ".bz", ".xz", ".zst", ".zstd"] {
        if lower.ends_with(suffix) && lower.len() > suffix.len() {
            return leaf[..leaf.len() - suffix.len()].to_string();
        }
    }
    let _ = format;
    leaf.to_string()
}

/// The optional name and the modification time out of a gzip header (RFC 1952 §2.3).
///
/// Anything unreadable gives up quietly and the caller falls back on the file's own name: a
/// header field this does not understand is not a broken archive.
fn gzip_header(path: &Path) -> (Option<String>, Option<SystemTime>) {
    let Ok(file) = File::open(path) else {
        return (None, None);
    };
    let mut reader = BufReader::new(file);
    let mut head = [0u8; 10];
    if reader.read_exact(&mut head).is_err() {
        return (None, None);
    }
    let flags = head[3];
    let stamp = u32::from_le_bytes([head[4], head[5], head[6], head[7]]);
    // Zero means "no time available", which is what gzip writes when it compressed a pipe.
    let mtime = (stamp != 0).then(|| SystemTime::UNIX_EPOCH + Duration::from_secs(stamp.into()));
    if flags & 0x08 == 0 {
        return (None, mtime);
    }
    // FEXTRA comes before the name and says its own length.
    if flags & 0x04 != 0 {
        let mut len = [0u8; 2];
        if reader.read_exact(&mut len).is_err() {
            return (None, mtime);
        }
        let extra = u64::from(u16::from_le_bytes(len));
        if io::copy(&mut reader.by_ref().take(extra), &mut io::sink()).is_err() {
            return (None, mtime);
        }
    }
    let mut bytes = Vec::new();
    for byte in reader.bytes().take(4096) {
        match byte {
            Ok(0) => break,
            Ok(b) => bytes.push(b),
            Err(_) => return (None, mtime),
        }
    }
    (safe_leaf(&bytes), mtime)
}

/// A name that came out of a file, and only if it is a name.
///
/// gzip stores it as the bytes whatever filesystem wrote it used, so it is read leniently —
/// and then anything that is not a plain leaf is refused. A `.gz` whose header says
/// `../../etc/passwd` is the same attack a zip makes with its entry names; `normalise` would
/// pin it at the root and the row would still be called `passwd`, which is a name somebody
/// else chose. Refusing it leaves the file's own name, which is always there and always true.
fn safe_leaf(bytes: &[u8]) -> Option<String> {
    let name = String::from_utf8_lossy(bytes);
    let bad = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control);
    (!bad).then(|| name.into_owned())
}

fn index_zip(
    path: &Path,
    name: &str,
    limits: &Limits,
    cancel: &CancelToken,
) -> Result<Vec<ArcEntry>> {
    let file = File::open(path).map_err(|e| ArcError::Io {
        path: name.to_string(),
        source: e,
    })?;
    let mut zip = zip::ZipArchive::new(BufReader::new(file)).map_err(|e| ArcError::Broken {
        path: name.to_string(),
        reason: e.to_string(),
    })?;
    if zip.len() > limits.entries {
        return Err(ArcError::Refused {
            path: name.to_string(),
            reason: format!(
                "{} entries, over the limit of {}",
                zip.len(),
                limits.entries
            ),
        });
    }
    let mut out = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        let item = zip.by_index(i).map_err(|e| ArcError::Broken {
            path: name.to_string(),
            reason: e.to_string(),
        })?;
        // An entry whose name we cannot read is skipped rather than losing the archive.
        let Ok(raw) = item.name() else { continue };
        let inner = normalise(&raw);
        if inner == "/" {
            continue;
        }
        let mode = item.unix_mode();
        let kind = if item.is_dir() {
            EntryKind::Dir
        } else if mode.map(|m| m & 0o170000 == 0o120000).unwrap_or(false) {
            EntryKind::Symlink
        } else {
            EntryKind::File
        };
        out.push(ArcEntry {
            path: inner,
            size: item.size(),
            packed: Some(item.compressed_size()),
            mtime: item.last_modified().and_then(system_time),
            mode,
            kind,
            link_target: None,
            at: Some(i as u64),
        });
    }
    Ok(out)
}

/// The index of a 7z: its header, read in one go.
///
/// A 7z stores a Windows attribute word per entry and, when it was written on a Unix, the Unix
/// The index of a Unix archive (`.a`, and the outer layer of a `.deb`).
///
/// The format is the oldest one here and the simplest: eight magic bytes, then for each member
/// a header of sixty ASCII bytes — name, modification time, uid, gid, mode, size — followed by
/// the member's bytes, padded to an even offset. Nothing is compressed, nothing is nested, and
/// there is no index at the end, so the whole of it is read by walking the headers once.
///
/// Two conventions put a long name somewhere other than the sixteen bytes the header has for
/// it, and both are met in the wild:
///
/// * **GNU** stores every long name in a member called `//`, newline-separated, and writes
///   `/<offset>` in the header. A name there ends at the `/` before the newline.
/// * **BSD** writes `#1/<length>` in the header and puts the name in front of the member's
///   bytes, which also shortens the data by that much.
///
/// A plain GNU name ends with `/` to allow trailing spaces; `/` and `//` are the symbol table
/// and the name table and are not files, so they are left out of the listing rather than shown
/// as entries nobody can use.
fn index_ar(
    path: &Path,
    name: &str,
    limits: &Limits,
    cancel: &CancelToken,
) -> Result<Vec<ArcEntry>> {
    const MAGIC: &[u8; 8] = b"!<arch>\n";
    const HEADER: u64 = 60;

    let mut file = BufReader::with_capacity(
        64 * 1024,
        File::open(path).map_err(|e| ArcError::Io {
            path: name.to_string(),
            source: e,
        })?,
    );
    let broken = |reason: &str| ArcError::Broken {
        path: name.to_string(),
        reason: reason.to_string(),
    };
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)
        .map_err(|_| broken("it is too short to be an archive"))?;
    if &magic != MAGIC {
        return Err(broken("it does not start with `!<arch>`"));
    }

    // The GNU long-name table, read when it is met. It always comes before the members that
    // point into it, so one pass is enough.
    let mut long_names: Vec<u8> = Vec::new();
    let mut out: Vec<ArcEntry> = Vec::new();
    let mut at = MAGIC.len() as u64;
    loop {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        if out.len() >= limits.entries {
            return Err(ArcError::Refused {
                path: name.to_string(),
                reason: format!("it holds more than {} members", limits.entries),
            });
        }
        let mut header = [0u8; HEADER as usize];
        match file.read_exact(&mut header) {
            Ok(()) => {}
            // The end of the file is the end of the archive: there is no trailer to find.
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => {
                return Err(ArcError::Io {
                    path: name.to_string(),
                    source: e,
                })
            }
        }
        if &header[58..60] != b"`\n" {
            return Err(broken("a member header does not end where it should"));
        }
        let field = |from: usize, to: usize| -> String {
            String::from_utf8_lossy(&header[from..to])
                .trim()
                .to_string()
        };
        let raw_name = field(0, 16);
        let size: u64 = field(48, 58)
            .parse()
            .map_err(|_| broken("a member does not say how long it is"))?;
        let mtime = field(16, 28)
            .parse::<i64>()
            .ok()
            .and_then(|secs| u64::try_from(secs).ok())
            .map(|secs| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs));
        // The mode is octal here, as it is in a tar, and masked to the permission bits: `ar`
        // writes `0644` and `dpkg-deb` writes `100644` for the same thing, and two archives of
        // one format should not report it two ways.
        let mode = u32::from_str_radix(&field(40, 48), 8)
            .ok()
            .map(|mode| mode & 0o7777);

        let body = at + HEADER;
        // Where the member's bytes start and how many there are, after whichever long-name
        // convention this header uses has taken its share.
        let (member, start, length) = if raw_name == "//" {
            // The name table itself: read it and move on. It is not a file.
            long_names = read_exactly(&mut file, size, &broken)?;
            skip_padding(&mut file, size, &broken)?;
            at = body + size + (size % 2);
            continue;
        } else if raw_name == "/" || raw_name == "/SYM64/" {
            // The symbol table, which is an index into this archive rather than a member of
            // it. `ar t` does not list it either.
            skip_bytes(&mut file, size + (size % 2), &broken)?;
            at = body + size + (size % 2);
            continue;
        } else if let Some(count) = raw_name.strip_prefix("#1/") {
            // BSD: the name is the first `count` bytes of the member.
            let count: u64 = count
                .parse()
                .map_err(|_| broken("a BSD long name does not say how long it is"))?;
            if count > size {
                return Err(broken("a BSD long name is longer than its member"));
            }
            let text = read_exactly(&mut file, count, &broken)?;
            let text = String::from_utf8_lossy(&text)
                .trim_end_matches('\0')
                .to_string();
            skip_bytes(&mut file, size - count + (size % 2), &broken)?;
            (text, body + count, size - count)
        } else if let Some(offset) = raw_name.strip_prefix('/') {
            // GNU: an offset into the table read above.
            let offset: usize = offset
                .parse()
                .map_err(|_| broken("a GNU long name is not an offset"))?;
            // A name in the table ends at the newline, and carries one `/` in front of it to
            // allow a trailing space. Stopping at the first `/` instead would cut
            // `src/deep/notes.txt` down to `src`: a member name may hold slashes, and in an
            // archive built from a source tree it usually does.
            let text = long_names
                .get(offset..)
                .map(|rest| {
                    let end = rest.iter().position(|b| *b == b'\n').unwrap_or(rest.len());
                    let name = &rest[..end];
                    let name = name.strip_suffix(b"/").unwrap_or(name);
                    String::from_utf8_lossy(name).to_string()
                })
                .ok_or_else(|| broken("a GNU long name points outside the name table"))?;
            skip_bytes(&mut file, size + (size % 2), &broken)?;
            (text, body, size)
        } else {
            skip_bytes(&mut file, size + (size % 2), &broken)?;
            (raw_name.trim_end_matches('/').to_string(), body, size)
        };

        at = body + size + (size % 2);
        if member.is_empty() {
            continue;
        }
        out.push(ArcEntry {
            path: normalise(&member),
            size: length,
            packed: Some(length),
            mtime,
            mode,
            kind: EntryKind::File,
            link_target: None,
            at: Some(start),
        });
    }
    Ok(out)
}

/// Exactly `count` bytes, or a broken archive. Bounded by the caller's own limit on how much
/// an entry may be, because this is used for the name table and nothing else.
fn read_exactly(
    file: &mut impl Read,
    count: u64,
    broken: &impl Fn(&str) -> ArcError,
) -> Result<Vec<u8>> {
    const MOST: u64 = 16 * 1024 * 1024;
    if count > MOST {
        return Err(broken("its name table is larger than 16 MiB"));
    }
    let mut buffer = vec![0u8; count as usize];
    file.read_exact(&mut buffer)
        .map_err(|_| broken("it ends in the middle of a member"))?;
    Ok(buffer)
}

/// Past `count` bytes without keeping them.
fn skip_bytes(file: &mut impl Read, count: u64, broken: &impl Fn(&str) -> ArcError) -> Result<()> {
    if io::copy(&mut file.take(count), &mut io::sink())
        .map_err(|_| broken("it ends in the middle of a member"))?
        != count
    {
        return Err(broken("it ends in the middle of a member"));
    }
    Ok(())
}

/// The one padding byte an odd-sized member is followed by.
fn skip_padding(file: &mut impl Read, size: u64, broken: &impl Fn(&str) -> ArcError) -> Result<()> {
    skip_bytes(file, size % 2, broken)
}

/// mode in the top half of it with bit 15 set — which is the convention `p7zip` and `7-Zip`
/// both follow, and the only way the mode of a file in a 7z is ever recorded.
fn index_sevenz(
    path: &Path,
    name: &str,
    limits: &Limits,
    cancel: &CancelToken,
    key: Option<&str>,
) -> Result<Vec<ArcEntry>> {
    let reader = sevenz_reader(path, key)?;
    let files = &reader.archive().files;
    if files.len() > limits.entries {
        return Err(ArcError::Refused {
            path: name.to_string(),
            reason: format!(
                "{} entries, over the limit of {}",
                files.len(),
                limits.entries
            ),
        });
    }
    let mut out = Vec::with_capacity(files.len());
    for file in files {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        let inner = normalise(&file.name);
        if inner == "/" {
            continue;
        }
        let mode = (file.has_windows_attributes && file.windows_attributes & 0x8000 != 0)
            .then_some(file.windows_attributes >> 16);
        let kind = if file.is_directory {
            EntryKind::Dir
        } else if mode.map(|m| m & 0o170000 == 0o120000).unwrap_or(false) {
            EntryKind::Symlink
        } else {
            EntryKind::File
        };
        out.push(ArcEntry {
            path: inner,
            size: file.size,
            // A 7z packs several files into one block, so the compressed size it reports is
            // the block's share of them rather than this file's own — which would be a column
            // that does not add up. The format says what it knows, and this is not it.
            packed: None,
            mtime: file
                .has_last_modified_date
                .then(|| SystemTime::from(file.last_modified_date)),
            mode,
            kind,
            // A 7z stores a symlink's target as the entry's *contents*, so the target is one
            // read away rather than in the header. The panel does not need it to show a row.
            link_target: None,
            at: None,
        });
    }
    Ok(out)
}

fn index_tar(
    path: &Path,
    name: &str,
    format: Format,
    limits: &Limits,
    cancel: &CancelToken,
) -> Result<Vec<ArcEntry>> {
    let mut archive = crate::tar::Reader::new(decoder(path, format)?);
    let mut out = Vec::new();
    while let Some(item) = archive.next().map_err(|e| ArcError::Broken {
        path: name.to_string(),
        reason: e.to_string(),
    })? {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        if out.len() >= limits.entries {
            return Err(ArcError::Refused {
                path: name.to_string(),
                reason: format!("more than {} entries", limits.entries),
            });
        }
        let inner = tar_name(&item.name);
        if inner == "/" {
            continue;
        }
        let kind = match item.kind() {
            crate::tar::Kind::Dir => EntryKind::Dir,
            crate::tar::Kind::File => EntryKind::File,
            crate::tar::Kind::Symlink | crate::tar::Kind::HardLink => EntryKind::Symlink,
            // Devices, fifos and the GNU bookkeeping entries: shown, never read.
            crate::tar::Kind::Other => EntryKind::Other,
        };
        out.push(ArcEntry {
            path: inner,
            size: item.size,
            packed: None,
            mtime: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(item.mtime)),
            mode: Some(item.mode),
            kind,
            link_target: (!item.link.is_empty()).then(|| item.link.clone()),
            // Only a plain tar can be seeked back to; the compressed ones re-read the stream.
            at: format.seekable().then_some(item.data_at),
        });
    }
    Ok(out)
}

/// A zip timestamp is MS-DOS local time with no zone at all. We read it as UTC, which is what
/// every other tool does, and keep `None` for the dates a zip cannot hold (before 1980).
fn system_time(at: zip::DateTime) -> Option<SystemTime> {
    if !at.is_valid() {
        return None;
    }
    let days = days_from_civil(at.year() as i64, at.month() as u32, at.day() as u32)?;
    let secs = days * 86_400
        + i64::from(at.hour()) * 3_600
        + i64::from(at.minute()) * 60
        + i64::from(at.second());
    if secs < 0 {
        return None;
    }
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs as u64))
}

/// Days from the epoch to a civil date, by Howard Hinnant's algorithm — no calendar crate
/// for one conversion.
pub(crate) fn days_from_civil(year: i64, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// The container cache of SPEC 04 §4: an open index per archive, so stepping in and out of one
/// does not read it again. Keyed by the file and its stamp, so an archive that changed on disk
/// is read afresh rather than answered from a stale index.
#[derive(Debug)]
pub struct Cache {
    limits: Limits,
    /// Most recently used last; small by design, because an index can be large.
    open: Mutex<Vec<(PathBuf, Arc<Archive>)>>,
    capacity: usize,
    /// The copies of the containers that are not files on this disk (SPEC 04 §4).
    nest: Nest,
    /// The passwords given for the archives that wanted one, by the path they were given for.
    /// They live as long as the program does and are never written anywhere.
    keys: Mutex<HashMap<String, String>>,
    /// The tables for the formats this crate does not parse, in the order they are asked —
    /// a user's own first, the shipped ones last (`docs/05-multiarc.md` §6).
    tools: Vec<Arc<crate::ext::Tool>>,
    /// And how their programs are run. One place in the program spawns a child.
    runner: Arc<dyn crate::ext::Run>,
}

impl Default for Cache {
    /// The default limits, and the shipped tables. Written out rather than derived because a
    /// derived one would leave the cache with room for nothing and no tables at all, which is
    /// not what "default" means for either.
    fn default() -> Cache {
        Cache::new(Limits::default())
    }
}

impl Cache {
    pub fn new(limits: Limits) -> Cache {
        Cache {
            limits,
            open: Mutex::new(Vec::new()),
            capacity: 4,
            nest: Nest::with_capacity(limits.depth.max(4)),
            keys: Mutex::new(HashMap::new()),
            // The run's tables — the configuration's in front of the shipped ones — so a cache
            // built anywhere sees what the panel sees.
            tools: crate::ext::tables().iter().cloned().map(Arc::new).collect(),
            runner: Arc::new(crate::ext::Spawn),
        }
    }

    /// Put these tables in front of the shipped ones, which is what a `[[archiver]]` block in
    /// the configuration does — and what a test does to drive a program of its own.
    pub fn with_tools(mut self, tools: Vec<Arc<crate::ext::Tool>>) -> Cache {
        let mut all = tools;
        all.append(&mut self.tools);
        self.tools = all;
        self
    }

    /// Run the tables' programs some other way. A test hands over canned output; nothing else
    /// has a reason to.
    pub fn with_runner(mut self, runner: Arc<dyn crate::ext::Run>) -> Cache {
        self.runner = runner;
        self
    }

    /// The program a table needs for this file and has not got, if that is the case.
    ///
    /// Asked before anything has failed, because "install `7z`" is a different message from
    /// "this is broken" and the reader can act on only one of them.
    pub fn missing_program(&self, path: &Path) -> Option<String> {
        let leaf = path.file_name()?.to_string_lossy().into_owned();
        let head = head_of(path).unwrap_or_default();
        let tool = self.tools.iter().find(|tool| tool.claims(&leaf, &head))?;
        tool.found().is_none().then(|| tool.program.clone())
    }

    /// Keep the password somebody typed for this archive, so the next read of it — the
    /// listing, then every entry opened out of it — does not ask again.
    pub fn remember_password(&self, host: &VPath, password: &str) {
        if let Ok(mut keys) = self.keys.lock() {
            keys.insert(host.to_string(), password.to_string());
        }
    }

    /// Forget it, for an archive that turned out to want a different one.
    pub fn forget_password(&self, host: &VPath) {
        if let Ok(mut keys) = self.keys.lock() {
            keys.remove(&host.to_string());
        }
    }

    fn key_for(&self, host: &VPath) -> Option<String> {
        self.keys.lock().ok()?.get(&host.to_string()).cloned()
    }

    /// Hand the cache the way to reach a container that is not on this disk — a server, a
    /// bucket. Without one, such an archive is refused with that as the reason.
    pub fn set_fetch(&self, fetch: Arc<dyn Fetch>) {
        self.nest.set_fetch(fetch);
    }

    /// The archive that holds `path`, and where `path` sits inside it.
    ///
    /// This is the call everything above should make: it is the one that knows an archive may
    /// be inside another archive, or on a server, and what to do about it.
    pub fn open_in(&self, path: &VPath, cancel: &CancelToken) -> Result<(Arc<Archive>, String)> {
        let host = path
            .leave()
            .ok_or_else(|| ArcError::Unknown(path.to_string()))?;
        let archive = self.open_host(&host, cancel)?;
        Ok((archive, path.inner_path().to_string()))
    }

    /// The archive `host` names, wherever it is.
    pub fn open_host(&self, host: &VPath, cancel: &CancelToken) -> Result<Arc<Archive>> {
        let file = self.host_file(host, cancel)?;
        self.open_with(&file, cancel, self.key_for(host).as_deref())
    }

    /// The file whose bytes are `host`'s: itself when it is one, a copy of it when it is not.
    ///
    /// The copy is made once and kept for as long as the archive it came out of has not
    /// changed, because the common act — stepping in and out of a nested archive — would
    /// otherwise unpack it again on every listing.
    pub fn host_file(&self, host: &VPath, cancel: &CancelToken) -> Result<PathBuf> {
        if host.is_local() && !host.in_container() {
            return Ok(host.as_path().to_path_buf());
        }
        if host.depth() > self.limits.depth {
            return Err(nest::too_deep(host, self.limits.depth));
        }
        if let Some(outer_path) = host.leave() {
            let outer = self.open_host(&outer_path, cancel)?;
            let inner = host.inner_path().to_string();
            let entry = outer.entry(&inner).ok_or_else(|| ArcError::NoEntry {
                path: outer_path.to_string(),
                entry: inner.clone(),
            })?;
            if entry.size > self.limits.spill_bytes {
                return Err(nest::too_big(host, entry.size, self.limits.spill_bytes));
            }
            return self.nest.spill(host, |to| {
                outer.extract(&inner, to, cancel, &mut |_| {}).map(|_| ())
            });
        }
        let fetch = self.nest.fetcher().ok_or_else(|| nest::no_fetcher(host))?;
        if let Some(size) = fetch.size(host) {
            if size > self.limits.spill_bytes {
                return Err(nest::too_big(host, size, self.limits.spill_bytes));
            }
        }
        self.nest.spill(host, |to| fetch.fetch(host, to, cancel))
    }

    /// Is this container a file on this disk — the only kind that can be written to?
    ///
    /// A rebuild of a copy would change a file that is about to be thrown away and report that
    /// it had changed the archive, so everything that writes asks this first.
    pub fn is_writable_host(host: &VPath) -> bool {
        host.is_local() && !host.in_container()
    }

    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    /// The archive at `path`, from the cache when the file has not changed under us.
    pub fn open(&self, path: &Path, cancel: &CancelToken) -> Result<Arc<Archive>> {
        let key = self.key_for(&VPath::local(path));
        self.open_with(path, cancel, key.as_deref())
    }

    /// The same, with the password for an encrypted archive.
    pub fn open_with(
        &self,
        path: &Path,
        cancel: &CancelToken,
        key: Option<&str>,
    ) -> Result<Arc<Archive>> {
        if let Ok(mut open) = self.open.lock() {
            if let Some(i) = open.iter().position(|(p, _)| p == path) {
                let (_, archive) = open[i].clone();
                if archive.is_current() {
                    let hit = open.remove(i);
                    open.push(hit);
                    return Ok(archive);
                }
                open.remove(i);
            }
        }
        let archive = Arc::new(self.read(path, cancel, key)?);
        if let Ok(mut open) = self.open.lock() {
            open.push((path.to_path_buf(), archive.clone()));
            while open.len() > self.capacity {
                open.remove(0);
            }
        }
        Ok(archive)
    }

    /// Which reader: this crate's own when it knows the format, a program on the machine when
    /// a table claims it, and neither when nothing does.
    ///
    /// The order matters and is the one it looks like: what we can parse ourselves, we parse.
    /// A table is the answer for the formats we do not implement, not an override for the ones
    /// we do — a `.zip` is never handed to `7z` because `7z` would also read it.
    fn read(&self, path: &Path, cancel: &CancelToken, key: Option<&str>) -> Result<Archive> {
        match Archive::open_with(path, &self.limits, cancel, key) {
            // Read here, but not all of it: a cabinet packed with a compressor this crate
            // has not got. A program that reads the whole of it is the better reader, so it
            // gets the archive where it is installed; where it is not, the listing made
            // here still stands, and the reason an entry will not open names the program.
            Ok(mut archive) if archive.beyond.is_some() => {
                let leaf = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let head = head_of(path).unwrap_or_default();
                let Some(tool) = self.tools.iter().find(|tool| tool.claims(&leaf, &head)) else {
                    return Ok(archive);
                };
                if tool.found().is_some() {
                    let through = Archive::open_external(
                        path,
                        tool.clone(),
                        self.runner.clone(),
                        &self.limits,
                        cancel,
                    );
                    return match through {
                        Ok(read) => Ok(read),
                        Err(ArcError::Cancelled) => Err(ArcError::Cancelled),
                        // The program could not either: what was read here is still more
                        // than nothing.
                        Err(_) => Ok(archive),
                    };
                }
                if let Some(reason) = archive.beyond.as_mut() {
                    reason.push_str(&format!(
                        " — `{}` reads it, and is not installed",
                        tool.program
                    ));
                }
                Ok(archive)
            }
            Ok(archive) => Ok(archive),
            Err(ArcError::Unknown(_)) => {
                let leaf = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let head = head_of(path).unwrap_or_default();
                let tool = self
                    .tools
                    .iter()
                    .find(|tool| tool.claims(&leaf, &head))
                    .ok_or_else(|| ArcError::Unknown(path.display().to_string()))?;
                Archive::open_external(
                    path,
                    tool.clone(),
                    self.runner.clone(),
                    &self.limits,
                    cancel,
                )
            }
            Err(other) => Err(other),
        }
    }

    /// Forget an archive, for when something has written to it.
    pub fn forget(&self, path: &Path) {
        if let Ok(mut open) = self.open.lock() {
            open.retain(|(p, _)| p != path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The same three files in every format, one of them two directories down.
    const TREE: [(&str, &[u8]); 3] = [
        ("readme.md", b"# the readme\n"),
        ("src/main.rs", b"fn main() {}\n"),
        ("src/deep/notes.txt", b"notes, two directories down\n"),
    ];

    fn write_zip(path: &Path, names: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, body) in names {
            zip.start_file(*name, options).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }

    fn write_sevenz(path: &Path, names: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut writer = sevenz_rust2::ArchiveWriter::new(file).unwrap();
        for (name, body) in names {
            let mut entry = sevenz_rust2::ArchiveEntry::new();
            entry.name = (*name).to_string();
            writer.push_archive_entry(entry, Some(*body)).unwrap();
        }
        writer.finish().unwrap();
    }

    fn tar_bytes(names: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = crate::tar::Builder::new(Vec::new());
        for (name, body) in names {
            let mut header = crate::tar::Header::new(name, crate::tar::Kind::File);
            header.size = body.len() as u64;
            header.mode = 0o644;
            header.mtime = 1_700_000_000;
            builder.append(&header, &mut &body[..]).unwrap();
        }
        builder.finish().unwrap()
    }

    /// An `ar` built by hand, so the test needs no `ar` on the machine running it.
    ///
    /// Short names go in the header as `name/`; a name too long for the sixteen bytes there
    /// goes into the GNU `//` table and the header carries `/<offset>`. Both conventions are
    /// what `ar` itself writes, and the reader is asked to understand both.
    fn ar_bytes(names: &[(&str, &[u8])]) -> Vec<u8> {
        let mut table = Vec::new();
        let mut offsets = Vec::new();
        for (name, _) in names {
            if name.len() > 15 {
                offsets.push(Some(table.len()));
                table.extend_from_slice(name.as_bytes());
                table.extend_from_slice(b"/\n");
            } else {
                offsets.push(None);
            }
        }
        let header = |name: &str, size: usize| -> Vec<u8> {
            let mut h = Vec::with_capacity(60);
            h.extend_from_slice(format!("{name:<16}").as_bytes());
            h.extend_from_slice(format!("{:<12}", 1_700_000_000u64).as_bytes());
            h.extend_from_slice(format!("{:<6}", 0).as_bytes());
            h.extend_from_slice(format!("{:<6}", 0).as_bytes());
            h.extend_from_slice(format!("{:<8}", "100644").as_bytes());
            h.extend_from_slice(format!("{size:<10}").as_bytes());
            h.extend_from_slice(b"`\n");
            assert_eq!(h.len(), 60);
            h
        };
        let mut out = b"!<arch>\n".to_vec();
        if !table.is_empty() {
            out.extend_from_slice(&header("//", table.len()));
            out.extend_from_slice(&table);
            if table.len() % 2 == 1 {
                out.push(b'\n');
            }
        }
        for (i, (name, body)) in names.iter().enumerate() {
            let in_header = match offsets[i] {
                Some(at) => format!("/{at}"),
                None => format!("{name}/"),
            };
            out.extend_from_slice(&header(&in_header, body.len()));
            out.extend_from_slice(body);
            if body.len() % 2 == 1 {
                out.push(b'\n');
            }
        }
        out
    }

    fn write_archive(path: &Path, format: Format, names: &[(&str, &[u8])]) {
        if format == Format::Zip {
            return write_zip(path, names);
        }
        if format == Format::SevenZ {
            return write_sevenz(path, names);
        }
        let tar = tar_bytes(names);
        let out = File::create(path).unwrap();
        match format {
            Format::Tar => {
                let mut out = out;
                out.write_all(&tar).unwrap();
            }
            Format::TarGz => {
                let mut enc = flate2::write::GzEncoder::new(out, flate2::Compression::fast());
                enc.write_all(&tar).unwrap();
                enc.finish().unwrap();
            }
            Format::TarBz2 => {
                let mut enc = bzip2::write::BzEncoder::new(out, bzip2::Compression::fast());
                enc.write_all(&tar).unwrap();
                enc.finish().unwrap();
            }
            Format::TarXz => {
                let mut enc =
                    lzma_rust2::XzWriter::new(out, lzma_rust2::XzOptions::with_preset(1)).unwrap();
                enc.write_all(&tar).unwrap();
                enc.finish().unwrap();
            }
            Format::TarZst => {
                let mut out = out;
                let packed = ruzstd::encoding::compress_to_vec(
                    &tar[..],
                    ruzstd::encoding::CompressionLevel::Fastest,
                );
                out.write_all(&packed).unwrap();
            }
            Format::Ar => {
                let mut out = out;
                out.write_all(&ar_bytes(names)).unwrap();
            }
            Format::Cpio | Format::Rpm => {
                let members: Vec<(&str, u32, &[u8])> =
                    names.iter().map(|(n, b)| (*n, 0o100644, *b)).collect();
                let cpio = crate::cpio::tests::newc(&members);
                let mut out = out;
                if format == Format::Rpm {
                    out.write_all(&crate::cpio::tests::rpm(&cpio)).unwrap();
                } else {
                    out.write_all(&cpio).unwrap();
                }
            }
            Format::Cab => {
                // One MSZIP folder, with a cabinet's own separator in the names.
                let names: Vec<(String, &[u8])> = names
                    .iter()
                    .map(|(name, body)| (name.replace('/', "\\"), *body))
                    .collect();
                let bytes = crate::cab::tests::cabinet(&[crate::cab::tests::Unit {
                    method: 1,
                    members: names
                        .iter()
                        .map(|(name, body)| crate::cab::tests::Member { name, body })
                        .collect(),
                    blocks: None,
                }]);
                let mut out = out;
                out.write_all(&bytes).unwrap();
            }
            Format::Zip | Format::SevenZ => unreachable!("handled above"),
            f if f.is_single() => unreachable!("a {} holds one file: write_single", f.label()),
            _ => unreachable!("no writer for {}", format.label()),
        }
    }

    /// One compressed file, with no tar inside it: what `gzip notes.txt` leaves behind.
    fn write_single(path: &Path, format: Format, bytes: &[u8]) {
        let out = File::create(path).unwrap();
        match format {
            Format::Gz => {
                let mut enc = flate2::write::GzEncoder::new(out, flate2::Compression::fast());
                enc.write_all(bytes).unwrap();
                enc.finish().unwrap();
            }
            Format::Bz2 => {
                let mut enc = bzip2::write::BzEncoder::new(out, bzip2::Compression::fast());
                enc.write_all(bytes).unwrap();
                enc.finish().unwrap();
            }
            Format::Xz => {
                let mut enc =
                    lzma_rust2::XzWriter::new(out, lzma_rust2::XzOptions::with_preset(1)).unwrap();
                enc.write_all(bytes).unwrap();
                enc.finish().unwrap();
            }
            Format::Zst => {
                let mut out = out;
                let packed = ruzstd::encoding::compress_to_vec(
                    bytes,
                    ruzstd::encoding::CompressionLevel::Fastest,
                );
                out.write_all(&packed).unwrap();
            }
            other => unreachable!("{} is not a single stream", other.label()),
        }
    }

    fn open(path: &Path) -> Archive {
        Archive::open(path, &Limits::default(), &CancelToken::new()).expect("the archive")
    }

    #[test]
    fn every_format_lists_the_same_tree_and_reads_the_same_bytes() {
        let dir = tempfile::tempdir().unwrap();
        // The single streams have no tree to list: they get their own test below.
        // A database holds tables, not a tree of files: `sqlite.rs` has its own tests.
        for format in Format::ALL.into_iter().filter(|f| {
            f.holds_many()
                && !matches!(
                    f,
                    Format::Sqlite | Format::Json | Format::Eml | Format::Mbox
                )
        }) {
            let path = dir.path().join(format!("tree.{}", format.extension()));
            write_archive(&path, format, &TREE);
            let archive = open(&path);
            assert_eq!(archive.format(), Some(format), "{}", format.label());

            let root: Vec<String> = archive
                .list("/")
                .iter()
                .map(|e| e.name_str().into_owned())
                .collect();
            assert_eq!(root, ["readme.md", "src"], "{}", format.label());

            let src: Vec<String> = archive
                .list("/src")
                .iter()
                .map(|e| e.name_str().into_owned())
                .collect();
            assert_eq!(src, ["deep", "main.rs"], "{}", format.label());

            assert!(archive.is_dir("/src") && archive.is_dir("/src/deep"));
            assert!(!archive.is_dir("/readme.md"));
            // The root is a directory even in an archive that never says so.
            assert!(archive.is_dir("/"));

            let bytes = archive
                .read("/src/deep/notes.txt", &CancelToken::new())
                .expect("the entry");
            assert_eq!(bytes, TREE[2].1, "{}", format.label());
            // And the first entry too, so a seekable format is not read off by one.
            assert_eq!(
                archive.read("/readme.md", &CancelToken::new()).unwrap(),
                TREE[0].1,
                "{}",
                format.label()
            );

            let (files, dirs, bytes) = archive.totals();
            assert_eq!((files, dirs), (3, 2), "{}", format.label());
            assert_eq!(bytes, TREE.iter().map(|(_, b)| b.len() as u64).sum::<u64>());
        }
    }

    #[test]
    fn a_directory_nobody_wrote_down_is_still_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("deep.zip");
        // One entry, three levels down, and no entries for the levels themselves.
        write_zip(&path, &[("a/b/c.txt", b"x")]);
        let archive = open(&path);
        assert_eq!(archive.list("/").len(), 1);
        assert_eq!(archive.list("/")[0].name_str(), "a");
        assert_eq!(archive.list("/a")[0].name_str(), "b");
        assert_eq!(archive.list("/a/b")[0].name_str(), "c.txt");
        assert!(archive.is_dir("/a") && archive.is_dir("/a/b"));
    }

    #[test]
    fn an_entry_that_climbs_out_of_the_archive_stays_inside_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("slip.zip");
        write_zip(
            &path,
            &[("../../etc/passwd", b"root:x:0:0\n"), ("./ok.txt", b"ok")],
        );
        let archive = open(&path);
        // The name is pinned at the root rather than trusted, so no extraction of this
        // archive can write outside where it was told to.
        assert!(archive.entry("/etc/passwd").is_some());
        assert_eq!(
            archive
                .list("/")
                .iter()
                .map(|e| e.name_str().into_owned())
                .collect::<Vec<_>>(),
            ["etc", "ok.txt"]
        );
    }

    #[test]
    fn extraction_writes_the_file_where_it_is_told_and_counts_the_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tree.tar.gz");
        write_archive(&path, Format::TarGz, &TREE);
        let archive = open(&path);

        let out = dir.path().join("out/src/deep/notes.txt");
        let mut seen = 0;
        let written = archive
            .extract("/src/deep/notes.txt", &out, &CancelToken::new(), &mut |n| {
                seen = n
            })
            .expect("extraction");
        assert_eq!(written, TREE[2].1.len() as u64);
        assert_eq!(seen, written, "the progress ends where the file does");
        assert_eq!(std::fs::read(&out).unwrap(), TREE[2].1);

        // A directory is not a file: say so rather than writing an empty one.
        assert!(matches!(
            archive.extract(
                "/src",
                &dir.path().join("x"),
                &CancelToken::new(),
                &mut |_| {}
            ),
            Err(ArcError::NoEntry { .. })
        ));
    }

    #[test]
    fn under_gives_a_directory_and_everything_in_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tree.zip");
        write_archive(&path, Format::Zip, &TREE);
        let archive = open(&path);
        let mut under: Vec<&str> = archive
            .under("/src")
            .iter()
            .map(|e| e.path.as_str())
            .collect();
        under.sort();
        assert_eq!(
            under,
            ["/src", "/src/deep", "/src/deep/notes.txt", "/src/main.rs"]
        );
        assert_eq!(archive.under("/").len(), archive.entries().len());
        // `/src` is not `/srcery`: a name prefix is not a path prefix.
        assert!(archive.under("/sr").is_empty());
    }

    #[test]
    fn the_guards_refuse_a_bomb_and_say_which_limit_fired() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tree.zip");
        write_archive(&path, Format::Zip, &TREE);

        let few = Limits {
            entries: 2,
            ..Limits::default()
        };
        let err = Archive::open(&path, &few, &CancelToken::new()).unwrap_err();
        assert!(
            matches!(&err, ArcError::Refused { reason, .. } if reason.contains("limit of 2")),
            "{err}"
        );

        let small = Limits {
            total_bytes: 8,
            ..Limits::default()
        };
        let err = Archive::open(&path, &small, &CancelToken::new()).unwrap_err();
        assert!(
            matches!(&err, ArcError::Refused { reason, .. } if reason.contains("unpacks to")),
            "{err}"
        );

        // A tar of a megabyte of zeroes compresses far past the ratio limit, and that is
        // exactly the shape of a bomb.
        let zeroes = vec![0u8; 1 << 20];
        let bomb = dir.path().join("bomb.tar.gz");
        write_archive(&bomb, Format::TarGz, &[("zeroes", &zeroes)]);
        let err = Archive::open(
            &bomb,
            &Limits {
                ratio: 100,
                ..Limits::default()
            },
            &CancelToken::new(),
        )
        .unwrap_err();
        assert!(
            matches!(&err, ArcError::Refused { reason, .. } if reason.contains('×')),
            "{err}"
        );
        // And with the guard raised it reads fine: the limit is a decision, not a verdict.
        assert!(Archive::open(&bomb, &Limits::unlimited(), &CancelToken::new()).is_ok());
    }

    #[test]
    fn a_file_that_is_not_an_archive_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, b"just text").unwrap();
        assert!(matches!(
            Archive::open(&path, &Limits::default(), &CancelToken::new()),
            Err(ArcError::Unknown(_))
        ));
    }

    #[test]
    fn the_cache_reads_an_archive_once_and_notices_when_it_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tree.zip");
        write_archive(&path, Format::Zip, &TREE);
        let cache = Cache::new(Limits::default());

        let first = cache.open(&path, &CancelToken::new()).unwrap();
        let again = cache.open(&path, &CancelToken::new()).unwrap();
        assert!(Arc::ptr_eq(&first, &again), "the index was read twice");

        // Rewritten under us: the stamp no longer matches, so the index is read afresh.
        std::thread::sleep(std::time::Duration::from_millis(10));
        write_zip(&path, &[("only.txt", b"one")]);
        let third = cache.open(&path, &CancelToken::new()).unwrap();
        assert!(!Arc::ptr_eq(&first, &third), "a stale index was handed out");
        assert_eq!(third.list("/").len(), 1);

        cache.forget(&path);
        let fourth = cache.open(&path, &CancelToken::new()).unwrap();
        assert!(!Arc::ptr_eq(&third, &fourth));
    }

    // ── an archive a program on the machine read ──────────────────────────────────────────

    /// One run the fake runner was asked for: the program, its arguments, and the folder it
    /// was given to work in.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Call {
        program: String,
        args: Vec<String>,
        cwd: Option<PathBuf>,
    }

    /// A runner that answers from a script rather than from a program, and remembers what it
    /// was asked. Every test about the grammar and the plumbing uses this, so none of them
    /// depends on what is installed where they run.
    #[derive(Debug)]
    struct Canned {
        out: String,
        code: i32,
        err: String,
        /// Each call, in order.
        asked: Mutex<Vec<Call>>,
        /// A file to write into that folder, as an extraction would.
        writes: Option<(String, Vec<u8>)>,
    }

    impl Canned {
        fn listing(out: &str) -> Canned {
            Canned {
                out: out.to_string(),
                code: 0,
                err: String::new(),
                asked: Mutex::new(Vec::new()),
                writes: None,
            }
        }
        fn failing(code: i32, err: &str) -> Canned {
            Canned {
                out: String::new(),
                code,
                err: err.to_string(),
                asked: Mutex::new(Vec::new()),
                writes: None,
            }
        }
        fn calls(&self) -> Vec<Call> {
            self.asked.lock().expect("not poisoned").clone()
        }
    }

    impl crate::ext::Run for Canned {
        fn run(
            &self,
            program: &Path,
            args: &[OsString],
            cwd: Option<&Path>,
            cancel: &CancelToken,
        ) -> std::result::Result<crate::ext::Ran, crate::ext::ExtError> {
            if cancel.is_cancelled() {
                return Err(crate::ext::ExtError::Cancelled);
            }
            if let Ok(mut asked) = self.asked.lock() {
                asked.push(Call {
                    program: program.display().to_string(),
                    args: args
                        .iter()
                        .map(|a| a.to_string_lossy().into_owned())
                        .collect(),
                    cwd: cwd.map(Path::to_path_buf),
                });
            }
            if let (Some((name, bytes)), Some(cwd)) = (&self.writes, cwd) {
                let at = cwd.join(name);
                if let Some(parent) = at.parent() {
                    std::fs::create_dir_all(parent).expect("the folder");
                }
                std::fs::write(at, bytes).expect("it writes");
            }
            Ok(crate::ext::Ran {
                code: Some(self.code),
                out: self.out.clone(),
                err: self.err.clone(),
            })
        }
    }

    /// A table for a program that is certainly there, so `Tool::found` says yes wherever
    /// these run: the test binary itself. Nothing is ever really spawned — `Canned` answers.
    fn table() -> crate::ext::Tool {
        let mut tool = crate::ext::builtin_tools()
            .into_iter()
            .next()
            .expect("the shipped table");
        tool.program = std::env::current_exe()
            .expect("this test binary")
            .display()
            .to_string();
        tool
    }

    /// What `7z l` printed for a `.wim` on the machine this was written on, 7-Zip 23.01 —
    /// chosen because a directory row there has *both* numeric columns empty, which is the
    /// case a column grammar gets wrong.
    const WIM_LISTING: &str = concat!(
        "\n7-Zip 23.01 (x64) : Copyright (c) 1999-2023 Igor Pavlov : 2023-06-20\n\n",
        "Listing archive: prova.wim\n\n",
        "--\nPath = prova.wim\nType = wim\nImages = 1\n\n",
        "   Date      Time    Attr         Size   Compressed  Name\n",
        "------------------- ----- ------------ ------------  ------------------------\n",
        "2026-11-04 09:57:13 D....                            src\n",
        "2026-11-04 09:57:13 D....                            src/deep\n",
        "2026-11-04 09:57:13 ....A            8            8  readme.md\n",
        "2026-11-04 09:57:13 ....A            5            5  src/main.rs\n",
        "2026-11-04 09:57:13 ....A            1            1  src/deep/x.bin\n",
        "------------------- ----- ------------ ------------  ------------------------\n",
        "2026-11-04 09:57:13                 14           14  3 files, 2 folders\n",
    );

    #[test]
    fn a_program_on_the_machine_lists_an_archive_this_crate_cannot_parse() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prova.wim");
        std::fs::write(&path, b"MSWIM\x00\x00\x00 and some more").unwrap();
        let runner = Arc::new(Canned::listing(WIM_LISTING));
        let archive = Archive::open_external(
            &path,
            Arc::new(table()),
            runner.clone(),
            &Limits::default(),
            &CancelToken::new(),
        )
        .expect("it lists");

        // No `Format`, because this crate parses nothing here; the frame has the table's name.
        assert_eq!(archive.format(), None);
        assert_eq!(archive.label(), "7z");

        let root: Vec<String> = archive
            .list("/")
            .iter()
            .map(|e| e.name_str().into_owned())
            .collect();
        assert_eq!(root, ["readme.md", "src"]);
        let src: Vec<String> = archive
            .list("/src")
            .iter()
            .map(|e| e.name_str().into_owned())
            .collect();
        assert_eq!(src, ["deep", "main.rs"]);
        assert!(archive.is_dir("/src") && archive.is_dir("/src/deep"));

        assert_eq!(archive.entry("/readme.md").expect("there").size, 8);
        assert_eq!(archive.entry("/src/deep/x.bin").expect("there").size, 1);
        // A listing prints a mode as letters, not bits, so nothing is invented.
        assert_eq!(archive.entry("/readme.md").expect("there").mode, None);

        // One call, no shell, and the archive's path as one argument of its own.
        let calls = runner.calls();
        assert_eq!(calls.len(), 1, "{calls:?}");
        let call = &calls[0];
        assert_eq!(call.args[0], "l", "the table's own first word");
        assert!(
            call.args.iter().any(|a| a == &path.display().to_string()),
            "{:?}",
            call.args
        );
        assert_eq!(
            call.cwd, None,
            "listing writes nothing, so it needs no folder"
        );
    }

    #[test]
    fn getting_one_entry_out_runs_the_program_again_into_a_folder_of_our_own() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prova.wim");
        std::fs::write(&path, b"MSWIM\x00\x00\x00").unwrap();
        let runner = Arc::new(Canned {
            writes: Some(("src/main.rs".to_string(), b"ciao\n".to_vec())),
            ..Canned::listing(WIM_LISTING)
        });
        let archive = Archive::open_external(
            &path,
            Arc::new(table()),
            runner.clone(),
            &Limits::default(),
            &CancelToken::new(),
        )
        .expect("it lists");

        let out = archive
            .read("/src/main.rs", &CancelToken::new())
            .expect("it comes out");
        assert_eq!(out, b"ciao\n");

        let calls = runner.calls();
        assert_eq!(calls.len(), 2, "the listing, then the extraction");
        let call = &calls[1];
        assert_eq!(call.args[0], "x", "extract");
        let folder = call.cwd.as_ref().expect("an extraction is given a folder");
        // The names never go on the command line: one argument carries a list file's path.
        assert!(
            call.args.iter().any(|a| a.starts_with("-i@")),
            "the names go in a file: {:?}",
            call.args
        );
        assert!(
            !call.args.iter().any(|a| a.contains("main.rs")),
            "no name on the line: {:?}",
            call.args
        );
        // And the folder is gone, with the program's files in it.
        assert!(!folder.exists(), "{folder:?} was left behind");
    }

    #[test]
    fn a_table_whose_program_is_not_installed_names_the_program() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prova.wim");
        std::fs::write(&path, b"MSWIM").unwrap();
        let mut tool = table();
        tool.program = "fcmd-no-such-archiver".to_string();
        let why = Archive::open_external(
            &path,
            Arc::new(tool),
            Arc::new(Canned::listing("")),
            &Limits::default(),
            &CancelToken::new(),
        )
        .expect_err("it cannot be read");
        let said = why.to_string();
        assert!(said.contains("fcmd-no-such-archiver"), "{said}");
        assert!(said.contains("not installed"), "{said}");
    }

    #[test]
    fn a_program_that_says_no_hands_its_own_reason_over() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prova.wim");
        std::fs::write(&path, b"MSWIM").unwrap();
        let why = Archive::open_external(
            &path,
            Arc::new(table()),
            Arc::new(Canned::failing(2, "Cannot open the file as an archive\n")),
            &Limits::default(),
            &CancelToken::new(),
        )
        .expect_err("it said no");
        let said = why.to_string();
        assert!(
            said.contains("Cannot open the file as an archive"),
            "{said}"
        );
        assert!(said.contains('2'), "the exit code is in it: {said}");
    }

    #[test]
    fn the_cache_asks_this_crate_first_and_a_table_only_for_what_is_left() {
        let dir = tempfile::tempdir().unwrap();
        // A zip is read here, and never handed to a program that would also read it.
        let zip = dir.path().join("tree.zip");
        write_archive(&zip, Format::Zip, &TREE);
        let runner = Arc::new(Canned::listing(WIM_LISTING));
        let cache = Cache::new(Limits::default())
            .with_tools(vec![Arc::new(table())])
            .with_runner(runner.clone());
        let read = cache.open(&zip, &CancelToken::new()).expect("the zip");
        assert_eq!(read.format(), Some(Format::Zip));
        assert!(
            runner.calls().is_empty(),
            "no program was asked about a zip"
        );

        // And a format this crate does not know goes to the table.
        let wim = dir.path().join("prova.wim");
        std::fs::write(&wim, b"MSWIM\x00\x00\x00").unwrap();
        let read = cache.open(&wim, &CancelToken::new()).expect("the wim");
        assert_eq!(read.format(), None);
        assert_eq!(read.entry("/readme.md").expect("there").size, 8);
        assert_eq!(runner.calls().len(), 1);
    }

    #[test]
    fn a_file_nothing_claims_is_still_not_an_archive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, b"just text\n").unwrap();
        let cache = Cache::new(Limits::default());
        match cache.open(&path, &CancelToken::new()) {
            Err(ArcError::Unknown(_)) => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_cache_says_which_program_is_missing_before_anything_has_failed() {
        let dir = tempfile::tempdir().unwrap();
        let wim = dir.path().join("prova.wim");
        std::fs::write(&wim, b"MSWIM").unwrap();
        let mut absent = table();
        absent.program = "fcmd-no-such-archiver".to_string();
        let cache = Cache::new(Limits::default()).with_tools(vec![Arc::new(absent)]);
        assert_eq!(
            cache.missing_program(&wim).as_deref(),
            Some("fcmd-no-such-archiver")
        );
        // A table whose program *is* there has nothing to say, and nor has a file no table
        // claims at all.
        let here = Cache::new(Limits::default()).with_tools(vec![Arc::new(table())]);
        assert_eq!(here.missing_program(&wim), None);
        let txt = dir.path().join("notes.txt");
        std::fs::write(&txt, b"x").unwrap();
        assert_eq!(here.missing_program(&txt), None);
    }

    /// The real thing, when the machine has it: `7z` makes a `.wim` — a container this crate
    /// has no reader for — and the shipped table lists it and gets a file out of it.
    ///
    /// Skipped where `7z` is not installed rather than failed, because what it would be
    /// reporting then is somebody's package manager.
    #[test]
    fn the_shipped_table_really_reads_a_wim_when_7z_is_here() {
        let Some(program) = crate::ext::which("7z") else {
            eprintln!("no 7z on this machine: skipped");
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), b"ciao\n").unwrap();
        std::fs::write(dir.path().join("readme.md"), b"leggimi\n").unwrap();
        let made = std::process::Command::new(&program)
            .args(["a", "-bd", "-twim", "prova.wim", "src", "readme.md"])
            .current_dir(dir.path())
            .output()
            .expect("7z runs");
        assert!(made.status.success(), "{made:?}");

        let cache = Cache::new(Limits::default());
        let path = dir.path().join("prova.wim");
        let archive = cache.open(&path, &CancelToken::new()).expect("it opens");
        assert_eq!(archive.format(), None, "no reader of ours parsed it");
        let root: Vec<String> = archive
            .list("/")
            .iter()
            .map(|e| e.name_str().into_owned())
            .collect();
        assert_eq!(root, ["readme.md", "src"]);
        assert_eq!(archive.entry("/readme.md").expect("there").size, 8);
        let out = archive
            .read("/src/main.rs", &CancelToken::new())
            .expect("it comes out");
        assert_eq!(out, b"ciao\n", "the bytes 7z wrote are the bytes we read");
    }

    // ── one compressed file, which is an archive of one entry ─────────────────────────────

    #[test]
    fn a_single_compressed_file_is_an_archive_of_one_entry() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = b"the quick brown fox\n";
        for format in Format::ALL.into_iter().filter(|f| f.is_single()) {
            let path = dir.path().join(format!("notes.txt.{}", format.extension()));
            write_single(&path, format, bytes);
            let archive = open(&path);
            assert_eq!(archive.format(), Some(format), "{}", format.label());

            // One row, named after the file without the suffix that says how it was packed.
            let root: Vec<String> = archive
                .list("/")
                .iter()
                .map(|e| e.name_str().into_owned())
                .collect();
            assert_eq!(root, ["notes.txt"], "{}", format.label());
            assert!(!archive.is_dir("/notes.txt"), "{}", format.label());

            let entry = archive.entry("/notes.txt").expect("the one entry");
            assert_eq!(entry.size, bytes.len() as u64, "{}", format.label());
            assert_eq!(entry.kind, EntryKind::File, "{}", format.label());

            let out = archive
                .read("/notes.txt", &CancelToken::new())
                .expect("it reads");
            assert_eq!(out, bytes, "{}", format.label());
        }
    }

    #[test]
    fn a_name_with_no_suffix_to_take_off_keeps_its_own() {
        let dir = tempfile::tempdir().unwrap();
        // No `.bz2` on the end, so there is nothing to strip and nothing to invent.
        let path = dir.path().join("dump");
        write_single(&path, Format::Bz2, b"x");
        let archive = open(&path);
        assert_eq!(archive.format(), Some(Format::Bz2));
        let root: Vec<String> = archive
            .list("/")
            .iter()
            .map(|e| e.name_str().into_owned())
            .collect();
        assert_eq!(root, ["dump"]);
    }

    #[test]
    fn a_tarball_under_a_bare_suffix_is_still_a_tarball() {
        let dir = tempfile::tempdir().unwrap();
        // `.gz` says gzip and nothing else; what comes out is a tar, and that wins.
        let path = dir.path().join("backup.gz");
        let tar = tar_bytes(&TREE);
        let out = File::create(&path).unwrap();
        let mut enc = flate2::write::GzEncoder::new(out, flate2::Compression::fast());
        enc.write_all(&tar).unwrap();
        enc.finish().unwrap();

        let archive = open(&path);
        assert_eq!(archive.format(), Some(Format::TarGz), "the peek settles it");
        let root: Vec<String> = archive
            .list("/")
            .iter()
            .map(|e| e.name_str().into_owned())
            .collect();
        assert_eq!(root, ["readme.md", "src"]);
    }

    #[test]
    fn gzip_carries_the_name_the_file_had_and_it_is_used() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("blob.gz");
        let out = File::create(&path).unwrap();
        let mut enc = flate2::GzBuilder::new()
            .filename("report.csv")
            .write(out, flate2::Compression::fast());
        enc.write_all(b"a,b\n").unwrap();
        enc.finish().unwrap();

        let archive = open(&path);
        let root: Vec<String> = archive
            .list("/")
            .iter()
            .map(|e| e.name_str().into_owned())
            .collect();
        assert_eq!(root, ["report.csv"], "the stored name beats the file's");
    }

    #[test]
    fn a_name_inside_a_gzip_that_is_a_path_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt.gz");
        let out = File::create(&path).unwrap();
        let mut enc = flate2::GzBuilder::new()
            .filename("../../etc/passwd")
            .write(out, flate2::Compression::fast());
        enc.write_all(b"x").unwrap();
        enc.finish().unwrap();

        let archive = open(&path);
        let root: Vec<String> = archive
            .list("/")
            .iter()
            .map(|e| e.name_str().into_owned())
            .collect();
        // Not `passwd` and not a path: a name that tried to be one is not used at all.
        assert_eq!(root, ["notes.txt"]);
        assert!(archive.entry("/passwd").is_none());
    }

    #[test]
    fn a_single_stream_that_unpacks_past_the_limit_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("zeroes.gz");
        write_single(&path, Format::Gz, &vec![0u8; 64 * 1024]);
        let limits = Limits {
            total_bytes: 1024,
            ..Limits::default()
        };
        match Archive::open(&path, &limits, &CancelToken::new()) {
            Err(ArcError::Refused { reason, .. }) => {
                assert!(reason.contains("1024"), "{reason}")
            }
            other => panic!("it should be refused: {other:?}"),
        }
    }

    /// Testing an archive has to answer two questions: does every entry come out, and when
    /// one does not, which one and why. A corrupted zip answers the second.
    #[test]
    fn a_whole_archive_tests_whole_and_a_damaged_one_names_what_is_broken() {
        let dir = tempfile::tempdir().unwrap();
        // A database holds tables, not a tree of files: `sqlite.rs` has its own tests.
        for format in Format::ALL.into_iter().filter(|f| {
            f.holds_many()
                && !matches!(
                    f,
                    Format::Sqlite | Format::Json | Format::Eml | Format::Mbox
                )
        }) {
            let path = dir.path().join(format!("tree.{}", format.extension()));
            write_archive(&path, format, &TREE);
            let mut seen: Vec<String> = Vec::new();
            let report = open(&path)
                .verify(&CancelToken::new(), &mut |name, _, _| {
                    seen.push(name.trim_start_matches('/').to_string())
                })
                .expect("a whole archive");
            assert!(report.is_whole(), "{}: {:?}", format.label(), report.bad);
            assert_eq!(report.members, 3, "{}", format.label());
            let total: u64 = TREE.iter().map(|(_, b)| b.len() as u64).sum();
            assert_eq!(report.bytes, total, "{}", format.label());
            // Every member is named as it is read, so the progress bar can say where it is.
            // More than once each: when it is begun, and as its bytes come out.
            seen.sort();
            seen.dedup();
            assert_eq!(
                seen,
                ["readme.md", "src/deep/notes.txt", "src/main.rs"],
                "{}",
                format.label()
            );
        }

        // A zip whose deflated bytes have been scribbled on: the entry is still listed, and
        // the CRC is what catches it. The first entry's payload starts after its local
        // header — thirty bytes, then the name, then the extra field — and that is what is
        // damaged, rather than a byte in the middle of the file that might be a header.
        let path = dir.path().join("broken.zip");
        write_zip(&path, &TREE);
        let mut bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        let name_len = u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
        let extra_len = u16::from_le_bytes([bytes[28], bytes[29]]) as usize;
        let at = 30 + name_len + extra_len;
        bytes[at] ^= 0xff;
        std::fs::write(&path, &bytes).unwrap();
        let report = open(&path)
            .verify(&CancelToken::new(), &mut |_, _, _| {})
            .expect("the listing still reads");
        assert!(!report.is_whole(), "the damage went unnoticed: {report:?}");
        assert_eq!(report.members, 3);
    }

    #[test]
    fn a_zip_timestamp_becomes_a_real_time() {
        // The epoch, and a date a zip can hold.
        assert_eq!(days_from_civil(1970, 1, 1), Some(0));
        assert_eq!(days_from_civil(2000, 1, 1), Some(10_957));
        assert_eq!(days_from_civil(1980, 1, 1), Some(3_652));
        assert_eq!(days_from_civil(2026, 10, 24), Some(20_750));
        assert_eq!(days_from_civil(2026, 13, 1), None);

        let dir = tempfile::tempdir().unwrap();
        // A zip written with a stamp of its own comes back with that stamp, read as UTC.
        let stamped = dir.path().join("stamped.zip");
        let file = File::create(&stamped).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(
            "readme.md",
            zip::write::SimpleFileOptions::default().last_modified_time(
                zip::DateTime::from_date_and_time(2026, 10, 24, 9, 30, 0).unwrap(),
            ),
        )
        .unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();

        let archive = open(&stamped);
        let entry = archive.entry("/readme.md").unwrap();
        let when = entry.mtime.expect("a zip carries a modification time");
        assert_eq!(
            when,
            SystemTime::UNIX_EPOCH + Duration::from_secs(20_750 * 86_400 + 9 * 3_600 + 30 * 60)
        );
    }
    /// A Unix archive, listed and read, with both conventions for a name too long for the
    /// sixteen bytes a header has.
    ///
    /// Verified against what GNU `ar` and `dpkg-deb` actually write before this was committed;
    /// the fixture is built here rather than shelled out for, so the test needs no `ar` on the
    /// machine running it.
    #[test]
    fn an_ar_lists_its_members_and_reads_them_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("libstuff.a");
        let long = "un-nome-molto-lungo-oltre-sedici.o";
        write_archive(
            &path,
            Format::Ar,
            &[
                ("short.o", b"uno" as &[u8]),
                (long, b"due due"),
                ("terzo.o", b"tre"),
            ],
        );
        let archive = open(&path);
        assert_eq!(archive.format, Some(Format::Ar));
        let names: Vec<&str> = archive.entries().iter().map(|e| e.path.as_str()).collect();
        assert_eq!(
            names,
            vec!["/short.o", &format!("/{long}")[..], "/terzo.o"],
            "a long name comes out of the GNU table whole"
        );
        // An odd-sized member is padded to an even offset, so the one after it is only found
        // if the padding was stepped over.
        assert_eq!(
            archive.read("/terzo.o", &CancelToken::new()).unwrap(),
            b"tre"
        );
        assert_eq!(
            archive
                .read(&format!("/{long}"), &CancelToken::new())
                .unwrap(),
            b"due due"
        );
        // The mode is the permission bits, whichever of the two ways the writer wrote it.
        assert_eq!(archive.entries()[0].mode, Some(0o644));
    }

    /// The symbol table and the name table are an index into the archive, not members of it:
    /// `ar t` does not list them and neither does the panel.
    #[test]
    fn the_tables_of_an_ar_are_not_files_in_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("withindex.a");
        // Built by hand, because `ar_bytes` writes no symbol table: `/` first, as `ar -s` puts
        // it, then one ordinary member.
        let header = |name: &str, size: usize| {
            format!(
                "{name:<16}{:<12}{:<6}{:<6}{:<8}{size:<10}`\n",
                0, 0, 0, "100644"
            )
        };
        let mut bytes = b"!<arch>\n".to_vec();
        bytes.extend_from_slice(header("/", 4).as_bytes());
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes.extend_from_slice(header("real.o/", 2).as_bytes());
        bytes.extend_from_slice(b"ok");
        std::fs::write(&path, &bytes).unwrap();

        let archive = open(&path);
        let names: Vec<&str> = archive.entries().iter().map(|e| e.path.as_str()).collect();
        assert_eq!(names, vec!["/real.o"], "the symbol table is not a file");
        assert_eq!(archive.read("/real.o", &CancelToken::new()).unwrap(), b"ok");
    }

    /// A `.deb` is an `ar` of three members, and the one that holds the package's files is a
    /// compressed tar — so opening a package is the archive-in-an-archive this program already
    /// does, with nothing new underneath it (SPEC 04 §4).
    #[test]
    fn a_deb_is_an_ar_and_the_tar_inside_it_opens() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("finto_1.0_all.deb");
        let mut packed = Vec::new();
        {
            let tar = tar_bytes(&[("usr/bin/finto", b"#!/bin/sh\n" as &[u8])]);
            let mut enc = flate2::write::GzEncoder::new(&mut packed, flate2::Compression::fast());
            enc.write_all(&tar).unwrap();
            enc.finish().unwrap();
        }
        write_archive(
            &path,
            Format::Ar,
            &[
                ("debian-binary", b"2.0\n" as &[u8]),
                ("control.tar.gz", &packed[..]),
                ("data.tar.gz", &packed[..]),
            ],
        );
        let outer = open(&path);
        assert_eq!(outer.format, Some(Format::Ar));
        assert_eq!(
            outer.read("/debian-binary", &CancelToken::new()).unwrap(),
            b"2.0\n"
        );

        // And into the tar, through the cache, which is what the panel does.
        let cache = Cache::default();
        let inner = fcmd_core::VPath::local(&path)
            .enter("")
            .join("data.tar.gz")
            .enter("");
        let (nested, _) = cache
            .open_in(&inner, &CancelToken::new())
            .expect("the tar inside the package");
        assert_eq!(nested.format, Some(Format::TarGz));
        assert!(
            nested
                .entries()
                .iter()
                .any(|e| e.path == "/usr/bin/finto" && e.size == 10),
            "the package's own file: {:?}",
            nested.entries()
        );
    }

    /// A format this program reads and does not write is not offered for a new archive, and
    /// refuses by name if something asks anyway.
    #[test]
    fn a_format_that_is_only_read_is_not_offered_for_a_new_archive() {
        assert!(!Format::Ar.writable());
        assert!(
            !Format::writable_formats().contains(&Format::Ar),
            "the dialog's list is the writable ones"
        );
        assert_eq!(
            Format::writable_formats().len(),
            Format::ALL.into_iter().filter(|f| f.writable()).count(),
            "and it is every other one"
        );
        for f in Format::ALL {
            assert_eq!(
                Format::writable_formats().contains(&f),
                f.writable(),
                "{}",
                f.label()
            );
        }
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("nope.a");
        let source = dir.path().join("one.txt");
        std::fs::write(&source, b"x").unwrap();
        match crate::write::create(
            &out,
            Format::Ar,
            &[source],
            crate::Level::Normal,
            false,
            &CancelToken::new(),
            &mut |_, _| {},
        ) {
            Err(ArcError::Refused { reason, .. }) => {
                assert!(reason.contains("read here, not written"), "{reason}")
            }
            other => panic!("writing an ar should be refused: {other:?}"),
        }
        assert!(!out.exists(), "and nothing is left behind");
    }

    #[test]
    fn a_solid_7z_gives_up_each_file_on_its_own() {
        // Solid is 7z's own default and the case that could go wrong: several files share one
        // compressed stream, so reading the third means decoding the two in front of it. What
        // the caller asks for is still one file's bytes and nothing else.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("solid.7z");
        let file = File::create(&path).unwrap();
        let mut writer = sevenz_rust2::ArchiveWriter::new(file).unwrap();
        let bodies: Vec<&[u8]> = TREE.iter().map(|(_, body)| *body).collect();
        let entries = TREE
            .iter()
            .map(|(name, _)| {
                let mut entry = sevenz_rust2::ArchiveEntry::new();
                entry.name = (*name).to_string();
                // Writing a whole block at once leaves this to the caller, unlike pushing one
                // entry at a time; without it the archive says every file in the block is
                // empty, and a reader believes it.
                entry.has_stream = true;
                entry
            })
            .collect();
        writer
            .push_archive_entries(
                entries,
                bodies.iter().map(|b| (*b).into()).collect::<Vec<_>>(),
            )
            .unwrap();
        writer.finish().unwrap();

        let archive = open(&path);
        let cancel = CancelToken::new();
        for (name, body) in TREE {
            let inner = format!("/{name}");
            assert_eq!(
                archive.read(&inner, &cancel).expect(name),
                body.to_vec(),
                "{name}"
            );
        }
        let report = archive.verify(&cancel, &mut |_, _, _| {}).expect("a test");
        assert!(report.is_whole(), "{report:?}");
        assert_eq!(report.members, TREE.len() as u64);
    }

    #[test]
    fn an_encrypted_7z_says_it_wants_a_password_rather_than_looking_broken() {
        // Both halves of this matter: the error has to be the one the user can act on, and it
        // has to arrive at `open`, because a 7z can encrypt its own header and then even the
        // list of names is behind the password.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.7z");
        let file = File::create(&path).unwrap();
        let mut writer = sevenz_rust2::ArchiveWriter::new(file).unwrap();
        writer.set_content_methods(vec![sevenz_rust2::EncoderConfiguration::new(
            sevenz_rust2::EncoderMethod::AES256_SHA256,
        )
        .with_options(sevenz_rust2::encoder_options::EncoderOptions::Aes(
            sevenz_rust2::encoder_options::AesEncoderOptions::new("hunter2".into()),
        ))]);
        writer.set_encrypt_header(true);
        let mut entry = sevenz_rust2::ArchiveEntry::new();
        entry.name = "notes.txt".to_string();
        writer
            .push_archive_entry(entry, Some(&b"private"[..]))
            .unwrap();
        writer.finish().unwrap();

        match Archive::open(&path, &Limits::default(), &CancelToken::new()) {
            Err(ArcError::NeedsPassword { .. }) => {}
            other => panic!("expected a password to be asked for, got {other:?}"),
        }
    }

    #[test]
    fn a_7z_we_write_carries_the_mode_back_out_again() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("run.sh");
        std::fs::write(&source, b"#!/bin/sh\necho hi\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let path = dir.path().join("made.7z");
        crate::create(
            &path,
            Format::SevenZ,
            &[source],
            crate::Level::Fast,
            false,
            &CancelToken::new(),
            &mut |_, _| {},
        )
        .expect("the archive was written");

        let archive = open(&path);
        let entry = archive.entry("/run.sh").expect("the entry is there");
        assert_eq!(entry.kind, EntryKind::File);
        #[cfg(unix)]
        assert_eq!(
            entry.mode.map(|m| m & 0o777),
            Some(0o755),
            "a 7z carries a Unix mode in the top half of its attribute word"
        );
        assert!(
            entry.mtime.is_some(),
            "and the time it was written with came back too"
        );
    }

    /// A compressed stream's own check is what a test of it rests on, and each of the four
    /// has one: gzip's CRC-32, bzip2's block CRC, xz's check and zstd's checksum — which the
    /// decoder computes and leaves for somebody else to compare.
    #[test]
    fn a_damaged_compressed_stream_is_caught_by_its_own_checksum() {
        let dir = tempfile::tempdir().unwrap();
        // Long enough, and unlike itself enough, that the compressed form has a middle.
        let body: Vec<u8> = (0..40_000u32)
            .flat_map(|n| n.wrapping_mul(2_654_435_761).to_le_bytes())
            .collect();
        for format in [Format::Gz, Format::Bz2, Format::Xz, Format::Zst] {
            let path = dir.path().join(format!("dump.bin.{}", format.extension()));
            write_single(&path, format, &body);
            // Opened while it is whole and tested after the damage, as a file that went bad
            // on the disk is: opening reads the stream once to learn how long the entry is.
            let archive = open(&path);
            let report = archive
                .verify(&CancelToken::new(), &mut |_, _, _| {})
                .expect("a test");
            assert!(report.is_whole(), "{}: {:?}", format.label(), report.bad);
            assert_eq!((report.members, report.bytes), (1, body.len() as u64));

            // The checksum itself, where the format ends with one, so nothing but the
            // comparison can notice; a byte of the data for bzip2, which checks per block.
            let mut bytes = std::fs::read(&path).unwrap();
            let at = match format {
                // CRC-32 then the length, four bytes each, at the very end.
                Format::Gz => bytes.len() - 8,
                Format::Zst => bytes.len() - 1,
                _ => bytes.len() / 2,
            };
            bytes[at] ^= 0x01;
            std::fs::write(&path, bytes).unwrap();
            let report = archive
                .verify(&CancelToken::new(), &mut |_, _, _| {})
                .expect("the entry is still listed");
            assert_eq!(report.members, 1, "{}", format.label());
            assert_eq!(report.bad.len(), 1, "{}: went unnoticed", format.label());
            assert_eq!(report.bad[0].0, "/dump.bin", "{}", format.label());
            assert!(
                !report.bad[0].1.contains("dump.bin"),
                "the reason alone, the entry is named beside it: {:?}",
                report.bad[0]
            );
        }
    }

    /// A tarball whose second header is damaged: the member before it was read whole and
    /// says so, and the walk that could not go on is one line against the archive rather
    /// than a test that failed to say anything.
    #[test]
    fn a_tar_header_with_a_wrong_checksum_ends_the_test_with_a_reason() {
        let dir = tempfile::tempdir().unwrap();
        let mut tar = tar_bytes(&TREE);
        // The first member is one block of header and one of data; the second header's
        // name starts the third block.
        tar[1024] ^= 0x20;
        let path = dir.path().join("tree.tar.gz");
        let mut enc = flate2::write::GzEncoder::new(
            File::create(&path).unwrap(),
            flate2::Compression::fast(),
        );
        enc.write_all(&tar).unwrap();
        enc.finish().unwrap();
        // The index is built by the same walk, so the archive is opened from one that is
        // whole and tested after the damage, as a file that went bad on the disk is.
        let good = dir.path().join("good.tar.gz");
        write_archive(&good, Format::TarGz, &TREE);
        let mut archive = open(&good);
        archive.source = path;

        let report = archive
            .verify(&CancelToken::new(), &mut |_, _, _| {})
            .expect("a report, not a refusal");
        assert_eq!(report.members, 1, "{report:?}");
        assert_eq!(report.bytes, TREE[0].1.len() as u64);
        assert_eq!(report.bad.len(), 1, "{report:?}");
        assert_eq!(report.bad[0].0, "", "against the archive");
        assert!(report.bad[0].1.contains("checksum"), "{report:?}");
    }

    /// The bar moves inside an entry and not only between two of them: one entry can be
    /// most of an archive.
    #[test]
    fn a_test_reports_bytes_inside_an_entry() {
        let dir = tempfile::tempdir().unwrap();
        let body = vec![7u8; 400_000];
        let path = dir.path().join("big.zip");
        write_zip(&path, &[("big.bin", &body), ("small.txt", b"x")]);
        let mut calls: Vec<(String, u64, u64)> = Vec::new();
        let report = open(&path)
            .verify(&CancelToken::new(), &mut |name, members, bytes| {
                calls.push((name.to_string(), members, bytes))
            })
            .expect("a test");
        assert!(report.is_whole());
        let inside = calls
            .iter()
            .filter(|(name, _, bytes)| name == "/big.bin" && *bytes > 0 && *bytes < 400_000)
            .count();
        assert!(inside >= 2, "{calls:?}");
        assert!(
            calls.windows(2).all(|w| w[0].2 <= w[1].2),
            "never backwards"
        );
        assert_eq!(calls.last().map(|c| (c.1, c.2)), Some((2, 400_001)));
    }

    /// A runner that lists, and answers `t` with whatever a test wants a program to have
    /// said.
    #[derive(Debug)]
    struct Tester {
        code: i32,
        out: &'static str,
        err: &'static str,
        asked: Mutex<Vec<Call>>,
    }

    impl crate::ext::Run for Tester {
        fn run(
            &self,
            program: &Path,
            args: &[OsString],
            cwd: Option<&Path>,
            _cancel: &CancelToken,
        ) -> std::result::Result<crate::ext::Ran, crate::ext::ExtError> {
            if args[0] == "l" {
                return Ok(crate::ext::Ran {
                    code: Some(0),
                    out: WIM_LISTING.to_string(),
                    err: String::new(),
                });
            }
            self.asked.lock().unwrap().push(Call {
                program: program.display().to_string(),
                args: args
                    .iter()
                    .map(|a| a.to_string_lossy().into_owned())
                    .collect(),
                cwd: cwd.map(Path::to_path_buf),
            });
            // The names are where the table says they are: in the list file.
            let names = std::fs::read_to_string(cwd.expect("a folder").join(".fcmd-names"))
                .expect("the list file");
            assert_eq!(names, "readme.md\nsrc/main.rs\nsrc/deep/x.bin\n");
            Ok(crate::ext::Ran {
                code: Some(self.code),
                out: self.out.to_string(),
                err: self.err.to_string(),
            })
        }
    }

    fn tested_by(runner: &Arc<Tester>, dir: &Path) -> Archive {
        let path = dir.join("prova.wim");
        std::fs::write(&path, b"MSWIM\x00\x00\x00").unwrap();
        Archive::open_external(
            &path,
            Arc::new(table()),
            runner.clone(),
            &Limits::default(),
            &CancelToken::new(),
        )
        .expect("it lists")
    }

    /// An archive a program reads is tested by that program's own `Test`, in one run, and
    /// what it complains about comes back against the entries it names.
    #[test]
    fn a_program_s_archive_is_tested_by_the_program_s_own_test() {
        let dir = tempfile::tempdir().unwrap();
        let whole = Arc::new(Tester {
            code: 0,
            out: "Everything is Ok\n",
            err: "",
            asked: Mutex::new(Vec::new()),
        });
        let report = tested_by(&whole, dir.path())
            .verify(&CancelToken::new(), &mut |_, _, _| {})
            .expect("a test");
        assert!(report.is_whole(), "{report:?}");
        // What the listing said there is: nothing was read here to count.
        assert_eq!((report.members, report.bytes), (3, 14));
        let calls = whole.asked.lock().unwrap().clone();
        assert_eq!(calls.len(), 1, "one run for the whole archive");
        assert_eq!(calls[0].args[0], "t");
        assert!(
            !calls[0].args.iter().any(|a| a.contains("main.rs")),
            "no name on the line: {:?}",
            calls[0].args
        );
        let folder = calls[0].cwd.clone().expect("a folder of ours");
        assert!(!folder.exists(), "{folder:?} was left behind");

        // What 7-Zip 23.01 really wrote on its standard error for a damaged `.wim`.
        let damaged = Arc::new(Tester {
            code: 2,
            out: "Sub items Errors: 1\n\nArchives with Errors: 1\n",
            err: "ERROR: CRC Failed : src/main.rs\n",
            asked: Mutex::new(Vec::new()),
        });
        let report = tested_by(&damaged, dir.path())
            .verify(&CancelToken::new(), &mut |_, _, _| {})
            .expect("a report");
        assert_eq!(
            report.bad,
            [(
                "/src/main.rs".to_string(),
                "ERROR: CRC Failed : src/main.rs".to_string()
            )]
        );

        // A program that fails and names nothing has still failed: against the archive.
        let mute = Arc::new(Tester {
            code: 2,
            out: "",
            err: "",
            asked: Mutex::new(Vec::new()),
        });
        let report = tested_by(&mute, dir.path())
            .verify(&CancelToken::new(), &mut |_, _, _| {})
            .expect("a report");
        assert_eq!(report.bad.len(), 1);
        assert_eq!(report.bad[0].0, "");
        assert!(report.bad[0].1.contains("code 2"), "{report:?}");
    }

    /// The real thing, when the machine has it: a `.wim` 7z made, damaged in the middle of
    /// its data, and 7z's own test naming the file the damage is in.
    #[test]
    fn a_damaged_wim_is_found_by_7z_s_own_test_when_7z_is_here() {
        let Some(program) = crate::ext::which("7z") else {
            eprintln!("no 7z on this machine: skipped");
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        let big: Vec<u8> = (0..100_000u32)
            .flat_map(|n| n.wrapping_mul(2_654_435_761).to_le_bytes())
            .collect();
        std::fs::write(dir.path().join("src/big one.bin"), &big).unwrap();
        std::fs::write(dir.path().join("readme.md"), b"leggimi\n").unwrap();
        let made = std::process::Command::new(&program)
            .args(["a", "-bd", "-twim", "prova.wim", "src", "readme.md"])
            .current_dir(dir.path())
            .output()
            .expect("7z runs");
        assert!(made.status.success(), "{made:?}");
        let path = dir.path().join("prova.wim");

        let cache = Cache::new(Limits::default());
        let report = cache
            .open(&path, &CancelToken::new())
            .expect("it opens")
            .verify(&CancelToken::new(), &mut |_, _, _| {})
            .expect("a test");
        assert!(report.is_whole(), "{report:?}");
        assert_eq!(report.members, 2);

        // The large file is nearly all of the archive, so its middle is that file's data.
        let mut bytes = std::fs::read(&path).unwrap();
        let middle = bytes.len() / 2;
        for byte in &mut bytes[middle..middle + 64] {
            *byte ^= 0x55;
        }
        std::fs::write(&path, bytes).unwrap();
        cache.forget(&path);
        let report = cache
            .open(&path, &CancelToken::new())
            .expect("the listing still reads")
            .verify(&CancelToken::new(), &mut |_, _, _| {})
            .expect("a report");
        assert_eq!(report.bad.len(), 1, "{report:?}");
        assert_eq!(
            report.bad[0].0, "/src/big one.bin",
            "a name with a space in it"
        );
        assert!(report.bad[0].1.contains("CRC"), "{report:?}");
    }

    // ── cabinets ────────────────────────────────────────────────────────────────────────────

    use crate::cab::tests::{cabinet, noise, Member, Unit};

    /// Two folders, one packed and one stored, with a file long enough to cross blocks.
    fn two_folder_cabinet(big: &[u8]) -> Vec<u8> {
        cabinet(&[
            Unit {
                method: 1,
                members: vec![
                    Member {
                        name: "readme.md",
                        body: b"leggimi\n",
                    },
                    Member {
                        name: "src\\big.bin",
                        body: big,
                    },
                    Member {
                        name: "src\\last.txt",
                        body: b"the last of the folder",
                    },
                ],
                blocks: None,
            },
            Unit {
                method: 0,
                members: vec![Member {
                    name: "stored.txt",
                    body: b"as it is",
                }],
                blocks: None,
            },
        ])
    }

    /// A cabinet is tested a folder at a time, and damage in one block is the entries from
    /// that block on — in that folder and no other.
    #[test]
    fn a_cabinet_tests_whole_and_a_damaged_block_names_the_entries_it_takes_with_it() {
        let dir = tempfile::tempdir().unwrap();
        let big = noise(90_000, 11);
        let bytes = two_folder_cabinet(&big);
        let path = dir.path().join("setup.cab");
        std::fs::write(&path, &bytes).unwrap();
        let archive = open(&path);
        assert_eq!(archive.format(), Some(Format::Cab));
        assert_eq!(archive.beyond(), None);
        assert_eq!(
            archive.read("/src/last.txt", &CancelToken::new()).unwrap(),
            b"the last of the folder"
        );
        assert_eq!(
            archive.read("/src/big.bin", &CancelToken::new()).unwrap(),
            big
        );
        let report = archive
            .verify(&CancelToken::new(), &mut |_, _, _| {})
            .expect("a test");
        assert!(report.is_whole(), "{report:?}");
        assert_eq!(report.members, 4);
        assert_eq!(report.bytes, 8 + 90_000 + 22 + 8);

        // A byte of the second block of the first folder: the first block is the readme
        // and the start of the big file, which is therefore the first entry to fail.
        let folder_start = u32::from_le_bytes(bytes[36..40].try_into().unwrap()) as usize;
        let first_block = u16::from_le_bytes(
            bytes[folder_start + 4..folder_start + 6]
                .try_into()
                .unwrap(),
        );
        let second = folder_start + 8 + usize::from(first_block);
        let mut damaged = bytes.clone();
        damaged[second + 8 + 100] ^= 0x10;
        std::fs::write(&path, &damaged).unwrap();
        let report = open(&path)
            .verify(&CancelToken::new(), &mut |_, _, _| {})
            .expect("a report");
        let bad: Vec<&str> = report.bad.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(bad, ["/src/big.bin", "/src/last.txt"], "{report:?}");
        assert!(report.bad[0].1.contains("checksum"), "{report:?}");
        assert_eq!(report.members, 4, "the other folder was still read");
    }

    /// What `7z` makes of the cabinets these tests build, where there is a `7z`: the
    /// fixture is written by hand from the specification, and this is the second opinion
    /// on it — the block checksum, the tables, and the block that reaches back.
    #[test]
    fn the_cabinets_built_here_are_cabinets_to_7z_too() {
        let Some(program) = crate::ext::which("7z") else {
            eprintln!("no 7z on this machine: skipped");
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let big = noise(90_000, 11);
        let first = noise(32 * 1024, 99);
        let mut looking_back = first.clone();
        looking_back.extend_from_slice(&first[32 * 1024 - 1000..32 * 1024 - 990]);
        let cabinets = [
            ("tree.cab", two_folder_cabinet(&big), "src/big.bin", big),
            (
                "back.cab",
                cabinet(&[Unit {
                    method: 1,
                    members: vec![Member {
                        name: "two-blocks.bin",
                        body: &looking_back,
                    }],
                    blocks: Some(vec![
                        (crate::cab::tests::mszip_block(&first), 32 * 1024),
                        (crate::cab::tests::a_block_that_only_looks_back(), 10),
                    ]),
                }]),
                "two-blocks.bin",
                looking_back.clone(),
            ),
        ];
        for (name, bytes, member, body) in cabinets {
            let out = dir.path().join(format!("{name}.out"));
            std::fs::create_dir(&out).unwrap();
            let path = dir.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            let ran = std::process::Command::new(&program)
                .args(["x", "-y", "-bd"])
                .arg(&path)
                .current_dir(&out)
                .output()
                .expect("7z runs");
            assert!(ran.status.success(), "{name}: {ran:?}");
            assert_eq!(std::fs::read(out.join(member)).unwrap(), body, "{name}");
            // And this crate agrees with it.
            assert_eq!(
                open(&path)
                    .read(&format!("/{member}"), &CancelToken::new())
                    .unwrap(),
                body,
                "{name}"
            );
        }
    }

    /// A cabinet packed with LZX: the program that reads all of it gets it where there is
    /// one, and where there is none the listing made here stands and an entry says what it
    /// wants — the compressor, and the program to install.
    #[test]
    fn a_cabinet_this_crate_cannot_unpack_goes_to_a_program_or_says_which_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("driver.cab");
        let bytes = cabinet(&[Unit {
            method: 0x1503,
            members: vec![Member {
                name: "driver.sys",
                body: &[0u8; 1000],
            }],
            blocks: Some(vec![(vec![0x5b, 0x80, 0x80, 0x8d], 1000)]),
        }]);
        std::fs::write(&path, bytes).unwrap();

        // No table at all: read here, as far as it goes. (The tables are set outright in
        // this test, so that what is installed on the machine running it decides nothing.)
        let mut alone = Cache::new(Limits::default());
        alone.tools.clear();
        let archive = alone.open(&path, &CancelToken::new()).expect("it lists");
        assert_eq!(archive.format(), Some(Format::Cab));
        assert_eq!(archive.entry("/driver.sys").expect("listed").size, 1000);
        let error = archive
            .read("/driver.sys", &CancelToken::new())
            .unwrap_err();
        assert!(matches!(error, ArcError::Refused { .. }), "{error:?}");
        assert!(error.to_string().contains("LZX"), "{error}");
        let report = archive
            .verify(&CancelToken::new(), &mut |_, _, _| {})
            .expect("a report");
        assert_eq!(report.bad.len(), 1, "{report:?}");
        assert!(report.bad[0].1.contains("LZX"), "{report:?}");

        // A table whose program is not installed: the same listing, and the program named.
        let mut absent = table();
        absent.program = "fcmd-no-such-archiver".to_string();
        let mut without = Cache::new(Limits::default());
        without.tools = vec![Arc::new(absent)];
        let archive = without.open(&path, &CancelToken::new()).expect("it lists");
        assert_eq!(archive.format(), Some(Format::Cab));
        let error = archive
            .read("/driver.sys", &CancelToken::new())
            .unwrap_err()
            .to_string();
        assert!(error.contains("LZX"), "{error}");
        assert!(error.contains("fcmd-no-such-archiver"), "{error}");

        // A table whose program is here: the whole archive is its to read.
        let runner = Arc::new(Canned::listing(concat!(
            "   Date      Time    Attr         Size   Compressed  Name\n",
            "------------------- ----- ------------ ------------  ------------------------\n",
            "2026-11-04 09:57:12 ....A         1000               driver.sys\n",
            "------------------- ----- ------------ ------------  ------------------------\n",
        )));
        let with = Cache::new(Limits::default())
            .with_tools(vec![Arc::new(table())])
            .with_runner(runner.clone());
        let archive = with.open(&path, &CancelToken::new()).expect("it lists");
        assert_eq!(archive.format(), None, "the program read it");
        assert_eq!(archive.label(), "7z");
        assert_eq!(runner.calls().len(), 1);

        // And a cabinet this crate does read never reaches the program at all.
        let plain = dir.path().join("plain.cab");
        std::fs::write(&plain, two_folder_cabinet(b"small")).unwrap();
        let archive = with.open(&plain, &CancelToken::new()).expect("it opens");
        assert_eq!(archive.format(), Some(Format::Cab));
        assert_eq!(runner.calls().len(), 1, "nothing was run for it");
    }
}

//! What an executable says about itself beyond its layout: the version resource and the
//! manifest of a PE, where its debug information went, who signed it, and the same questions
//! asked of ELF notes and Mach-O load commands (SPEC 05 §1.8).
//!
//! Every reader here takes the bytes it needs and returns rows, and none of them follows an
//! offset it has not checked: these are the parts of a file most often written by a tool other
//! than the linker, and the parts a damaged or hostile file is most likely to get wrong. A
//! signature is **read, not verified** — the rows say who the certificates name, which is what
//! one looks at a file to find out, and never that the file is trustworthy.

use crate::structure::Row;

// ── little helpers ──────────────────────────────────────────────────────────────────────────

fn le16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn align4(n: usize) -> usize {
    (n + 3) & !3
}

/// A NUL-terminated string at `at`, as far as the bytes go.
fn cstr(b: &[u8], at: usize) -> String {
    let Some(tail) = b.get(at..) else {
        return String::new();
    };
    let end = tail.iter().position(|&c| c == 0).unwrap_or(tail.len());
    String::from_utf8_lossy(&tail[..end]).into_owned()
}

/// UTF-16LE up to a NUL, and where the NUL ended.
fn utf16z(b: &[u8], at: usize) -> (String, usize) {
    let mut units = Vec::new();
    let mut i = at;
    while let Some(u) = le16(b, i) {
        i += 2;
        if u == 0 {
            break;
        }
        units.push(u);
    }
    (String::from_utf16_lossy(&units), i)
}

/// `1.2.3.4` out of the two words a version resource keeps a version in.
fn quad(ms: u32, ls: u32) -> String {
    format!("{}.{}.{}.{}", ms >> 16, ms & 0xFFFF, ls >> 16, ls & 0xFFFF)
}

// ── PE: the resource tree ───────────────────────────────────────────────────────────────────

/// The resource directory's entries: `(id, offset)` with the high bit of the offset saying a
/// subdirectory. Named entries come first in the table and are skipped: the types this reads
/// (version, manifest) are always numbered.
fn res_entries(rsrc: &[u8], dir: usize) -> Vec<(u32, u32)> {
    let named = le16(rsrc, dir + 12).unwrap_or(0) as usize;
    let ids = le16(rsrc, dir + 14).unwrap_or(0) as usize;
    let mut out = Vec::new();
    for i in 0..(named + ids).min(4096) {
        let at = dir + 16 + i * 8;
        let (Some(name), Some(off)) = (le32(rsrc, at), le32(rsrc, at + 4)) else {
            break;
        };
        if name & 0x8000_0000 == 0 {
            out.push((name, off));
        }
    }
    out
}

/// The first leaf under a resource type, three levels down (type → name → language): the RVA
/// of its data and its size. `rsrc` is the resource section from its start, which is what
/// every offset inside the tree is relative to.
pub(crate) fn pe_resource(rsrc: &[u8], kind: u32) -> Option<(u32, u32)> {
    let (_, mut off) = res_entries(rsrc, 0)
        .into_iter()
        .find(|(id, _)| *id == kind)?;
    // Two more directories, each time taking the first entry, whatever it is called.
    for _ in 0..2 {
        if off & 0x8000_0000 == 0 {
            break;
        }
        let dir = (off & 0x7FFF_FFFF) as usize;
        let named = le16(rsrc, dir + 12)? as usize;
        let ids = le16(rsrc, dir + 14)? as usize;
        if named + ids == 0 {
            return None;
        }
        off = le32(rsrc, dir + 16 + 4)?;
    }
    if off & 0x8000_0000 != 0 {
        return None;
    }
    let leaf = off as usize;
    Some((le32(rsrc, leaf)?, le32(rsrc, leaf + 4)?))
}

/// Every resource of a type, by name: the name (`#12` for a numbered one), and the RVA and
/// size of its first language's data. Delphi keeps its forms and its package information as
/// named `RCDATA`, which is why this reads names where [`pe_resource`] skips them.
pub(crate) fn pe_resources(rsrc: &[u8], kind: u32) -> Vec<(String, u32, u32)> {
    let root_named = le16(rsrc, 12).unwrap_or(0) as usize;
    let root_ids = le16(rsrc, 14).unwrap_or(0) as usize;
    let Some(dir) = (0..(root_named + root_ids).min(4096)).find_map(|i| {
        let at = 16 + i * 8;
        let id = le32(rsrc, at)?;
        let off = le32(rsrc, at + 4)?;
        (id == kind && off & 0x8000_0000 != 0).then_some((off & 0x7FFF_FFFF) as usize)
    }) else {
        return Vec::new();
    };
    let named = le16(rsrc, dir + 12).unwrap_or(0) as usize;
    let ids = le16(rsrc, dir + 14).unwrap_or(0) as usize;
    let mut out = Vec::new();
    for i in 0..(named + ids).min(4096) {
        let at = dir + 16 + i * 8;
        let (Some(name), Some(mut off)) = (le32(rsrc, at), le32(rsrc, at + 4)) else {
            break;
        };
        let label = if name & 0x8000_0000 != 0 {
            let s = (name & 0x7FFF_FFFF) as usize;
            let n = le16(rsrc, s).unwrap_or(0) as usize;
            let units: Vec<u16> = (0..n.min(256))
                .filter_map(|k| le16(rsrc, s + 2 + k * 2))
                .collect();
            String::from_utf16_lossy(&units)
        } else {
            format!("#{name}")
        };
        // The language level: the first one.
        if off & 0x8000_0000 != 0 {
            let lang = (off & 0x7FFF_FFFF) as usize;
            let count = le16(rsrc, lang + 12).unwrap_or(0) as usize
                + le16(rsrc, lang + 14).unwrap_or(0) as usize;
            if count == 0 {
                continue;
            }
            let Some(leaf) = le32(rsrc, lang + 16 + 4) else {
                continue;
            };
            off = leaf;
        }
        if off & 0x8000_0000 != 0 {
            continue;
        }
        let leaf = off as usize;
        if let (Some(rva), Some(size)) = (le32(rsrc, leaf), le32(rsrc, leaf + 4)) {
            out.push((label, rva, size));
        }
    }
    out
}

/// What Delphi's `PACKAGEINFO` resource says: which compiler made the module, the packages it
/// needs and the units it is made of.
pub(crate) struct PackageInfo {
    pub producer: &'static str,
    pub requires: Vec<String>,
    pub units: Vec<String>,
}

/// `PACKAGEINFO`, as `GetPackageInfo` reads it: flags, the required packages (a hash byte and
/// a name each) and the contained units (a flags byte, a hash byte and a name each).
pub(crate) fn package_info(b: &[u8]) -> Option<PackageInfo> {
    let flags = le32(b, 0)?;
    let producer = match (flags >> 26) & 3 {
        3 => "Delphi",
        2 => "C++Builder",
        1 => "unknown",
        _ => "Delphi 3 or older",
    };
    let mut at = 4;
    let names = |at: &mut usize, skip: usize| -> Option<Vec<String>> {
        let count = le32(b, *at)? as usize;
        *at += 4;
        let mut v = Vec::new();
        for _ in 0..count.min(10_000) {
            *at += skip;
            let s = cstr(b, *at);
            *at += s.len() + 1;
            if *at > b.len() {
                return None;
            }
            v.push(s);
        }
        Some(v)
    };
    let requires = names(&mut at, 1)?;
    let units = names(&mut at, 2)?;
    Some(PackageInfo {
        producer,
        requires,
        units,
    })
}

/// One block of a version resource: its key, where its value is and how long, where its
/// children start and where it ends.
struct Block {
    key: String,
    value: std::ops::Range<usize>,
    text: bool,
    children: usize,
    end: usize,
}

fn block(b: &[u8], at: usize) -> Option<Block> {
    let length = le16(b, at)? as usize;
    let value_len = le16(b, at + 2)? as usize;
    let kind = le16(b, at + 4)?;
    if length < 6 || at + length > b.len() {
        return None;
    }
    let (key, after) = utf16z(b, at + 6);
    let value_at = align4(after);
    // A text value counts its length in characters, a binary one in bytes — and some
    // resource compilers count a text value in bytes anyway, so the length is capped at the
    // block rather than trusted.
    let bytes = if kind == 1 { value_len * 2 } else { value_len };
    let end = at + length;
    let value_end = (value_at + bytes).min(end);
    Some(Block {
        key,
        value: value_at.min(end)..value_end,
        text: kind == 1,
        children: align4(value_end).min(end),
        end,
    })
}

fn children(b: &[u8], parent: &Block) -> Vec<Block> {
    let mut out = Vec::new();
    let mut at = parent.children;
    while at + 6 <= parent.end && out.len() < 256 {
        let Some(child) = block(b, at) else {
            break;
        };
        at = align4(child.end);
        out.push(child);
    }
    out
}

fn file_os(os: u32) -> &'static str {
    match os {
        0x0004_0004 => "Windows NT",
        0x0000_0004 => "Windows (32-bit)",
        0x0001_0001 => "DOS, 16-bit Windows",
        0x0001_0004 => "DOS, 32-bit Windows",
        0x0004_0000 => "Windows NT (any)",
        _ => "",
    }
}

fn file_type(t: u32) -> &'static str {
    match t {
        1 => "application",
        2 => "DLL",
        3 => "driver",
        4 => "font",
        5 => "virtual device",
        7 => "static library",
        _ => "",
    }
}

/// The rows of a `VS_VERSIONINFO`: the fixed part (the versions as numbers, the flags) and
/// then every string the file carries — company, description, product, copyright — in the
/// order the file has them, which is the order its author wrote them in.
pub(crate) fn pe_version(b: &[u8]) -> Vec<Row> {
    let mut rows = Vec::new();
    let Some(root) = block(b, 0) else {
        return rows;
    };
    if root.key != "VS_VERSION_INFO" {
        return rows;
    }
    let v = &b[root.value.clone()];
    if v.len() >= 52 && le32(v, 0) == Some(0xFEEF_04BD) {
        let w = |i: usize| le32(v, i).unwrap_or(0);
        let file = quad(w(8), w(12));
        let product = quad(w(16), w(20));
        rows.push(Row::field(
            "FileVersion",
            format!("{:08X}{:08X}", w(8), w(12)),
            file,
        ));
        rows.push(Row::field(
            "ProductVersion",
            format!("{:08X}{:08X}", w(16), w(20)),
            product,
        ));
        let flags = w(28) & w(24);
        let mut said = Vec::new();
        for (bit, name) in [
            (1, "debug"),
            (2, "pre-release"),
            (4, "patched"),
            (8, "private build"),
            (0x10, "info inferred"),
            (0x20, "special build"),
        ] {
            if flags & bit != 0 {
                said.push(name);
            }
        }
        if !said.is_empty() {
            rows.push(Row::field(
                "FileFlags",
                format!("{flags:08X}"),
                said.join(", "),
            ));
        }
        let os = file_os(w(32));
        if !os.is_empty() {
            rows.push(Row::field("FileOS", format!("{:08X}", w(32)), os));
        }
        let kind = file_type(w(36));
        if !kind.is_empty() {
            rows.push(Row::field("FileType", format!("{:08X}", w(36)), kind));
        }
    }
    for child in children(b, &root) {
        match child.key.as_str() {
            "StringFileInfo" => {
                for table in children(b, &child) {
                    for s in children(b, &table) {
                        if !s.text {
                            continue;
                        }
                        let units: Vec<u16> = b[s.value.clone()]
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .map(|c| u16::from_le_bytes([c[0], c[1]]))
                            .take_while(|&u| u != 0)
                            .collect();
                        let value = String::from_utf16_lossy(&units).trim().to_string();
                        if !value.is_empty() {
                            rows.push(Row::field(s.key.clone(), String::new(), value));
                        }
                    }
                }
            }
            "VarFileInfo" => {
                for var in children(b, &child) {
                    if var.key != "Translation" {
                        continue;
                    }
                    let langs: Vec<String> = b[var.value.clone()]
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|c| {
                            let lang = u16::from_le_bytes([c[0], c[1]]);
                            let cp = u16::from_le_bytes([c[2], c[3]]);
                            format!("{lang:04X} code page {cp}")
                        })
                        .collect();
                    if !langs.is_empty() {
                        rows.push(Row::field("Translation", String::new(), langs.join(", ")));
                    }
                }
            }
            _ => {}
        }
    }
    rows
}

/// What an application manifest asks for: the execution level (`asInvoker`,
/// `requireAdministrator`) — the one line in it that decides whether starting the program
/// raises a UAC prompt — and whether it says it is DPI aware.
pub(crate) fn pe_manifest(b: &[u8]) -> Vec<Row> {
    let text = String::from_utf8_lossy(b);
    let mut rows = Vec::new();
    if let Some(at) = text.find("requestedExecutionLevel") {
        let tail = &text[at..];
        if let Some(l) = tail.find("level=") {
            let rest = &tail[l + 6..];
            let quote = rest.chars().next().unwrap_or('"');
            let value: String = rest.chars().skip(1).take_while(|&c| c != quote).collect();
            if !value.is_empty() {
                rows.push(Row::field("execution level", String::new(), value));
            }
        }
        if tail.contains("uiAccess=\"true\"") || tail.contains("uiAccess='true'") {
            rows.push(Row::field("uiAccess", String::new(), "true"));
        }
    }
    for tag in ["dpiAwareness", "dpiAware"] {
        let open = format!("<{tag}");
        if let Some(at) = text.find(&open) {
            let tail = &text[at..];
            if let Some(gt) = tail.find('>') {
                let value: String = tail[gt + 1..].chars().take_while(|&c| c != '<').collect();
                let value = value.trim();
                if !value.is_empty() {
                    rows.push(Row::field(tag, String::new(), value.to_string()));
                    break;
                }
            }
        }
    }
    rows
}

/// What `DllCharacteristics` says about how the image may be loaded: the mitigations a
/// security review asks about first.
pub(crate) fn dll_characteristics(f: u16) -> String {
    let mut out = Vec::new();
    for (bit, name) in [
        (0x0020, "high-entropy ASLR"),
        (0x0040, "ASLR"),
        (0x0080, "integrity checked"),
        (0x0100, "DEP"),
        (0x0200, "no isolation"),
        (0x0400, "no SEH"),
        (0x0800, "no bind"),
        (0x1000, "AppContainer"),
        (0x2000, "WDM driver"),
        (0x4000, "Control Flow Guard"),
        (0x8000, "terminal server aware"),
    ] {
        if f & bit != 0 {
            out.push(name);
        }
    }
    if out.is_empty() {
        "none".into()
    } else {
        out.join(", ")
    }
}

/// One entry of the debug directory, as `(type, raw data offset in the file, size)`.
pub(crate) fn debug_entries(dir: &[u8]) -> Vec<(u32, u32, u32)> {
    dir.as_chunks::<28>()
        .0
        .iter()
        .take(64)
        .filter_map(|e| Some((le32(e, 12)?, le32(e, 24)?, le32(e, 16)?)))
        .collect()
}

/// A CodeView record: the PDB a debugger looks for, and the GUID and age it must match —
/// which is what a symbol server is asked for.
pub(crate) fn codeview(b: &[u8]) -> Vec<Row> {
    let mut rows = Vec::new();
    if b.get(..4) == Some(b"RSDS") && b.len() >= 24 {
        let g = &b[4..20];
        let guid = format!(
            "{:08X}-{:04X}-{:04X}-{}-{}",
            le32(g, 0).unwrap_or(0),
            le16(g, 4).unwrap_or(0),
            le16(g, 6).unwrap_or(0),
            hex(&g[8..10]),
            hex(&g[10..16])
        );
        let age = le32(b, 20).unwrap_or(0);
        rows.push(Row::field("PDB", String::new(), cstr(b, 24)));
        rows.push(Row::field(
            "PDB GUID",
            String::new(),
            format!("{guid} age {age}"),
        ));
    } else if b.get(..4) == Some(b"NB10") && b.len() >= 16 {
        rows.push(Row::field("PDB", String::new(), cstr(b, 16)));
    }
    rows
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect()
}

/// The attribute certificate table of a PE: each `WIN_CERTIFICATE` and what it holds. The
/// table is the one directory whose address is a file offset rather than an RVA, because it
/// is not loaded with the image.
pub(crate) fn pe_certificates(b: &[u8]) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut at = 0usize;
    while at + 8 <= b.len() && rows.len() < 64 {
        let length = le32(b, at).unwrap_or(0) as usize;
        let kind = le16(b, at + 6).unwrap_or(0);
        if length < 8 || at + length > b.len() {
            break;
        }
        let body = &b[at + 8..at + length];
        let what = match kind {
            1 => "X.509 certificate",
            2 => "Authenticode (PKCS #7)",
            3 => "reserved",
            4 => "terminal server protocol stack",
            _ => "unknown kind",
        };
        rows.push(Row::field(
            "certificate",
            format!("{kind:04X}"),
            format!("{what}, {} bytes, not verified", body.len()),
        ));
        if kind == 2 {
            rows.extend(signer_rows(body));
        }
        at = align4(at + length);
    }
    rows
}

/// The names a PKCS #7 blob's certificates carry, and when it says it was signed.
pub(crate) fn signer_rows(der: &[u8]) -> Vec<Row> {
    let mut rows = Vec::new();
    let names = der_names(der);
    if let Some(first) = names.first() {
        rows.push(Row::field("signed by", String::new(), first.clone()));
    }
    if names.len() > 1 {
        rows.push(Row::field(
            "certificates",
            String::new(),
            names[1..].join(" · "),
        ));
    }
    if let Some(when) = signing_time(der) {
        rows.push(Row::field("signing time", String::new(), when));
    }
    rows
}

// ── DER, as little of it as names need ──────────────────────────────────────────────────────

const OID_CN: &[u8] = &[0x06, 0x03, 0x55, 0x04, 0x03];
const OID_O: &[u8] = &[0x06, 0x03, 0x55, 0x04, 0x0A];
const OID_SIGNING_TIME: &[u8] = &[
    0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x05,
];

/// A DER string right after `at`: its tag, and the text in it.
fn der_string(b: &[u8], at: usize) -> Option<String> {
    let tag = *b.get(at)?;
    let len = *b.get(at + 1)? as usize;
    if len >= 0x80 {
        return None;
    }
    let body = b.get(at + 2..at + 2 + len)?;
    match tag {
        // UTF8String, PrintableString, T61String, IA5String
        0x0C | 0x13 | 0x14 | 0x16 => Some(String::from_utf8_lossy(body).into_owned()),
        // BMPString: UTF-16BE
        0x1E => {
            let units: Vec<u16> = body
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            Some(String::from_utf16_lossy(&units))
        }
        _ => None,
    }
}

/// The distinct common names (organisation where a certificate has no common name) the
/// certificates in a DER blob carry, in the order they first appear. In an Authenticode
/// signature that order starts with the signer's own certificate in nearly every file, and
/// the issuers follow — which is all the pane claims about it.
///
/// This is a scan for the attribute's object identifier rather than a parse of the ASN.1:
/// the names are the one thing wanted from the blob, and a scan cannot be sent anywhere by a
/// length field that lies.
pub(crate) fn der_names(der: &[u8]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for oid in [OID_CN, OID_O] {
        let mut i = 0usize;
        while let Some(pos) = find(&der[i..], oid) {
            let at = i + pos + oid.len();
            if let Some(name) = der_string(der, at) {
                let name = name.trim().to_string();
                if !name.is_empty() && !out.contains(&name) {
                    out.push(name);
                }
            }
            i = at;
            if out.len() >= 16 {
                break;
            }
        }
        if !out.is_empty() {
            break;
        }
    }
    out
}

/// The signing time attribute, where the signer put one: `YYYY-MM-DD hh:mm:ss UTC`.
fn signing_time(der: &[u8]) -> Option<String> {
    let at = find(der, OID_SIGNING_TIME)? + OID_SIGNING_TIME.len();
    // SET { UTCTime | GeneralizedTime }
    let set = der.get(at..at + 2)?;
    if set[0] != 0x31 {
        return None;
    }
    let tag = *der.get(at + 2)?;
    let len = *der.get(at + 3)? as usize;
    let body = std::str::from_utf8(der.get(at + 4..at + 4 + len)?).ok()?;
    let (year, rest) = match tag {
        0x17 if body.len() >= 12 => {
            let yy: u32 = body[..2].parse().ok()?;
            (if yy < 50 { 2000 + yy } else { 1900 + yy }, &body[2..])
        }
        0x18 if body.len() >= 14 => (body[..4].parse().ok()?, &body[4..]),
        _ => return None,
    };
    Some(format!(
        "{year:04}-{}-{} {}:{}:{} UTC",
        &rest[0..2],
        &rest[2..4],
        &rest[4..6],
        &rest[6..8],
        &rest[8..10]
    ))
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

// ── ELF notes ───────────────────────────────────────────────────────────────────────────────

/// The notes in a note section, as `(name, type, descriptor)`.
pub(crate) fn elf_notes(b: &[u8]) -> Vec<(String, u32, &[u8])> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at + 12 <= b.len() && out.len() < 64 {
        let namesz = le32(b, at).unwrap_or(0) as usize;
        let descsz = le32(b, at + 4).unwrap_or(0) as usize;
        let kind = le32(b, at + 8).unwrap_or(0);
        let name_at = at + 12;
        let desc_at = name_at + align4(namesz);
        let end = desc_at + align4(descsz);
        if desc_at + descsz > b.len() {
            break;
        }
        let name = cstr(&b[name_at..name_at + namesz], 0);
        out.push((name, kind, &b[desc_at..desc_at + descsz]));
        at = end;
    }
    out
}

/// What the notes of an ELF file say, row by row: the ABI it was built for, the Go build id,
/// the package it came from.
pub(crate) fn elf_note_rows(b: &[u8]) -> Vec<Row> {
    let mut rows = Vec::new();
    for (name, kind, desc) in elf_notes(b) {
        match (name.as_str(), kind) {
            ("GNU", 1) if desc.len() >= 16 => {
                let os = match le32(desc, 0).unwrap_or(9) {
                    0 => "Linux",
                    1 => "GNU/Hurd",
                    2 => "Solaris",
                    3 => "FreeBSD",
                    _ => "unknown OS",
                };
                rows.push(Row::field(
                    "ABI",
                    String::new(),
                    format!(
                        "{os} {}.{}.{} or later",
                        le32(desc, 4).unwrap_or(0),
                        le32(desc, 8).unwrap_or(0),
                        le32(desc, 12).unwrap_or(0)
                    ),
                ));
            }
            ("Go", 4) => {
                rows.push(Row::field("Go build id", String::new(), cstr(desc, 0)));
            }
            // systemd's package metadata: a JSON object naming the package and its version.
            ("FDO", 0xCAFE_1A7E) => {
                rows.push(Row::field("package", String::new(), cstr(desc, 0)));
            }
            ("FreeBSD", 1) if desc.len() >= 4 => {
                rows.push(Row::field(
                    "ABI",
                    String::new(),
                    format!("FreeBSD {}", le32(desc, 0).unwrap_or(0)),
                ));
            }
            _ => {}
        }
    }
    rows
}

/// The `.comment` section: the compilers and linkers that put their name in the file, once
/// each.
pub(crate) fn elf_comment(b: &[u8]) -> Vec<Row> {
    let mut seen: Vec<String> = Vec::new();
    for part in b.split(|&c| c == 0) {
        let s = String::from_utf8_lossy(part).trim().to_string();
        if !s.is_empty() && !seen.contains(&s) && seen.len() < 8 {
            seen.push(s);
        }
    }
    seen.into_iter()
        .map(|s| Row::field("built by", String::new(), s))
        .collect()
}

/// `.gnu_debuglink`: the file the debug information was split into, and its CRC.
pub(crate) fn elf_debuglink(b: &[u8]) -> Vec<Row> {
    let name = cstr(b, 0);
    if name.is_empty() {
        return Vec::new();
    }
    let crc_at = align4(name.len() + 1);
    let crc = le32(b, crc_at)
        .map(|c| format!(", CRC {c:08x}"))
        .unwrap_or_default();
    vec![Row::field(
        "debug link",
        String::new(),
        format!("{name}{crc}"),
    )]
}

// ── Mach-O ──────────────────────────────────────────────────────────────────────────────────

/// A packed `xxxx.yy.zz` version, as Mach-O keeps the OS and SDK versions.
pub(crate) fn macho_version(v: u32) -> String {
    let (major, minor, patch) = (v >> 16, (v >> 8) & 0xFF, v & 0xFF);
    if patch == 0 {
        format!("{major}.{minor}")
    } else {
        format!("{major}.{minor}.{patch}")
    }
}

pub(crate) fn macho_platform(p: u32) -> &'static str {
    match p {
        1 => "macOS",
        2 => "iOS",
        3 => "tvOS",
        4 => "watchOS",
        5 => "bridgeOS",
        6 => "Mac Catalyst",
        7 => "iOS simulator",
        8 => "tvOS simulator",
        9 => "watchOS simulator",
        10 => "DriverKit",
        11 => "visionOS",
        12 => "visionOS simulator",
        _ => "unknown platform",
    }
}

/// `LC_SOURCE_VERSION`: five numbers packed 24/10/10/10/10 into 64 bits.
pub(crate) fn source_version(v: u64) -> String {
    let a = v >> 40;
    let parts = [
        (v >> 30) & 0x3FF,
        (v >> 20) & 0x3FF,
        (v >> 10) & 0x3FF,
        v & 0x3FF,
    ];
    let mut s = a.to_string();
    for p in parts {
        s.push_str(&format!(".{p}"));
    }
    s.trim_end_matches(".0").to_string()
}

/// A dylib's `current_version`, packed like the OS versions but with a 16-bit major.
pub(crate) fn dylib_version(v: u32) -> String {
    format!("{}.{}.{}", v >> 16, (v >> 8) & 0xFF, v & 0xFF)
}

/// The code signature an `LC_CODE_SIGNATURE` points at: a big-endian SuperBlob whose
/// CodeDirectory names the identifier and the team, says ad-hoc or not and whether the
/// hardened runtime is on, and whose CMS blob — when there is one — names the certificates.
pub(crate) fn code_signature(b: &[u8]) -> Vec<Row> {
    let mut rows = Vec::new();
    if be32(b, 0) != Some(0xFADE_0CC0) {
        return rows;
    }
    let count = be32(b, 8).unwrap_or(0).min(64) as usize;
    let mut cms = false;
    let mut directory = false;
    for i in 0..count {
        let (Some(kind), Some(off)) = (be32(b, 12 + i * 8), be32(b, 16 + i * 8)) else {
            break;
        };
        let at = off as usize;
        let Some(magic) = be32(b, at) else {
            continue;
        };
        let blob_len = be32(b, at + 4).unwrap_or(0) as usize;
        let Some(blob) = b.get(at..at + blob_len.max(8)) else {
            continue;
        };
        match magic {
            // The CodeDirectory. Several may be present (SHA-1 and SHA-256); the first is
            // enough to say who and how.
            0xFADE_0C02 if !directory => {
                directory = true;
                let version = be32(blob, 8).unwrap_or(0);
                let flags = be32(blob, 12).unwrap_or(0);
                let ident = be32(blob, 20).unwrap_or(0) as usize;
                rows.push(Row::field("identifier", String::new(), cstr(blob, ident)));
                // The team offset is there from version 2.2 of the directory on.
                if version >= 0x2_0200 {
                    if let Some(team) = be32(blob, 48).filter(|&t| t != 0) {
                        let team = cstr(blob, team as usize);
                        if !team.is_empty() {
                            rows.push(Row::field("team", String::new(), team));
                        }
                    }
                }
                let mut said = Vec::new();
                if flags & 0x2 != 0 {
                    said.push("ad-hoc");
                }
                if flags & 0x1_0000 != 0 {
                    said.push("hardened runtime");
                }
                if flags & 0x0200 != 0 {
                    said.push("library validation");
                }
                if flags & 0x0800 != 0 {
                    said.push("restrict");
                }
                if !said.is_empty() {
                    rows.push(Row::field("flags", format!("{flags:08X}"), said.join(", ")));
                }
                let hash = match blob.get(37) {
                    Some(1) => "SHA-1",
                    Some(2) => "SHA-256",
                    Some(3) => "SHA-256 (truncated)",
                    Some(4) => "SHA-384",
                    _ => "",
                };
                if !hash.is_empty() {
                    rows.push(Row::field("hash", String::new(), hash));
                }
            }
            // Entitlements, as the XML plist they are written in.
            0xFADE_7171 if kind == 5 => {
                rows.push(Row::field(
                    "entitlements",
                    String::new(),
                    format!("{} bytes", blob.len().saturating_sub(8)),
                ));
            }
            // The CMS signature: empty for an ad-hoc signature, the certificates otherwise.
            0xFADE_0B01 => {
                let der = blob.get(8..).unwrap_or(&[]);
                if !der.is_empty() {
                    cms = true;
                    rows.extend(signer_rows(der));
                }
            }
            _ => {}
        }
    }
    if directory && !cms {
        rows.push(Row::field(
            "certificates",
            String::new(),
            "none — signed ad hoc",
        ));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(s: &str) -> Vec<u8> {
        let mut out: Vec<u8> = s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        out.extend([0, 0]);
        out
    }

    /// A version block, built the way a resource compiler lays one out.
    fn ver(key: &str, value: &[u8], text: bool, kids: &[Vec<u8>]) -> Vec<u8> {
        let mut b = vec![0u8; 6];
        b.extend(utf16(key));
        while !b.len().is_multiple_of(4) {
            b.push(0);
        }
        b.extend(value);
        while !b.len().is_multiple_of(4) {
            b.push(0);
        }
        for k in kids {
            b.extend(k);
            while !b.len().is_multiple_of(4) {
                b.push(0);
            }
        }
        let len = b.len() as u16;
        b[0..2].copy_from_slice(&len.to_le_bytes());
        let vlen = if text { value.len() / 2 } else { value.len() } as u16;
        b[2..4].copy_from_slice(&vlen.to_le_bytes());
        b[4..6].copy_from_slice(&(text as u16).to_le_bytes());
        b
    }

    #[test]
    fn a_version_resource_reads_as_its_numbers_and_its_strings() {
        let mut fixed = Vec::new();
        for w in [
            0xFEEF_04BDu32,
            0x0001_0000,
            0x0002_0003, // file 2.3
            0x0004_0005, // .4.5
            0x0002_0003,
            0x0000_0000,
            0x3F,
            0x0000_0002, // pre-release
            0x0004_0004,
            1,
            0,
            0,
            0,
        ] {
            fixed.extend(w.to_le_bytes());
        }
        let company = ver("CompanyName", &utf16("Acme"), true, &[]);
        let desc = ver("FileDescription", &utf16("Rocket skates"), true, &[]);
        let table = ver("040904B0", &[], true, &[company, desc]);
        let sfi = ver("StringFileInfo", &[], true, &[table]);
        let trans = ver("Translation", &[0x09, 0x04, 0xB0, 0x04], false, &[]);
        let vfi = ver("VarFileInfo", &[], true, &[trans]);
        let root = ver("VS_VERSION_INFO", &fixed, false, &[sfi, vfi]);

        let rows = pe_version(&root);
        let get = |l: &str| {
            rows.iter()
                .find(|r| r.label == l)
                .map(|r| r.value.clone())
                .unwrap_or_default()
        };
        assert_eq!(get("FileVersion"), "2.3.4.5");
        assert_eq!(get("ProductVersion"), "2.3.0.0");
        assert_eq!(get("FileFlags"), "pre-release");
        assert_eq!(get("FileOS"), "Windows NT");
        assert_eq!(get("FileType"), "application");
        assert_eq!(get("CompanyName"), "Acme");
        assert_eq!(get("FileDescription"), "Rocket skates");
        assert_eq!(get("Translation"), "0409 code page 1200");
        // Garbage is nothing, not a panic.
        assert!(pe_version(&root[..10]).is_empty());
        assert!(pe_version(&[0xFF; 64]).is_empty());
    }

    /// Delphi's `RCDATA` is named: a form is found by its name, and `PACKAGEINFO` says the
    /// producer, the packages and the units.
    #[test]
    fn delphi_resources_are_read_by_name_and_the_package_info_by_its_layout() {
        let mut r = vec![0u8; 200];
        let w16 =
            |r: &mut Vec<u8>, at: usize, v: u16| r[at..at + 2].copy_from_slice(&v.to_le_bytes());
        let w32 =
            |r: &mut Vec<u8>, at: usize, v: u32| r[at..at + 4].copy_from_slice(&v.to_le_bytes());
        // Root: one numbered entry, RCDATA (10), to the directory at 24.
        w16(&mut r, 14, 1);
        w32(&mut r, 16, 10);
        w32(&mut r, 20, 0x8000_0000 | 24);
        // Names: one named entry, its string at 100, its languages at 48.
        w16(&mut r, 24 + 12, 1);
        w32(&mut r, 24 + 16, 0x8000_0000 | 100);
        w32(&mut r, 24 + 20, 0x8000_0000 | 48);
        // Languages: one, to the leaf at 72.
        w16(&mut r, 48 + 14, 1);
        w32(&mut r, 48 + 16, 0x409);
        w32(&mut r, 48 + 20, 72);
        w32(&mut r, 72, 0x5000);
        w32(&mut r, 76, 1234);
        w16(&mut r, 100, 6);
        for (i, u) in "TFORM1".encode_utf16().enumerate() {
            w16(&mut r, 102 + i * 2, u);
        }
        assert_eq!(
            pe_resources(&r, 10),
            vec![("TFORM1".to_string(), 0x5000, 1234)]
        );
        assert!(pe_resources(&r, 16).is_empty());

        let mut p = Vec::new();
        p.extend_from_slice(&(3u32 << 26).to_le_bytes());
        p.extend_from_slice(&1u32.to_le_bytes());
        p.push(0x11);
        p.extend_from_slice(b"rtl\0");
        p.extend_from_slice(&2u32.to_le_bytes());
        p.extend_from_slice(&[0, 0x22]);
        p.extend_from_slice(b"Unit1\0");
        p.extend_from_slice(&[0, 0x33]);
        p.extend_from_slice(b"System.SysUtils\0");
        let info = package_info(&p).unwrap();
        assert_eq!(info.producer, "Delphi");
        assert_eq!(info.requires, ["rtl"]);
        assert_eq!(info.units, ["Unit1", "System.SysUtils"]);
        assert!(package_info(&p[..p.len() - 4]).is_none(), "cut short");
    }

    #[test]
    fn the_resource_tree_is_walked_down_to_its_first_leaf() {
        // root: one id entry (16) → dir at 0x18 → one entry → dir at 0x30 → leaf at 0x48.
        let mut r = vec![0u8; 0x58];
        r[14] = 1;
        r[16..20].copy_from_slice(&16u32.to_le_bytes());
        r[20..24].copy_from_slice(&(0x8000_0018u32).to_le_bytes());
        r[0x18 + 14] = 1;
        r[0x18 + 16..0x18 + 20].copy_from_slice(&1u32.to_le_bytes());
        r[0x18 + 20..0x18 + 24].copy_from_slice(&(0x8000_0030u32).to_le_bytes());
        r[0x30 + 14] = 1;
        r[0x30 + 16..0x30 + 20].copy_from_slice(&0x409u32.to_le_bytes());
        r[0x30 + 20..0x30 + 24].copy_from_slice(&0x48u32.to_le_bytes());
        r[0x48..0x4C].copy_from_slice(&0x5000u32.to_le_bytes());
        r[0x4C..0x50].copy_from_slice(&0x123u32.to_le_bytes());
        assert_eq!(pe_resource(&r, 16), Some((0x5000, 0x123)));
        assert_eq!(pe_resource(&r, 24), None);
    }

    #[test]
    fn a_manifest_says_what_it_asks_for() {
        let m = br#"<assembly><trustInfo><security><requestedPrivileges>
            <requestedExecutionLevel level="requireAdministrator" uiAccess="false"/>
            </requestedPrivileges></security></trustInfo>
            <application><windowsSettings><dpiAware>true/pm</dpiAware></windowsSettings></application></assembly>"#;
        let rows = pe_manifest(m);
        assert_eq!(rows[0].value, "requireAdministrator");
        assert_eq!(rows[1].label, "dpiAware");
        assert_eq!(rows[1].value, "true/pm");
    }

    #[test]
    fn codeview_names_the_pdb_and_what_it_must_match() {
        let mut b = b"RSDS".to_vec();
        b.extend([
            0x78, 0x56, 0x34, 0x12, 0x34, 0x12, 0x78, 0x56, 1, 2, 3, 4, 5, 6, 7, 8,
        ]);
        b.extend(3u32.to_le_bytes());
        b.extend(b"C:\\build\\app.pdb\0");
        let rows = codeview(&b);
        assert_eq!(rows[0].value, "C:\\build\\app.pdb");
        assert_eq!(rows[1].value, "12345678-1234-5678-0102-030405060708 age 3");
    }

    #[test]
    fn a_signature_names_its_certificates_without_claiming_to_check_them() {
        // Two certificates' worth of names and a signing time, the bytes a PKCS #7 holds
        // them in.
        let mut der = vec![0x30, 0x00];
        der.extend(OID_CN);
        der.extend([0x0C, 4]);
        der.extend(b"Acme");
        der.extend(OID_CN);
        der.extend([0x13, 7]);
        der.extend(b"Acme CA");
        der.extend(OID_CN);
        der.extend([0x0C, 4]);
        der.extend(b"Acme");
        der.extend(OID_SIGNING_TIME);
        der.extend([0x31, 0x0F, 0x17, 0x0D]);
        der.extend(b"261106120000Z");
        let mut cert = Vec::new();
        let len = (8 + der.len()) as u32;
        cert.extend(len.to_le_bytes());
        cert.extend(0x0200u16.to_le_bytes());
        cert.extend(2u16.to_le_bytes());
        cert.extend(&der);
        let rows = pe_certificates(&cert);
        let get = |l: &str| {
            rows.iter()
                .find(|r| r.label == l)
                .map(|r| r.value.clone())
                .unwrap_or_default()
        };
        assert!(get("certificate").contains("not verified"));
        assert_eq!(get("signed by"), "Acme");
        assert_eq!(get("certificates"), "Acme CA");
        assert_eq!(get("signing time"), "2026-11-06 12:00:00 UTC");
        // A length that lies stops the walk.
        let mut bad = cert.clone();
        bad[0..4].copy_from_slice(&9999u32.to_le_bytes());
        assert!(pe_certificates(&bad).is_empty());
    }

    #[test]
    fn elf_notes_say_the_abi_and_the_builder() {
        let mut b = Vec::new();
        b.extend(4u32.to_le_bytes());
        b.extend(16u32.to_le_bytes());
        b.extend(1u32.to_le_bytes());
        b.extend(b"GNU\0");
        for w in [0u32, 3, 2, 0] {
            b.extend(w.to_le_bytes());
        }
        let rows = elf_note_rows(&b);
        assert_eq!(rows[0].value, "Linux 3.2.0 or later");
        let rows =
            elf_comment(b"GCC: (Debian 12.2.0) 12.2.0\0GCC: (Debian 12.2.0) 12.2.0\0rustc 1.83\0");
        assert_eq!(rows.len(), 2);
        let rows = elf_debuglink(b"app.debug\0\0\0\x78\x56\x34\x12");
        assert_eq!(rows[0].value, "app.debug, CRC 12345678");
    }

    #[test]
    fn mach_o_versions_and_a_code_signature() {
        assert_eq!(macho_version(0x000E_0000), "14.0");
        assert_eq!(macho_version(0x000D_0301), "13.3.1");
        assert_eq!(dylib_version(0x0501_0000), "1281.0.0");
        assert_eq!(source_version((1u64 << 40) | (2 << 30)), "1.2");

        // A SuperBlob with one ad-hoc CodeDirectory.
        let mut cd = vec![0u8; 0x60];
        cd[0..4].copy_from_slice(&0xFADE_0C02u32.to_be_bytes());
        cd[4..8].copy_from_slice(&(0x60u32).to_be_bytes());
        cd[8..12].copy_from_slice(&0x2_0400u32.to_be_bytes());
        cd[12..16].copy_from_slice(&0x1_0002u32.to_be_bytes());
        cd[20..24].copy_from_slice(&0x50u32.to_be_bytes());
        cd[37] = 2;
        cd[0x50..0x5A].copy_from_slice(b"com.acme\0\0");
        let mut sb = Vec::new();
        sb.extend(0xFADE_0CC0u32.to_be_bytes());
        sb.extend((20 + cd.len() as u32).to_be_bytes());
        sb.extend(1u32.to_be_bytes());
        sb.extend(0u32.to_be_bytes());
        sb.extend(20u32.to_be_bytes());
        sb.extend(&cd);
        let rows = code_signature(&sb);
        let get = |l: &str| {
            rows.iter()
                .find(|r| r.label == l)
                .map(|r| r.value.clone())
                .unwrap_or_default()
        };
        assert_eq!(get("identifier"), "com.acme");
        assert_eq!(get("flags"), "ad-hoc, hardened runtime");
        assert_eq!(get("hash"), "SHA-256");
        assert_eq!(get("certificates"), "none — signed ad hoc");
        assert!(code_signature(&[0u8; 4]).is_empty());
    }
}

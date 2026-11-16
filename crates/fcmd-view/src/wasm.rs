//! WebAssembly: a module as its text format on `F5`, and what it is made of in the structure
//! pane (SPEC 05 §1.6).
//!
//! Both come from the Bytecode Alliance's crates — `wasmprinter` writes the WAT text that
//! `wasm-tools print` writes, `wasmparser` reads the sections — because the format keeps
//! growing (SIMD, threads, GC, exceptions, components) and these are kept by the people who
//! write the specification. A module the parser refuses says why, in its words.

use crate::structure::{Row, Structure};

/// The module as WAT text: what `F5` shows, as a view derived from the file.
pub fn listing(bytes: &[u8]) -> Result<String, String> {
    wasmprinter::print_bytes(bytes).map_err(|e| format!("{e:#}"))
}

/// How many functions, imports or exports are listed one a row before the rest are counted.
const ROWS: usize = 500;

/// The structure of a module: its version, its sections with their sizes and where they are,
/// its imports and exports, and its functions with their names and the size of their code.
pub(crate) fn read(bytes: &[u8], len: u64) -> Option<Structure> {
    if !bytes.starts_with(b"\0asm") {
        return None;
    }
    use wasmparser::{KnownCustom, Name, Parser, Payload};

    let mut rows = vec![Row::group("WebAssembly")];
    let mut sections = vec![Row::group("Sections")];
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    let mut bodies: Vec<(u64, u64)> = Vec::new();
    let mut names: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
    let mut imported_funcs = 0u32;
    let mut defined = 0u32;
    let mut types = 0u32;
    let mut component = false;
    let mut module_name = None;
    let mut problem = None;
    let mut depth = 0usize;

    let section = |sections: &mut Vec<Row>, name: &str, range: std::ops::Range<u64>, count| {
        let size = range.end.saturating_sub(range.start);
        let value = match count {
            Some(n) => format!("{size} bytes, {n} entries"),
            None => format!("{size} bytes"),
        };
        sections.push(Row::entry(name, value, Some(range.start)));
    };

    for payload in Parser::new(0).parse_all(bytes) {
        let payload = match payload {
            Ok(p) => p,
            Err(e) => {
                problem = Some(e.to_string());
                break;
            }
        };
        match payload {
            Payload::Version { num, encoding, .. } => {
                depth += 1;
                if depth > 1 {
                    continue; // a module nested in a component: counted, not walked again
                }
                component = matches!(encoding, wasmparser::Encoding::Component);
                rows.push(Row::field(
                    "kind",
                    "",
                    if component { "component" } else { "module" },
                ));
                rows.push(Row::field("version", "", num.to_string()));
            }
            Payload::End(_) => {
                depth = depth.saturating_sub(1);
            }
            _ if depth > 1 => {}
            Payload::TypeSection(r) => {
                types = r.count();
                section(&mut sections, "type", r.range(), Some(r.count()));
            }
            Payload::ImportSection(r) => {
                let range = r.range();
                let mut n = 0u32;
                for import in r.into_imports().flatten() {
                    n += 1;
                    let kind = match import.ty {
                        wasmparser::TypeRef::Func(_) | wasmparser::TypeRef::FuncExact(_) => {
                            imported_funcs += 1;
                            "func"
                        }
                        wasmparser::TypeRef::Table(_) => "table",
                        wasmparser::TypeRef::Memory(_) => "memory",
                        wasmparser::TypeRef::Global(_) => "global",
                        wasmparser::TypeRef::Tag(_) => "tag",
                    };
                    if imports.len() < ROWS {
                        imports.push(Row::field(
                            format!("{}.{}", import.module, import.name),
                            "",
                            kind,
                        ));
                    }
                }
                section(&mut sections, "import", range, Some(n));
            }
            Payload::FunctionSection(r) => {
                defined = r.count();
                section(&mut sections, "function", r.range(), Some(r.count()));
            }
            Payload::TableSection(r) => section(&mut sections, "table", r.range(), Some(r.count())),
            Payload::MemorySection(r) => {
                section(&mut sections, "memory", r.range(), Some(r.count()))
            }
            Payload::TagSection(r) => section(&mut sections, "tag", r.range(), Some(r.count())),
            Payload::GlobalSection(r) => {
                section(&mut sections, "global", r.range(), Some(r.count()))
            }
            Payload::ExportSection(r) => {
                let range = r.range();
                let n = r.count();
                for export in r.into_iter().flatten() {
                    if exports.len() < ROWS {
                        let kind = match export.kind {
                            wasmparser::ExternalKind::Func
                            | wasmparser::ExternalKind::FuncExact => "func",
                            wasmparser::ExternalKind::Table => "table",
                            wasmparser::ExternalKind::Memory => "memory",
                            wasmparser::ExternalKind::Global => "global",
                            wasmparser::ExternalKind::Tag => "tag",
                        };
                        exports.push(Row::field(
                            export.name.to_string(),
                            "",
                            format!("{kind} {}", export.index),
                        ));
                    }
                }
                section(&mut sections, "export", range, Some(n));
            }
            Payload::StartSection { func, range } => {
                rows.push(Row::field("start", "", format!("func {func}")));
                section(&mut sections, "start", range, None);
            }
            Payload::ElementSection(r) => {
                section(&mut sections, "element", r.range(), Some(r.count()))
            }
            Payload::DataCountSection { range, .. } => {
                section(&mut sections, "data count", range, None)
            }
            Payload::DataSection(r) => section(&mut sections, "data", r.range(), Some(r.count())),
            Payload::CodeSectionStart {
                count,
                unchecked_range,
                ..
            } => {
                section(&mut sections, "code", unchecked_range, Some(count));
            }
            Payload::CodeSectionEntry(body) => {
                let r = body.range();
                bodies.push((r.start, r.end - r.start));
            }
            Payload::CustomSection(r) => {
                let name = format!("custom `{}`", r.name());
                let range = r.range();
                if let KnownCustom::Name(names_reader) = r.as_known() {
                    for name in names_reader.into_iter().flatten() {
                        match name {
                            Name::Module { name, .. } => module_name = Some(name.to_string()),
                            Name::Function(map) => {
                                for naming in map.into_iter().flatten() {
                                    names.insert(naming.index, naming.name.to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                section(&mut sections, &name, range, None);
            }
            Payload::ModuleSection { .. } | Payload::ComponentSection { .. } => {}
            _ => {}
        }
    }

    rows.push(Row::field("size", "", format!("{len} bytes")));
    if let Some(name) = &module_name {
        rows.push(Row::field("name", "", name.clone()));
    }
    if !component {
        rows.push(Row::field("types", "", types.to_string()));
        rows.push(Row::field(
            "functions",
            "",
            format!("{defined} defined, {imported_funcs} imported"),
        ));
    }
    if let Some(problem) = &problem {
        rows.push(Row::field("error", "", problem.clone()));
    }
    if sections.len() > 1 {
        rows.extend(sections);
    }
    if !imports.is_empty() {
        rows.push(Row::group("Imports"));
        rows.extend(imports);
    }
    if !exports.is_empty() {
        rows.push(Row::group("Exports"));
        rows.extend(exports);
    }
    if !bodies.is_empty() {
        rows.push(Row::group("Functions"));
        for (i, (offset, size)) in bodies.iter().enumerate().take(ROWS) {
            let index = imported_funcs + i as u32;
            let label = match names.get(&index) {
                Some(name) => format!("func {index} `{name}`"),
                None => format!("func {index}"),
            };
            rows.push(Row::entry(label, format!("{size} bytes"), Some(*offset)));
        }
        if bodies.len() > ROWS {
            rows.push(Row::field("…", "", format!("{} more", bodies.len() - ROWS)));
        }
    }
    let what = if component { "component" } else { "module" };
    let summary = match &module_name {
        Some(name) => format!("WebAssembly {what} `{name}`"),
        None => format!("WebAssembly {what}"),
    };
    Some(Structure {
        format: "WebAssembly",
        summary,
        rows,
        symbols: Vec::new(),
        maps: Vec::new(),
        clr: None,
        truncated: (bytes.len() as u64) < len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `(module (func (export "answer") (result i32) i32.const 42))`, with a `name` section that
    /// calls the function `answer_fn`.
    fn answer() -> Vec<u8> {
        let mut m = vec![
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // header
            0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f, // type: () -> i32
            0x03, 0x02, 0x01, 0x00, // function section
            0x07, 0x0a, 0x01, 0x06, b'a', b'n', b's', b'w', b'e', b'r', 0x00, 0x00, // export
            0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2a, 0x0b, // code: i32.const 42
        ];
        // custom "name": subsection 1 (function names): one entry, index 0, "answer_fn".
        let mut sub = Vec::new();
        let fname = b"answer_fn";
        let mut map = vec![0x01, 0x00, fname.len() as u8];
        map.extend(fname);
        sub.push(0x01);
        sub.push(map.len() as u8);
        sub.extend(&map);
        let mut custom = vec![0x04];
        custom.extend(b"name");
        custom.extend(&sub);
        m.push(0x00);
        m.push(custom.len() as u8);
        m.extend(custom);
        m
    }

    #[test]
    fn a_module_is_printed_as_its_text_format() {
        let text = listing(&answer()).expect("a module");
        assert!(text.starts_with("(module"), "{text}");
        assert!(text.contains("(export \"answer\""), "{text}");
        assert!(text.contains("i32.const 42"), "{text}");
        assert!(
            text.contains("$answer_fn"),
            "the name section is used: {text}"
        );
    }

    #[test]
    fn a_simd_instruction_is_printed_too() {
        // (module (func (result v128) v128.const i32x4 1 2 3 4))
        let mut m = vec![
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, //
            0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7b, // type: () -> v128
            0x03, 0x02, 0x01, 0x00, //
        ];
        let mut body = vec![0x00, 0xfd, 0x0c];
        for n in [1u32, 2, 3, 4] {
            body.extend(n.to_le_bytes());
        }
        body.push(0x0b);
        let mut code = vec![0x01, body.len() as u8];
        code.extend(&body);
        m.push(0x0a);
        m.push(code.len() as u8);
        m.extend(code);
        let text = listing(&m).expect("a module with SIMD");
        assert!(text.contains("v128.const"), "{text}");
    }

    #[test]
    fn the_structure_pane_lists_sections_exports_and_named_functions() {
        let m = answer();
        let s = read(&m, m.len() as u64).expect("a module");
        let get = |label: &str| {
            s.rows
                .iter()
                .find(|r| r.label == label)
                .map(|r| r.value.clone())
                .unwrap_or_default()
        };
        assert_eq!(s.summary, "WebAssembly module");
        assert_eq!(get("kind"), "module");
        assert_eq!(get("version"), "1");
        assert_eq!(get("functions"), "1 defined, 0 imported");
        assert_eq!(get("answer"), "func 0");
        assert_eq!(get("export"), "10 bytes, 1 entries");
        let func = s
            .rows
            .iter()
            .find(|r| r.label == "func 0 `answer_fn`")
            .unwrap_or_else(|| panic!("{:#?}", s.rows));
        assert_eq!(func.value, "4 bytes");
        let at = func.offset.expect("where its code is") as usize;
        assert_eq!(&m[at..at + 4], &[0x00, 0x41, 0x2a, 0x0b]);
        assert!(s.rows.iter().any(|r| r.label == "custom `name`"));
    }

    #[test]
    fn a_damaged_module_says_why_and_does_not_panic() {
        let mut m = answer();
        m.truncate(20);
        assert!(listing(&m).is_err());
        let s = read(&m, m.len() as u64).expect("still a module");
        assert!(s.rows.iter().any(|r| r.label == "error"), "{:#?}", s.rows);
        for cut in 0..m.len() {
            let _ = read(&m[..cut], cut as u64);
            let _ = listing(&m[..cut]);
        }
    }
}

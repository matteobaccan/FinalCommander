//! What a picture made by an image model says about how it was made (SPEC 05 §1.8): the tool,
//! the model and its LoRAs, the prompt, the seed, the steps, the guidance, the sampler.
//!
//! Each tool writes it its own way, and all of them into the text a picture can carry:
//!
//! - **ComfyUI** — a PNG's `prompt` text is the graph that ran, as JSON, and `workflow` the
//!   graph as the editor draws it; a WebP keeps the same two in EXIF, `prompt:` in the Model
//!   field and `workflow:` in the Make field. The summary is read off the graph by following
//!   the sampler's inputs back to where they come from — `KSampler`, and Flux's
//!   `SamplerCustomAdvanced` with `RandomNoise`, `FluxGuidance`, `BasicScheduler`,
//!   `UNETLoader` and `DualCLIPLoader`.
//! - **A1111** and **Forge** — `parameters`: the prompt, `Negative prompt:`, then one line of
//!   `Key: value` pairs; in a JPEG or a WebP the same text is the EXIF user comment.
//! - **Fooocus**, **SwarmUI** — `parameters` again, as JSON.
//! - **InvokeAI** — `invokeai_metadata`, JSON.
//! - **NovelAI** — `Software` says so, `Description` is the prompt, `Comment` the settings.
//! - **C2PA** content credentials — a signed manifest (`caBX` in a PNG, `C2PA` in a WebP,
//!   APP11 in a JPEG): who made the claim, and whether it says the picture was generated.
//!
//! Nothing here is trusted to be well formed: a field that is missing is a row that is not
//! there, and text that is not the JSON it should be is shown as the text it is.

use serde_json::Value;

use crate::structure::{Row, RowKind};

/// The most a text chunk may inflate to: a ComfyUI workflow is a few hundred kilobytes, and
/// anything past this is not a caption.
pub(crate) const TEXT_LIMIT: usize = 16 * 1024 * 1024;

/// One piece of text the picture carries, and where it is, for `Enter` to read it again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Text {
    pub key: String,
    pub value: String,
    /// Where `Enter` finds it: a PNG chunk's start, or an EXIF value's first byte.
    pub at: u64,
}

/// A PNG text chunk's key and text: `tEXt` (Latin-1), `zTXt` (deflated Latin-1) and `iTXt`
/// (UTF-8, deflated or not).
pub(crate) fn chunk_text(kind: &[u8], body: &[u8]) -> Option<(String, String)> {
    let nul = body.iter().position(|&c| c == 0)?;
    let key = String::from_utf8_lossy(&body[..nul]).into_owned();
    let rest = &body[nul + 1..];
    let latin1 = |b: &[u8]| b.iter().map(|&c| c as char).collect::<String>();
    let value = match kind {
        b"tEXt" => latin1(rest),
        b"zTXt" => latin1(&inflate(rest.get(1..)?)?),
        b"iTXt" => {
            let compressed = *rest.first()? == 1;
            let mut parts = rest.get(2..)?.splitn(3, |&c| c == 0);
            let _lang = parts.next()?;
            let _translated = parts.next()?;
            let text = parts.next()?;
            if compressed {
                String::from_utf8_lossy(&inflate(text)?).into_owned()
            } else {
                String::from_utf8_lossy(text).into_owned()
            }
        }
        _ => return None,
    };
    Some((key, value))
}

fn inflate(b: &[u8]) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(b)
        .take(TEXT_LIMIT as u64)
        .read_to_end(&mut out)
        .ok()?;
    Some(out)
}

/// The texts an EXIF block carries that image tools write into: Make and Model (ComfyUI's
/// WebP), the description, and the user comment (A1111's JPEG and WebP). `base` is where the
/// TIFF header is in the file.
pub(crate) fn exif_texts(b: &[u8], base: u64) -> Vec<Text> {
    let little = match b.get(..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return Vec::new(),
    };
    let u16_at = |at: usize| -> Option<u16> {
        let s: [u8; 2] = b.get(at..at + 2)?.try_into().ok()?;
        Some(if little {
            u16::from_le_bytes(s)
        } else {
            u16::from_be_bytes(s)
        })
    };
    let u32_at = |at: usize| -> Option<u32> {
        let s: [u8; 4] = b.get(at..at + 4)?.try_into().ok()?;
        Some(if little {
            u32::from_le_bytes(s)
        } else {
            u32::from_be_bytes(s)
        })
    };
    let mut out = Vec::new();
    let mut ifds = vec![u32_at(4).unwrap_or(0) as usize];
    let mut seen = 0;
    while let Some(ifd) = ifds.pop() {
        seen += 1;
        if seen > 4 || ifd == 0 {
            continue;
        }
        let Some(n) = u16_at(ifd) else { continue };
        for i in 0..n.min(512) as usize {
            let e = ifd + 2 + i * 12;
            let (Some(tag), Some(kind), Some(count)) = (u16_at(e), u16_at(e + 2), u32_at(e + 4))
            else {
                break;
            };
            if tag == 0x8769 {
                if let Some(sub) = u32_at(e + 8) {
                    ifds.push(sub as usize);
                }
                continue;
            }
            let key = match tag {
                0x010F => "Make",
                0x0110 => "Model",
                0x010E => "ImageDescription",
                0x9286 => "UserComment",
                0x9C9C => "XPComment",
                _ => continue,
            };
            let unit = match kind {
                1 | 2 | 7 => 1,
                3 => 2,
                _ => continue,
            };
            let size = unit * count as usize;
            let at = if size <= 4 {
                e + 8
            } else {
                u32_at(e + 8).unwrap_or(u32::MAX) as usize
            };
            let Some(data) = b.get(at..at.saturating_add(size)) else {
                continue;
            };
            let value = exif_value(key, data);
            if !value.trim().is_empty() {
                out.push(Text {
                    key: key.to_string(),
                    value,
                    at: base + at as u64,
                });
            }
        }
    }
    out
}

/// The text of one EXIF value: a user comment has an eight-byte character set in front of it,
/// `UNICODE` meaning UTF-16 in either order; Windows' comment is UTF-16 little-endian; the
/// rest is ASCII up to its NUL.
pub(crate) fn exif_value(key: &str, data: &[u8]) -> String {
    let utf16 = |b: &[u8], little: bool| {
        let units: Vec<u16> = b
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| {
                if little {
                    u16::from_le_bytes(*c)
                } else {
                    u16::from_be_bytes(*c)
                }
            })
            .take_while(|&u| u != 0)
            .collect();
        String::from_utf16_lossy(&units)
    };
    match key {
        "UserComment" => {
            let (head, rest) = data.split_at(data.len().min(8));
            if head.starts_with(b"UNICODE") {
                // The order is not written down; a picture's text starts with a letter, whose
                // zero byte says which half is high.
                let little = rest.len() >= 2 && rest[1] == 0 && rest[0] != 0;
                utf16(rest, little)
            } else {
                String::from_utf8_lossy(rest.split(|&c| c == 0).next().unwrap_or(&[])).into_owned()
            }
        }
        "XPComment" => utf16(data, true),
        _ => String::from_utf8_lossy(data.split(|&c| c == 0).next().unwrap_or(&[])).into_owned(),
    }
}

/// What a picture says about how it was made, from the texts it carries and its C2PA
/// manifest. Empty when it says nothing of the kind.
pub(crate) fn rows(texts: &[Text], c2pa: Option<&[u8]>) -> Vec<Row> {
    let mut gen = Gen::default();
    let get = |k: &str| texts.iter().find(|t| t.key.eq_ignore_ascii_case(k));
    // ComfyUI's WebP spells its two graphs into the camera fields.
    let prefixed = |prefix: &str| {
        texts
            .iter()
            .find_map(|t| t.value.strip_prefix(prefix).map(|v| (v.to_string(), t.at)))
    };
    let prompt_graph = get("prompt")
        .filter(|t| t.value.trim_start().starts_with('{'))
        .map(|t| (t.value.clone(), t.at))
        .or_else(|| prefixed("prompt:"));
    let workflow = get("workflow")
        .map(|t| (t.value.clone(), t.at))
        .or_else(|| prefixed("workflow:"));
    let parameters = get("parameters").or_else(|| get("UserComment"));
    let mut sources: Vec<(&'static str, u64)> = Vec::new();

    if let Some((graph, at)) = &prompt_graph {
        if let Ok(Value::Object(nodes)) = serde_json::from_str::<Value>(graph) {
            gen.tool = "ComfyUI".into();
            comfy(&nodes, &mut gen);
            sources.push(("prompt (graph)", *at));
        }
    }
    if let Some((_, at)) = &workflow {
        if gen.tool.is_empty() {
            gen.tool = "ComfyUI".into();
        }
        sources.push(("workflow", *at));
    }
    if gen.tool.is_empty() {
        if let Some(t) = get("invokeai_metadata") {
            if let Ok(v) = serde_json::from_str::<Value>(&t.value) {
                gen.tool = "InvokeAI".into();
                invoke(&v, &mut gen);
                sources.push(("invokeai_metadata", t.at));
            }
        }
    }
    if gen.tool.is_empty() && get("Software").is_some_and(|t| t.value.contains("NovelAI")) {
        gen.tool = "NovelAI".into();
        if let Some(t) = get("Source") {
            gen.model = t.value.trim().to_string();
        }
        if let Some(t) = get("Description") {
            gen.prompt = t.value.trim().to_string();
        }
        if let Some(t) = get("Comment") {
            if let Ok(v) = serde_json::from_str::<Value>(&t.value) {
                novelai(&v, &mut gen);
            }
            sources.push(("Comment", t.at));
        }
    }
    if gen.tool.is_empty() {
        if let Some(t) = parameters {
            let text = t.value.trim();
            if text.starts_with('{') {
                if let Ok(v) = serde_json::from_str::<Value>(text) {
                    if v.get("sui_image_params").is_some() {
                        gen.tool = "SwarmUI".into();
                        swarm(&v["sui_image_params"], &mut gen);
                    } else {
                        gen.tool = "Fooocus".into();
                        fooocus(&v, &mut gen);
                    }
                    sources.push(("parameters", t.at));
                }
            } else if text.contains("Steps:") {
                a1111(text, &mut gen);
                sources.push(("parameters", t.at));
            }
        }
    }
    let credentials = c2pa.map(credentials);
    if gen.tool.is_empty() && credentials.is_none() {
        return Vec::new();
    }

    let mut rows = vec![Row::group("AI generation")];
    let mut put = |label: &str, value: &str| {
        if !value.trim().is_empty() {
            rows.push(Row::field(label, "", one_line(value)));
        }
    };
    put("tool", &gen.tool);
    put("model", &gen.model);
    for (name, weight) in &gen.loras {
        let value = match weight {
            Some(w) => format!("{name} ({w})"),
            None => name.clone(),
        };
        put("LoRA", &value);
    }
    put("text encoders", &gen.encoders.join(", "));
    put("VAE", &gen.vae);
    put("prompt", &gen.prompt);
    put("negative", &gen.negative);
    put("seed", &gen.seed);
    put("steps", &gen.steps);
    put("cfg", &gen.cfg);
    put("guidance", &gen.guidance);
    put("sampler", &gen.sampler);
    put("scheduler", &gen.scheduler);
    put("denoise", &gen.denoise);
    put("size", &gen.size);
    put("version", &gen.version);
    for (label, at) in sources {
        rows.push(Row {
            kind: RowKind::Text,
            label: label.to_string(),
            raw: String::new(),
            value: "Enter: the whole of it".to_string(),
            offset: Some(at),
        });
    }
    if let Some(c) = credentials {
        rows.push(Row::group("Content credentials (C2PA)"));
        rows.push(Row::field(
            "manifest",
            "",
            format!("{} bytes, signed", c.bytes),
        ));
        if !c.generator.is_empty() {
            rows.push(Row::field("claimed by", "", c.generator));
        }
        if let Some(source) = c.source {
            rows.push(Row::field("source", "", source));
        }
    }
    rows
}

/// A value on one line of the structure pane: the rest is for `Enter`.
fn one_line(s: &str) -> String {
    let flat: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > 400 {
        let cut: String = flat.chars().take(400).collect();
        format!("{cut}…")
    } else {
        flat
    }
}

#[derive(Debug, Default)]
struct Gen {
    tool: String,
    model: String,
    loras: Vec<(String, Option<String>)>,
    encoders: Vec<String>,
    vae: String,
    prompt: String,
    negative: String,
    seed: String,
    steps: String,
    cfg: String,
    guidance: String,
    sampler: String,
    scheduler: String,
    denoise: String,
    size: String,
    version: String,
}

/// A JSON value as the text a row shows: strings bare, numbers as written.
fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

// ── ComfyUI ──────────────────────────────────────────────────────────────────────────────────

type Nodes = serde_json::Map<String, Value>;

fn class(node: &Value) -> &str {
    node.get("class_type").and_then(Value::as_str).unwrap_or("")
}

/// The node an input is wired to: `["12", 0]`.
fn linked<'a>(nodes: &'a Nodes, v: &Value) -> Option<&'a Value> {
    let id = v.as_array()?.first()?;
    let id = match id {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => return None,
    };
    nodes.get(&id)
}

/// An input's value: written in the node, or the one value of the node it is wired to (a
/// primitive, a string node), followed a few steps.
fn input(nodes: &Nodes, node: &Value, name: &str) -> Option<String> {
    let mut v = node.get("inputs")?.get(name)?;
    for _ in 0..8 {
        if let Some(s) = scalar(v) {
            return Some(s);
        }
        let next = linked(nodes, v)?;
        let inputs = next.get("inputs")?.as_object()?;
        v = [
            "value",
            "string",
            "text",
            "seed",
            "noise_seed",
            "int",
            "float",
        ]
        .iter()
        .find_map(|k| inputs.get(*k))
        .or_else(|| {
            (inputs.len() == 1)
                .then(|| inputs.values().next())
                .flatten()
        })?;
    }
    None
}

/// What a conditioning input says in words: the text of the encoder it comes from, through
/// the nodes that only pass it on — and the Flux guidance met on the way. A node that carries
/// both sides (`InstructPixToPixConditioning`, `ControlNetApplyAdvanced`) passes on the one its
/// output number says: the first is the positive, the second the negative.
fn conditioning(nodes: &Nodes, v: &Value, guidance: &mut String) -> String {
    let mut link = v.clone();
    for _ in 0..16 {
        let Some(n) = linked(nodes, &link) else { break };
        let output = link.get(1).and_then(Value::as_u64).unwrap_or(0);
        let inputs = n.get("inputs").and_then(Value::as_object);
        match class(n) {
            "FluxGuidance" => {
                if let Some(g) = input(nodes, n, "guidance") {
                    *guidance = g;
                }
            }
            c if c.starts_with("CLIPTextEncode") => {
                if let Some(g) = input(nodes, n, "guidance") {
                    if guidance.is_empty() {
                        *guidance = g;
                    }
                }
                // SDXL's encoder has two prompts, Flux's two as well; the first that says
                // something is the prompt.
                for k in ["text", "t5xxl", "text_g", "clip_l", "text_l"] {
                    if let Some(t) = input(nodes, n, k).filter(|t| !t.trim().is_empty()) {
                        return t;
                    }
                }
                break;
            }
            _ => {}
        }
        let side = if output == 1 { "negative" } else { "positive" };
        let next = inputs.and_then(|i| {
            i.get("conditioning")
                .or_else(|| i.get(side))
                .or_else(|| i.get("conditioning_1"))
        });
        match next {
            Some(n) => link = n.clone(),
            None => break,
        }
    }
    String::new()
}

/// The model a sampler runs, followed back to its loader, picking up the LoRAs on the way.
fn model_chain(nodes: &Nodes, v: &Value, gen: &mut Gen) {
    let mut node = linked(nodes, v);
    for _ in 0..32 {
        let Some(n) = node else { break };
        let c = class(n);
        if c.contains("Lora") {
            if let Some(name) = input(nodes, n, "lora_name") {
                let w = input(nodes, n, "strength_model");
                gen.loras.push((name, w));
            }
        }
        for k in ["ckpt_name", "unet_name", "model_name", "gguf_name"] {
            if let Some(name) = input(nodes, n, k) {
                gen.model = name;
                if let Some(enc) = input(nodes, n, "weight_dtype").filter(|d| d != "default") {
                    gen.model.push_str(&format!(" ({enc})"));
                }
                return;
            }
        }
        node = n
            .get("inputs")
            .and_then(|i| i.get("model"))
            .and_then(|m| linked(nodes, m));
    }
}

fn comfy(nodes: &Nodes, gen: &mut Gen) {
    let mut by_id: Vec<(&String, &Value)> = nodes.iter().collect();
    by_id.sort_by_key(|(id, _)| id.parse::<u64>().unwrap_or(u64::MAX));
    for (_, n) in &by_id {
        match class(n) {
            "DualCLIPLoader" | "TripleCLIPLoader" | "CLIPLoader" => {
                for k in ["clip_name1", "clip_name2", "clip_name3", "clip_name"] {
                    if let Some(c) = input(nodes, n, k) {
                        gen.encoders.push(c);
                    }
                }
            }
            "VAELoader" => {
                if let Some(v) = input(nodes, n, "vae_name") {
                    gen.vae = v;
                }
            }
            _ => {}
        }
    }
    // The sampler: the classic one says everything itself; the custom one through what is
    // wired to it.
    let sampler = by_id.iter().map(|(_, n)| *n).find(|n| {
        matches!(
            class(n),
            "KSampler" | "KSamplerAdvanced" | "SamplerCustomAdvanced" | "SamplerCustom"
        )
    });
    let Some(s) = sampler else { return };
    let inputs = s.get("inputs").cloned().unwrap_or(Value::Null);
    // The size of the picture made from nothing: an empty latent wired to the sampler. One
    // made from another picture (an edit, a depth map) has the size of that picture.
    if let Some(latent) = linked(nodes, &inputs["latent_image"]) {
        if class(latent).starts_with("Empty") {
            if let (Some(w), Some(h)) = (
                input(nodes, latent, "width"),
                input(nodes, latent, "height"),
            ) {
                gen.size = format!("{w} × {h}");
            }
        }
    }
    let mut guidance = String::new();
    match class(s) {
        "SamplerCustomAdvanced" => {
            if let Some(noise) = linked(nodes, &inputs["noise"]) {
                gen.seed = input(nodes, noise, "noise_seed").unwrap_or_default();
            }
            if let Some(sel) = linked(nodes, &inputs["sampler"]) {
                gen.sampler = input(nodes, sel, "sampler_name").unwrap_or_default();
            }
            if let Some(sig) = linked(nodes, &inputs["sigmas"]) {
                gen.scheduler = input(nodes, sig, "scheduler").unwrap_or_default();
                gen.steps = input(nodes, sig, "steps").unwrap_or_default();
                gen.denoise = input(nodes, sig, "denoise").unwrap_or_default();
                if let Some(m) = sig.get("inputs").and_then(|i| i.get("model")) {
                    model_chain(nodes, m, gen);
                }
            }
            if let Some(guider) = linked(nodes, &inputs["guider"]) {
                let g = guider.get("inputs").cloned().unwrap_or(Value::Null);
                if let Some(c) = input(nodes, guider, "cfg") {
                    gen.cfg = c;
                }
                let pos = g.get("conditioning").or_else(|| g.get("positive"));
                if let Some(p) = pos {
                    gen.prompt = conditioning(nodes, p, &mut guidance);
                }
                if let Some(n) = g.get("negative") {
                    let mut ignore = String::new();
                    gen.negative = conditioning(nodes, n, &mut ignore);
                }
                if gen.model.is_empty() {
                    if let Some(m) = g.get("model") {
                        model_chain(nodes, m, gen);
                    }
                }
            }
        }
        _ => {
            gen.seed = input(nodes, s, "seed")
                .or_else(|| input(nodes, s, "noise_seed"))
                .unwrap_or_default();
            gen.steps = input(nodes, s, "steps").unwrap_or_default();
            gen.cfg = input(nodes, s, "cfg").unwrap_or_default();
            gen.sampler = input(nodes, s, "sampler_name").unwrap_or_default();
            gen.scheduler = input(nodes, s, "scheduler").unwrap_or_default();
            gen.denoise = input(nodes, s, "denoise").unwrap_or_default();
            gen.prompt = conditioning(nodes, &inputs["positive"], &mut guidance);
            let mut ignore = String::new();
            gen.negative = conditioning(nodes, &inputs["negative"], &mut ignore);
            model_chain(nodes, &inputs["model"], gen);
        }
    }
    gen.guidance = guidance;
    if gen.denoise == "1" || gen.denoise == "1.0" {
        gen.denoise.clear();
    }
}

// ── A1111, Forge ─────────────────────────────────────────────────────────────────────────────

fn a1111(text: &str, gen: &mut Gen) {
    // The prompt, then `Negative prompt:`, then the last line of settings.
    let (body, settings) = match text.rfind("\nSteps:") {
        Some(at) => (&text[..at], &text[at + 1..]),
        None if text.starts_with("Steps:") => ("", text),
        None => (text, ""),
    };
    let (prompt, negative) = match body.find("Negative prompt:") {
        Some(at) => (&body[..at], body[at + "Negative prompt:".len()..].trim()),
        None => (body, ""),
    };
    gen.prompt = prompt.trim().to_string();
    gen.negative = negative.to_string();
    // `<lora:name:0.8>` in the prompt.
    let mut rest = gen.prompt.as_str();
    while let Some(at) = rest.find("<lora:") {
        let tail = &rest[at + 6..];
        let Some(end) = tail.find('>') else { break };
        let mut parts = tail[..end].splitn(2, ':');
        let name = parts.next().unwrap_or("").to_string();
        let weight = parts.next().map(str::to_string);
        gen.loras.push((name, weight));
        rest = &tail[end..];
    }
    for (k, v) in settings_pairs(settings) {
        match k.as_str() {
            "Steps" => gen.steps = v,
            "Sampler" => gen.sampler = v,
            "Schedule type" => gen.scheduler = v,
            "CFG scale" => gen.cfg = v,
            "Distilled CFG Scale" => gen.guidance = v,
            "Seed" => gen.seed = v,
            "Size" => gen.size = v.replace('x', " × "),
            "Model" => gen.model = v,
            "Denoising strength" => gen.denoise = v,
            "VAE" => gen.vae = v,
            "Version" => gen.version = v,
            _ => {}
        }
    }
    gen.tool = if gen.version.starts_with('f') {
        "Forge".into()
    } else {
        "AUTOMATIC1111".into()
    };
}

/// `Steps: 20, Sampler: Euler a, Lora hashes: "a: 1, b: 2"`: the pairs, quotes respected.
fn settings_pairs(line: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut quoted = false;
    let mut piece = String::new();
    let flush = |piece: &mut String, out: &mut Vec<(String, String)>| {
        if let Some((k, v)) = piece.split_once(':') {
            out.push((k.trim().to_string(), v.trim().trim_matches('"').to_string()));
        }
        piece.clear();
    };
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                piece.push(c);
            }
            ',' if !quoted => flush(&mut piece, &mut out),
            _ => piece.push(c),
        }
    }
    flush(&mut piece, &mut out);
    out
}

// ── The JSON ones ────────────────────────────────────────────────────────────────────────────

fn pick(v: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|k| v.get(*k).and_then(scalar))
        .unwrap_or_default()
}

fn fooocus(v: &Value, gen: &mut Gen) {
    gen.prompt = pick(v, &["prompt", "Prompt"]);
    gen.negative = pick(v, &["negative_prompt", "Negative Prompt"]);
    gen.model = pick(v, &["base_model", "Base Model"]);
    gen.seed = pick(v, &["seed", "Seed"]);
    gen.steps = pick(v, &["steps", "Steps"]);
    gen.cfg = pick(v, &["guidance_scale", "Guidance Scale"]);
    gen.sampler = pick(v, &["sampler", "Sampler"]);
    gen.scheduler = pick(v, &["scheduler", "Scheduler"]);
    gen.size = pick(v, &["resolution", "Resolution"]);
    gen.version = pick(v, &["version", "Version"]);
    if let Some(Value::Array(loras)) = v.get("loras") {
        for l in loras {
            if let Some(a) = l.as_array() {
                let name = a.first().and_then(scalar).unwrap_or_default();
                let w = a.get(1).and_then(scalar);
                gen.loras.push((name, w));
            }
        }
    }
}

fn swarm(v: &Value, gen: &mut Gen) {
    gen.prompt = pick(v, &["prompt"]);
    gen.negative = pick(v, &["negativeprompt"]);
    gen.model = pick(v, &["model"]);
    gen.seed = pick(v, &["seed"]);
    gen.steps = pick(v, &["steps"]);
    gen.cfg = pick(v, &["cfgscale"]);
    gen.guidance = pick(v, &["fluxguidancescale"]);
    gen.sampler = pick(v, &["sampler"]);
    gen.scheduler = pick(v, &["scheduler"]);
    let (w, h) = (pick(v, &["width"]), pick(v, &["height"]));
    if !w.is_empty() {
        gen.size = format!("{w} × {h}");
    }
    gen.version = pick(v, &["swarm_version"]);
    let weights: Vec<String> = v
        .get("loraweights")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(scalar).collect())
        .unwrap_or_default();
    if let Some(Value::Array(loras)) = v.get("loras") {
        for (i, l) in loras.iter().enumerate() {
            if let Some(name) = scalar(l) {
                gen.loras.push((name, weights.get(i).cloned()));
            }
        }
    }
}

fn invoke(v: &Value, gen: &mut Gen) {
    gen.prompt = pick(v, &["positive_prompt", "positive_style_prompt"]);
    gen.negative = pick(v, &["negative_prompt"]);
    gen.model = v
        .get("model")
        .map(|m| pick(m, &["name", "model_name"]))
        .unwrap_or_default();
    gen.seed = pick(v, &["seed"]);
    gen.steps = pick(v, &["steps"]);
    gen.cfg = pick(v, &["cfg_scale"]);
    gen.guidance = pick(v, &["guidance"]);
    gen.scheduler = pick(v, &["scheduler"]);
    let (w, h) = (pick(v, &["width"]), pick(v, &["height"]));
    if !w.is_empty() {
        gen.size = format!("{w} × {h}");
    }
    gen.version = pick(v, &["app_version"]);
    if let Some(Value::Array(loras)) = v.get("loras") {
        for l in loras {
            let name = l
                .get("model")
                .or_else(|| l.get("lora"))
                .map(|m| pick(m, &["name", "model_name"]))
                .unwrap_or_default();
            gen.loras.push((name, l.get("weight").and_then(scalar)));
        }
    }
}

fn novelai(v: &Value, gen: &mut Gen) {
    if gen.prompt.is_empty() {
        gen.prompt = pick(v, &["prompt"]);
    }
    gen.negative = pick(v, &["uc"]);
    gen.seed = pick(v, &["seed"]);
    gen.steps = pick(v, &["steps"]);
    gen.cfg = pick(v, &["scale"]);
    gen.sampler = pick(v, &["sampler"]);
    gen.scheduler = pick(v, &["noise_schedule"]);
    let (w, h) = (pick(v, &["width"]), pick(v, &["height"]));
    if !w.is_empty() {
        gen.size = format!("{w} × {h}");
    }
}

// ── C2PA ─────────────────────────────────────────────────────────────────────────────────────

struct Credentials {
    bytes: usize,
    generator: String,
    source: Option<String>,
}

/// What a C2PA manifest store says without checking its signature: the generator that made
/// the claim, and the IPTC digital source type when it calls the picture generated. The
/// store is CBOR inside JUMBF; the two strings are found by their CBOR keys.
fn credentials(b: &[u8]) -> Credentials {
    let after = |key: &[u8]| -> Option<String> {
        let at = find(b, key)? + key.len();
        cbor_text(b.get(at..)?)
    };
    let mut generator = after(b"\x6fclaim_generator").unwrap_or_default();
    if generator.is_empty() {
        // Newer claims name it in a list of maps: `claim_generator_info: [{name: …}]`.
        if let Some(at) = find(b, b"\x74claim_generator_info") {
            if let Some(n) = find(&b[at..], b"\x64name") {
                generator = cbor_text(&b[at + n + 5..]).unwrap_or_default();
            }
        }
    }
    let source = find(b, b"digitalsourcetype/").and_then(|at| {
        let tail = &b[at + 18..];
        let end = tail
            .iter()
            .position(|c| !c.is_ascii_alphanumeric())
            .unwrap_or(tail.len());
        let kind = String::from_utf8_lossy(&tail[..end]).into_owned();
        let said = match kind.as_str() {
            "trainedAlgorithmicMedia" => "generated by a model",
            "compositeWithTrainedAlgorithmicMedia" => "partly generated by a model",
            "algorithmicMedia" => "made by an algorithm",
            "digitalCapture" => "a camera's capture",
            _ => "",
        };
        (!kind.is_empty()).then(|| {
            if said.is_empty() {
                kind
            } else {
                format!("{said} ({kind})")
            }
        })
    });
    Credentials {
        bytes: b.len(),
        generator,
        source,
    }
}

/// A CBOR text string at the start of `b`.
fn cbor_text(b: &[u8]) -> Option<String> {
    let head = *b.first()?;
    if head >> 5 != 3 {
        return None;
    }
    let (len, from) = match head & 0x1F {
        n @ 0..=23 => (n as usize, 1),
        24 => (*b.get(1)? as usize, 2),
        25 => (u16::from_be_bytes([*b.get(1)?, *b.get(2)?]) as usize, 3),
        _ => return None,
    };
    Some(String::from_utf8_lossy(b.get(from..from + len)?).into_owned())
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// The whole of a text `Enter` was pressed on, ready to read: JSON laid out, the rest as it is.
pub(crate) fn readable(text: &str) -> String {
    let t = text.trim();
    for prefix in ["prompt:", "workflow:"] {
        if let Some(rest) = t.strip_prefix(prefix) {
            return readable(rest);
        }
    }
    if t.starts_with('{') || t.starts_with('[') {
        if let Ok(v) = serde_json::from_str::<Value>(t) {
            if let Ok(pretty) = serde_json::to_string_pretty(&v) {
                return pretty;
            }
        }
    }
    t.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(pairs: &[(&str, &str)]) -> Vec<Text> {
        pairs
            .iter()
            .enumerate()
            .map(|(i, (k, v))| Text {
                key: k.to_string(),
                value: v.to_string(),
                at: i as u64 * 100,
            })
            .collect()
    }

    fn get(rows: &[Row], label: &str) -> String {
        rows.iter()
            .find(|r| r.label == label)
            .map(|r| r.value.clone())
            .unwrap_or_default()
    }

    /// The classic graph: a checkpoint, a LoRA, two encoders and a `KSampler`.
    #[test]
    fn a_comfyui_graph_is_followed_back_from_its_sampler() {
        let graph = r#"{
          "3": {"class_type": "KSampler", "inputs": {"seed": 42, "steps": 20, "cfg": 7.5,
                "sampler_name": "euler", "scheduler": "normal", "denoise": 1,
                "model": ["10", 0], "positive": ["6", 0], "negative": ["7", 0],
                "latent_image": ["5", 0]}},
          "4": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": "sd_xl.safetensors"}},
          "10": {"class_type": "LoraLoader", "inputs": {"lora_name": "style.safetensors",
                 "strength_model": 0.8, "model": ["4", 0], "clip": ["4", 1]}},
          "5": {"class_type": "EmptyLatentImage", "inputs": {"width": 1024, "height": 768, "batch_size": 1}},
          "6": {"class_type": "CLIPTextEncode", "inputs": {"text": "a lighthouse at dusk", "clip": ["10", 1]}},
          "7": {"class_type": "CLIPTextEncode", "inputs": {"text": "blurry", "clip": ["10", 1]}}
        }"#;
        let r = rows(&texts(&[("prompt", graph), ("workflow", "{}")]), None);
        assert_eq!(get(&r, "tool"), "ComfyUI");
        assert_eq!(get(&r, "model"), "sd_xl.safetensors");
        assert_eq!(get(&r, "LoRA"), "style.safetensors (0.8)");
        assert_eq!(get(&r, "prompt"), "a lighthouse at dusk");
        assert_eq!(get(&r, "negative"), "blurry");
        assert_eq!(get(&r, "seed"), "42");
        assert_eq!(get(&r, "steps"), "20");
        assert_eq!(get(&r, "cfg"), "7.5");
        assert_eq!(get(&r, "sampler"), "euler");
        assert_eq!(get(&r, "scheduler"), "normal");
        assert_eq!(get(&r, "size"), "1024 × 768");
        assert_eq!(
            get(&r, "denoise"),
            "",
            "1 is the default and not worth a row"
        );
        let opens: Vec<_> = r.iter().filter(|r| r.kind == RowKind::Text).collect();
        assert_eq!(opens.len(), 2, "the graph and the workflow open with Enter");
        assert_eq!(opens[0].offset, Some(0));
    }

    /// Flux's graph, the one of ComfyUI's own example: the sampler says nothing itself, and
    /// everything is in the nodes wired to it.
    #[test]
    fn a_flux_graph_is_read_through_the_custom_sampler() {
        let graph = r#"{
          "13": {"class_type": "SamplerCustomAdvanced", "inputs": {"noise": ["25", 0],
                 "guider": ["22", 0], "sampler": ["16", 0], "sigmas": ["17", 0], "latent_image": ["27", 0]}},
          "25": {"class_type": "RandomNoise", "inputs": {"noise_seed": 219670278747233}},
          "22": {"class_type": "BasicGuider", "inputs": {"model": ["30", 0], "conditioning": ["26", 0]}},
          "26": {"class_type": "FluxGuidance", "inputs": {"guidance": 3.5, "conditioning": ["6", 0]}},
          "6": {"class_type": "CLIPTextEncode", "inputs": {"text": "cute anime girl", "clip": ["11", 0]}},
          "16": {"class_type": "KSamplerSelect", "inputs": {"sampler_name": "euler"}},
          "17": {"class_type": "BasicScheduler", "inputs": {"scheduler": "simple", "steps": 20,
                 "denoise": 1, "model": ["30", 0]}},
          "30": {"class_type": "ModelSamplingFlux", "inputs": {"model": ["12", 0], "max_shift": 1.15}},
          "12": {"class_type": "UNETLoader", "inputs": {"unet_name": "flux1-dev.safetensors", "weight_dtype": "fp8_e4m3fn"}},
          "11": {"class_type": "DualCLIPLoader", "inputs": {"clip_name1": "t5xxl_fp16.safetensors",
                 "clip_name2": "clip_l.safetensors", "type": "flux"}},
          "10": {"class_type": "VAELoader", "inputs": {"vae_name": "ae.safetensors"}},
          "27": {"class_type": "EmptySD3LatentImage", "inputs": {"width": 1024, "height": 1024, "batch_size": 1}}
        }"#;
        let r = rows(&texts(&[("prompt", graph)]), None);
        assert_eq!(get(&r, "model"), "flux1-dev.safetensors (fp8_e4m3fn)");
        assert_eq!(
            get(&r, "text encoders"),
            "t5xxl_fp16.safetensors, clip_l.safetensors"
        );
        assert_eq!(get(&r, "VAE"), "ae.safetensors");
        assert_eq!(get(&r, "prompt"), "cute anime girl");
        assert_eq!(get(&r, "seed"), "219670278747233");
        assert_eq!(get(&r, "guidance"), "3.5");
        assert_eq!(get(&r, "sampler"), "euler");
        assert_eq!(get(&r, "scheduler"), "simple");
        assert_eq!(get(&r, "steps"), "20");
    }

    /// A node that carries both sides passes on the one its output number names, and a
    /// latent made from another picture says no size.
    #[test]
    fn a_node_with_both_sides_is_followed_on_the_right_one() {
        let graph = r#"{
          "3": {"class_type": "KSampler", "inputs": {"seed": 1, "model": ["31", 0],
                "positive": ["35", 0], "negative": ["35", 1], "latent_image": ["35", 2]}},
          "7": {"class_type": "CLIPTextEncode", "inputs": {"text": "dull"}},
          "23": {"class_type": "CLIPTextEncode", "inputs": {"text": "a shark"}},
          "26": {"class_type": "FluxGuidance", "inputs": {"guidance": 10.0, "conditioning": ["23", 0]}},
          "31": {"class_type": "UNETLoader", "inputs": {"unet_name": "flux1-dev.safetensors", "weight_dtype": "default"}},
          "35": {"class_type": "InstructPixToPixConditioning", "inputs": {"positive": ["26", 0], "negative": ["7", 0]}}
        }"#;
        let r = rows(&texts(&[("prompt", graph)]), None);
        assert_eq!(get(&r, "prompt"), "a shark");
        assert_eq!(get(&r, "negative"), "dull");
        assert_eq!(get(&r, "guidance"), "10.0");
        assert_eq!(get(&r, "model"), "flux1-dev.safetensors");
        assert_eq!(get(&r, "size"), "");
    }

    /// A WebP from ComfyUI keeps the graph in the camera fields.
    #[test]
    fn a_comfyui_webp_spells_its_graph_into_make_and_model() {
        let graph = r#"{"1": {"class_type": "KSampler", "inputs": {"seed": 7, "steps": 4}}}"#;
        let model = format!("prompt:{graph}");
        let r = rows(&texts(&[("Make", "workflow:{}"), ("Model", &model)]), None);
        assert_eq!(get(&r, "tool"), "ComfyUI");
        assert_eq!(get(&r, "seed"), "7");
    }

    #[test]
    fn a1111_and_forge_parameters_are_split_into_their_fields() {
        let p = "a castle, <lora:stone:0.6>\nNegative prompt: ugly, lowres\nSteps: 30, Sampler: DPM++ 2M, Schedule type: Karras, CFG scale: 6, Seed: 1234, Size: 832x1216, Model hash: abc, Model: juggernaut, Lora hashes: \"stone: 1a2b\", Version: v1.10.1";
        let r = rows(&texts(&[("parameters", p)]), None);
        assert_eq!(get(&r, "tool"), "AUTOMATIC1111");
        assert_eq!(get(&r, "prompt"), "a castle, <lora:stone:0.6>");
        assert_eq!(get(&r, "negative"), "ugly, lowres");
        assert_eq!(get(&r, "LoRA"), "stone (0.6)");
        assert_eq!(get(&r, "steps"), "30");
        assert_eq!(get(&r, "scheduler"), "Karras");
        assert_eq!(get(&r, "size"), "832 × 1216");
        assert_eq!(get(&r, "model"), "juggernaut");
        let forge = p.replace("v1.10.1", "f2.0.1v1.10.1-previous");
        let r = rows(&texts(&[("parameters", &forge)]), None);
        assert_eq!(get(&r, "tool"), "Forge");
        // The same text as a JPEG's user comment.
        let r = rows(&texts(&[("UserComment", p)]), None);
        assert_eq!(get(&r, "seed"), "1234");
    }

    #[test]
    fn the_json_tools_are_told_apart() {
        let swarm = r#"{"sui_image_params": {"prompt": "a fox", "model": "flux-schnell", "seed": 5,
            "steps": 4, "cfgscale": 1, "width": 512, "height": 512, "loras": ["fur"], "loraweights": ["0.7"]}}"#;
        let r = rows(&texts(&[("parameters", swarm)]), None);
        assert_eq!(get(&r, "tool"), "SwarmUI");
        assert_eq!(get(&r, "LoRA"), "fur (0.7)");
        let fooocus = r#"{"prompt": "a cat", "base_model": "juggernautXL", "seed": "9", "loras": [["detail.safetensors", 0.5]]}"#;
        let r = rows(&texts(&[("parameters", fooocus)]), None);
        assert_eq!(get(&r, "tool"), "Fooocus");
        assert_eq!(get(&r, "LoRA"), "detail.safetensors (0.5)");
        let invoke = r#"{"positive_prompt": "a ship", "model": {"name": "dreamshaper"}, "seed": 3, "width": 640, "height": 480}"#;
        let r = rows(&texts(&[("invokeai_metadata", invoke)]), None);
        assert_eq!(get(&r, "tool"), "InvokeAI");
        assert_eq!(get(&r, "model"), "dreamshaper");
        assert_eq!(get(&r, "size"), "640 × 480");
        let r = rows(
            &texts(&[
                ("Software", "NovelAI"),
                ("Description", "a dragon"),
                ("Source", "NovelAI Diffusion V4"),
                (
                    "Comment",
                    r#"{"steps": 28, "scale": 5, "seed": 11, "sampler": "k_euler", "uc": "bad"}"#,
                ),
            ]),
            None,
        );
        assert_eq!(get(&r, "tool"), "NovelAI");
        assert_eq!(get(&r, "prompt"), "a dragon");
        assert_eq!(get(&r, "negative"), "bad");
        assert_eq!(get(&r, "model"), "NovelAI Diffusion V4");
    }

    #[test]
    fn a_c2pa_manifest_says_who_claims_it_and_that_a_model_made_it() {
        let mut m = b"jumb....c2pa".to_vec();
        m.extend(b"\x6fclaim_generator\x6eDALL\xc2\xb7E 3 v1.0");
        m.extend(b"...http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia\x00");
        let r = rows(&[], Some(&m));
        assert_eq!(get(&r, "claimed by"), "DALL·E 3 v1.0");
        assert!(get(&r, "source").starts_with("generated by a model"));
    }

    #[test]
    fn nothing_of_the_kind_gives_no_rows() {
        assert!(rows(&texts(&[("Software", "GIMP"), ("Title", "lake")]), None).is_empty());
    }

    #[test]
    fn compressed_text_chunks_are_read() {
        use std::io::Write;
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(b"{\"a\": 1}").unwrap();
        let packed = z.finish().unwrap();
        let mut ztxt = b"prompt\0\0".to_vec();
        ztxt.extend(&packed);
        assert_eq!(
            chunk_text(b"zTXt", &ztxt),
            Some(("prompt".into(), "{\"a\": 1}".into()))
        );
        let mut itxt = b"workflow\0\x01\0\0\0".to_vec();
        itxt.extend(&packed);
        assert_eq!(
            chunk_text(b"iTXt", &itxt),
            Some(("workflow".into(), "{\"a\": 1}".into()))
        );
        assert_eq!(readable("prompt:{\"a\":1}"), "{\n  \"a\": 1\n}");
    }

    #[test]
    fn exif_texts_find_make_model_and_the_user_comment() {
        // Little-endian TIFF: IFD0 with Make and a pointer to the Exif IFD with a comment.
        let mut t = b"II*\0\x08\0\0\0".to_vec();
        let make = b"workflow:{}\0";
        let comment: Vec<u8> = b"UNICODE\0"
            .iter()
            .copied()
            .chain("Steps: 5".encode_utf16().flat_map(|u| u.to_le_bytes()))
            .collect();
        // IFD0 at 8: two entries.
        let ifd0 = 8;
        let data0 = ifd0 + 2 + 2 * 12 + 4;
        let exif_ifd = data0 + make.len();
        let data1 = exif_ifd + 2 + 12 + 4;
        t.extend(2u16.to_le_bytes());
        t.extend(0x010Fu16.to_le_bytes());
        t.extend(2u16.to_le_bytes());
        t.extend((make.len() as u32).to_le_bytes());
        t.extend((data0 as u32).to_le_bytes());
        t.extend(0x8769u16.to_le_bytes());
        t.extend(4u16.to_le_bytes());
        t.extend(1u32.to_le_bytes());
        t.extend((exif_ifd as u32).to_le_bytes());
        t.extend(0u32.to_le_bytes());
        t.extend(make);
        t.extend(1u16.to_le_bytes());
        t.extend(0x9286u16.to_le_bytes());
        t.extend(7u16.to_le_bytes());
        t.extend((comment.len() as u32).to_le_bytes());
        t.extend((data1 as u32).to_le_bytes());
        t.extend(0u32.to_le_bytes());
        t.extend(&comment);
        let found = exif_texts(&t, 1000);
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(found[0].value, "workflow:{}");
        assert_eq!(found[0].at, 1000 + data0 as u64);
        assert_eq!(found[1].value, "Steps: 5");
    }
}

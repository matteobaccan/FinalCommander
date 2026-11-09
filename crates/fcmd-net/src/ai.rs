//! Asking a language model a question (SPEC 06 §14).
//!
//! Three wire formats cover the services people run: OpenAI's chat completions (which most
//! servers and proxies also speak — LM Studio, vLLM, OpenRouter, Azure's), Anthropic's
//! messages, and Ollama's chat on a local machine. One request, one answer, no streaming:
//! the answer is a document that goes into a file and the viewer, not a chat window. A
//! follow-up sends the earlier turns of its thread again, because none of the three keeps a
//! conversation on its side.
//!
//! Nothing is sent unless the user typed `ai:` and, for files, said yes to a dialog that
//! named them and the host they were going to.

use std::time::Duration;

use serde_json::{json, Value};

/// How to reach one service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    pub api: Api,
    pub url: String,
    pub model: String,
    pub token: Option<String>,
    pub system: String,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Api {
    OpenAi,
    Anthropic,
    Ollama,
}

impl Api {
    pub fn parse(text: &str) -> Option<Api> {
        match text.trim().to_ascii_lowercase().as_str() {
            "openai" | "" => Some(Api::OpenAi),
            "anthropic" | "claude" => Some(Api::Anthropic),
            "ollama" => Some(Api::Ollama),
            _ => None,
        }
    }
}

/// A file sent with the question: its name, and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub name: String,
    pub text: String,
}

/// The question with the files after it, each fenced and named, which every model reads.
pub fn user_text(question: &str, files: &[Attachment]) -> String {
    let mut text = question.trim().to_string();
    for file in files {
        text.push_str(&format!(
            "\n\n--- {} ---\n```\n{}\n```",
            file.name, file.text
        ));
    }
    text
}

/// One earlier turn of a thread: what was asked and what came back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    pub question: String,
    pub answer: String,
}

/// What came back: the answer, and what it cost as the service counts it — `None` where the
/// reply does not say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub text: String,
    pub tokens_in: Option<u64>,
    pub tokens_out: Option<u64>,
}

/// The tokens a reply says it used, read where each service puts them.
pub fn usage(api: Api, reply: &Value) -> (Option<u64>, Option<u64>) {
    let n = |p: &str| reply.pointer(p).and_then(Value::as_u64);
    let shapes = [
        (n("/usage/input_tokens"), n("/usage/output_tokens")),
        (n("/usage/prompt_tokens"), n("/usage/completion_tokens")),
        (n("/prompt_eval_count"), n("/eval_count")),
    ];
    let own = match api {
        Api::Anthropic => 0,
        Api::OpenAi => 1,
        Api::Ollama => 2,
    };
    // Its own shape first, then whichever the reply has, as for the answer itself.
    std::iter::once(shapes[own])
        .chain(shapes)
        .find(|(i, o)| i.is_some() || o.is_some())
        .unwrap_or((None, None))
}

/// The body of the request, in the service's own shape.
pub fn body(service: &Service, question: &str, files: &[Attachment]) -> Value {
    body_with(service, &[], question, files)
}

/// The body with the earlier turns of a thread in front of the new question, as the
/// alternating user and assistant messages every one of the three formats reads.
pub fn body_with(
    service: &Service,
    earlier: &[Turn],
    question: &str,
    files: &[Attachment],
) -> Value {
    let user = user_text(question, files);
    let mut turns = Vec::new();
    for t in earlier {
        turns.push(json!({"role": "user", "content": t.question}));
        turns.push(json!({"role": "assistant", "content": t.answer}));
    }
    turns.push(json!({"role": "user", "content": user}));
    let max = if service.max_tokens == 0 {
        4096
    } else {
        service.max_tokens
    };
    match service.api {
        Api::Anthropic => {
            let mut body = json!({
                "model": service.model,
                "max_tokens": max,
                "messages": turns,
            });
            if !service.system.is_empty() {
                body["system"] = json!(service.system);
            }
            body
        }
        Api::OpenAi | Api::Ollama => {
            let mut messages = Vec::new();
            if !service.system.is_empty() {
                messages.push(json!({"role": "system", "content": service.system}));
            }
            messages.extend(turns);
            let mut body = json!({"model": service.model, "messages": messages});
            if service.api == Api::Ollama {
                body["stream"] = json!(false);
            } else {
                body["max_tokens"] = json!(max);
            }
            body
        }
    }
}

/// The answer's text out of the service's reply, or what the service said went wrong.
pub fn answer(api: Api, reply: &Value) -> Result<String, String> {
    if let Some(error) = reply.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| error.to_string());
        return Err(message);
    }
    let anthropic = || {
        reply
            .get("content")
            .and_then(Value::as_array)
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|b| b.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("")
            })
    };
    let at = |path: &str| {
        reply
            .pointer(path)
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    let openai = || at("/choices/0/message/content");
    let ollama = || at("/message/content");
    // The protocol's own shape first, then the others: a local server picked as Ollama that
    // answers the OpenAI way (llama.cpp, LM Studio, vLLM under `/v1`) has still answered.
    let text = match api {
        Api::Anthropic => anthropic().or_else(openai).or_else(ollama),
        Api::OpenAi => openai().or_else(ollama).or_else(anthropic),
        Api::Ollama => ollama().or_else(openai).or_else(anthropic),
    };
    text.ok_or_else(|| format!("no answer in the reply: {}", short(&reply.to_string())))
}

fn short(text: &str) -> String {
    let mut t: String = text.chars().take(300).collect();
    if t.len() < text.len() {
        t.push('…');
    }
    t
}

/// Ask, and wait for the whole answer.
pub async fn ask(
    service: &Service,
    question: &str,
    files: &[Attachment],
) -> Result<String, String> {
    ask_thread(service, &[], question, files)
        .await
        .map(|r| r.text)
}

/// Ask with the earlier turns of a thread, and say what it cost.
pub async fn ask_thread(
    service: &Service,
    earlier: &[Turn],
    question: &str,
    files: &[Attachment],
) -> Result<Reply, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .user_agent(concat!("fcmd/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let mut request = client
        .post(endpoint(service.api, &service.url))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body_with(service, earlier, question, files).to_string());
    if let Some(token) = &service.token {
        request = match service.api {
            Api::Anthropic => request
                .header("x-api-key", token)
                .header("anthropic-version", "2023-06-01"),
            Api::OpenAi | Api::Ollama => request.bearer_auth(token),
        };
    } else if service.api == Api::Anthropic {
        request = request.header("anthropic-version", "2023-06-01");
    }
    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    let text = response.text().await.map_err(|e| e.to_string())?;
    let reply: Value =
        serde_json::from_str(&text).map_err(|_| format!("HTTP {status}: {}", short(&text)))?;
    let (tokens_in, tokens_out) = usage(service.api, &reply);
    match answer(service.api, &reply) {
        Ok(text) => Ok(Reply {
            text,
            tokens_in,
            tokens_out,
        }),
        Err(e) if !status.is_success() => Err(format!("HTTP {status}: {e}")),
        Err(e) => Err(e),
    }
}

/// The address a question is posted to. A list of models pasted in place of the endpoint
/// (`/api/tags`, `/v1/models`) is taken back to the endpoint beside it, and a bare
/// `scheme://host[:port]` gets the protocol's usual path.
pub fn endpoint(api: Api, url: &str) -> String {
    let mut url = url.trim().trim_end_matches('/').to_string();
    for tail in ["/api/tags", "/models"] {
        if let Some(base) = url.strip_suffix(tail) {
            url = base.trim_end_matches('/').to_string();
        }
    }
    let path = match api {
        Api::OpenAi => "/chat/completions",
        Api::Anthropic => "/messages",
        Api::Ollama => "/api/chat",
    };
    let bare = url
        .split_once("://")
        .map_or(url.as_str(), |(_, rest)| rest)
        .split_once('/')
        .is_none();
    if bare {
        let v1 = if api == Api::Ollama { "" } else { "/v1" };
        return format!("{url}{v1}{path}");
    }
    // `/v1` alone, as some servers document it.
    if url.ends_with("/v1") || (url.ends_with("/api") && api == Api::Ollama) {
        return match api {
            Api::Ollama => format!("{}{path}", url.trim_end_matches("/api")),
            _ => format!("{url}{path}"),
        };
    }
    url
}

/// An answer read as it is written: the bytes of a streamed reply go in as they come, and
/// what each of them adds to the text comes out. It reads the three shapes a stream comes in
/// — OpenAI's and Anthropic's server-sent events, Ollama's lines of JSON — and, for a server
/// that ignored the request to stream, the whole reply once it is all there.
#[derive(Debug, Default)]
pub struct Stream {
    pending: Vec<u8>,
    whole: Vec<u8>,
    pub text: String,
    pub tokens_in: Option<u64>,
    pub tokens_out: Option<u64>,
    pub error: Option<String>,
}

impl Stream {
    /// Bytes as they arrive; returns the text they add, which may be nothing yet.
    pub fn feed(&mut self, bytes: &[u8]) -> String {
        if self.whole.len() < 4 << 20 {
            self.whole.extend_from_slice(bytes);
        }
        self.pending.extend_from_slice(bytes);
        let mut added = String::new();
        while let Some(at) = self.pending.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=at).collect();
            let line = String::from_utf8_lossy(&line).into_owned();
            if let Some(piece) = self.line(&line) {
                added.push_str(&piece);
            }
        }
        self.text.push_str(&added);
        added
    }

    /// The end of the reply: a last line with no newline after it, or a reply that was
    /// never streamed at all, read whole.
    pub fn finish(&mut self, api: Api) -> String {
        let rest = std::mem::take(&mut self.pending);
        let mut added = String::new();
        if !rest.is_empty() {
            if let Some(piece) = self.line(&String::from_utf8_lossy(&rest)) {
                added.push_str(&piece);
            }
        }
        if self.text.is_empty() && added.is_empty() && self.error.is_none() {
            if let Ok(reply) = serde_json::from_slice::<Value>(&self.whole) {
                let (i, o) = usage(api, &reply);
                self.tokens_in = self.tokens_in.or(i);
                self.tokens_out = self.tokens_out.or(o);
                match answer(api, &reply) {
                    Ok(text) => added = text,
                    Err(e) => self.error = Some(e),
                }
            }
        }
        self.text.push_str(&added);
        added
    }

    fn line(&mut self, line: &str) -> Option<String> {
        let line = line.trim();
        let data = line.strip_prefix("data:").map(str::trim).unwrap_or(line);
        if data.is_empty()
            || data == "[DONE]"
            || line.starts_with("event:")
            || line.starts_with(':')
        {
            return None;
        }
        let v: Value = serde_json::from_str(data).ok()?;
        if let Some(error) = v.get("error") {
            self.error = Some(
                error
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| error.to_string()),
            );
            return None;
        }
        let n = |p: &str| v.pointer(p).and_then(Value::as_u64);
        for (i, o) in [
            (n("/usage/prompt_tokens"), n("/usage/completion_tokens")),
            (n("/prompt_eval_count"), n("/eval_count")),
            (
                n("/message/usage/input_tokens"),
                n("/message/usage/output_tokens"),
            ),
            (n("/usage/input_tokens"), n("/usage/output_tokens")),
        ] {
            self.tokens_in = i.or(self.tokens_in);
            self.tokens_out = o.or(self.tokens_out);
        }
        let s = |p: &str| v.pointer(p).and_then(Value::as_str).map(str::to_string);
        s("/choices/0/delta/content")
            .or_else(|| s("/delta/text"))
            .or_else(|| s("/message/content"))
            .or_else(|| s("/choices/0/message/content"))
            .filter(|t| !t.is_empty())
    }
}

/// Ask with the earlier turns, the answer read as it is written: `on_text` is called with
/// each piece as it arrives, so it can be shown while the rest is still coming.
pub async fn ask_streaming(
    service: &Service,
    earlier: &[Turn],
    question: &str,
    files: &[Attachment],
    mut on_text: impl FnMut(&str) + Send,
) -> Result<Reply, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .user_agent(concat!("fcmd/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let mut body = body_with(service, earlier, question, files);
    body["stream"] = json!(true);
    let mut request = client
        .post(endpoint(service.api, &service.url))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body.to_string());
    if let Some(token) = &service.token {
        request = match service.api {
            Api::Anthropic => request
                .header("x-api-key", token)
                .header("anthropic-version", "2023-06-01"),
            Api::OpenAi | Api::Ollama => request.bearer_auth(token),
        };
    } else if service.api == Api::Anthropic {
        request = request.header("anthropic-version", "2023-06-01");
    }
    let mut response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        let why = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| answer(service.api, &v).err())
            .unwrap_or_else(|| short(&text));
        return Err(format!("HTTP {status}: {why}"));
    }
    let mut stream = Stream::default();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        let added = stream.feed(&chunk);
        if !added.is_empty() {
            on_text(&added);
        }
    }
    let added = stream.finish(service.api);
    if !added.is_empty() {
        on_text(&added);
    }
    if let Some(e) = stream.error {
        return Err(e);
    }
    if stream.text.is_empty() {
        return Err(format!(
            "no answer in the reply: {}",
            short(&String::from_utf8_lossy(&stream.whole))
        ));
    }
    Ok(Reply {
        text: stream.text,
        tokens_in: stream.tokens_in,
        tokens_out: stream.tokens_out,
    })
}

/// Where a service lists its models — the cheapest question that proves the address, the
/// key and the protocol are right without asking the model anything: `…/models` beside an
/// OpenAI or Anthropic endpoint, `…/api/tags` beside Ollama's chat.
pub fn models_url(api: Api, url: &str) -> String {
    let url = endpoint(api, url);
    let url = url.as_str();
    let cut = |suffix: &str| url.strip_suffix(suffix).map(str::to_string);
    match api {
        Api::OpenAi => cut("/chat/completions")
            .or_else(|| cut("/completions"))
            .map(|base| format!("{base}/models"))
            .unwrap_or_else(|| format!("{url}/models")),
        Api::Anthropic => cut("/messages")
            .map(|base| format!("{base}/models"))
            .unwrap_or_else(|| format!("{url}/models")),
        // Ollama speaks OpenAI's API too, under `/v1`: an address of that shape is asked the
        // OpenAI way.
        Api::Ollama => cut("/api/chat")
            .or_else(|| cut("/api/generate"))
            .map(|base| format!("{base}/api/tags"))
            .or_else(|| cut("/v1/chat/completions").map(|base| format!("{base}/v1/models")))
            .unwrap_or_else(|| format!("{url}/api/tags")),
    }
}

/// The models a list names, from any of the three shapes (`data[].id`, `models[].name`).
pub fn model_names(reply: &Value) -> Vec<String> {
    let list = reply
        .get("data")
        .or_else(|| reply.get("models"))
        .and_then(Value::as_array);
    list.map(|items| {
        items
            .iter()
            .filter_map(|m| {
                m.get("id")
                    .or_else(|| m.get("name"))
                    .or_else(|| m.get("model"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .collect()
    })
    .unwrap_or_default()
}

/// What a probe of a service found: how it answered, how long it took, how many models it
/// lists and whether the one configured is among them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    pub url: String,
    pub status: u16,
    pub millis: u128,
    pub models: usize,
    pub has_model: bool,
}

/// Knock on the service: one `GET` of its list of models, with the key it will be asked with.
/// Nothing is sent but the key, and nothing is generated, so it costs nothing.
pub async fn probe(service: &Service) -> Result<Probe, String> {
    let url = models_url(service.api, &service.url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(concat!("fcmd/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let mut request = client.get(&url);
    if let Some(token) = &service.token {
        request = match service.api {
            Api::Anthropic => request
                .header("x-api-key", token)
                .header("anthropic-version", "2023-06-01"),
            Api::OpenAi | Api::Ollama => request.bearer_auth(token),
        };
    } else if service.api == Api::Anthropic {
        request = request.header("anthropic-version", "2023-06-01");
    }
    let started = std::time::Instant::now();
    let response = request.send().await.map_err(|e| format!("{url}: {e}"))?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    let millis = started.elapsed().as_millis();
    if !status.is_success() {
        return Err(format!("{url}: HTTP {status}: {}", short(&text)));
    }
    let names = serde_json::from_str::<Value>(&text)
        .map(|v| model_names(&v))
        .unwrap_or_default();
    let has_model = service.model.is_empty() || names.iter().any(|n| n == &service.model);
    Ok(Probe {
        url,
        status: status.as_u16(),
        millis,
        models: names.len(),
        has_model,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(api: Api) -> Service {
        Service {
            api,
            url: "http://localhost/x".into(),
            model: "m".into(),
            token: None,
            system: "be brief".into(),
            max_tokens: 0,
        }
    }

    #[test]
    fn the_list_of_models_is_beside_the_endpoint() {
        assert_eq!(
            models_url(Api::OpenAi, "https://api.openai.com/v1/chat/completions"),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            models_url(Api::Anthropic, "https://api.anthropic.com/v1/messages"),
            "https://api.anthropic.com/v1/models"
        );
        assert_eq!(
            models_url(Api::Ollama, "http://localhost:11434/api/chat"),
            "http://localhost:11434/api/tags"
        );
        // What the login reports must be the endpoint's own list, whatever was pasted.
        assert_eq!(
            models_url(
                Api::Ollama,
                "http://localhost:8080/v1/chat/completions/api/tags"
            ),
            "http://localhost:8080/v1/models"
        );
        assert_eq!(
            endpoint(
                Api::Ollama,
                "http://localhost:8080/v1/chat/completions/api/tags"
            ),
            "http://localhost:8080/v1/chat/completions"
        );
        assert_eq!(
            endpoint(Api::OpenAi, "http://localhost:8080/v1/models"),
            "http://localhost:8080/v1/chat/completions"
        );
        assert_eq!(
            endpoint(Api::Ollama, "http://localhost:11434"),
            "http://localhost:11434/api/chat"
        );
        assert_eq!(
            endpoint(Api::OpenAi, "https://api.openai.com/v1"),
            "https://api.openai.com/v1/chat/completions"
        );
        let openai = json!({"data": [{"id": "gpt-x"}, {"id": "gpt-y"}]});
        assert_eq!(model_names(&openai), ["gpt-x", "gpt-y"]);
        // An Ollama service whose server answers the OpenAI way has still answered.
        let openai_reply = json!({"choices": [{"message": {"role": "assistant", "content": "ciao"}}],
            "usage": {"prompt_tokens": 2, "completion_tokens": 3}});
        assert_eq!(answer(Api::Ollama, &openai_reply).unwrap(), "ciao");
        assert_eq!(usage(Api::Ollama, &openai_reply), (Some(2), Some(3)));
        let ollama = json!({"models": [{"name": "llama3:8b"}]});
        assert_eq!(model_names(&ollama), ["llama3:8b"]);
    }

    #[test]
    fn a_streamed_answer_is_read_as_it_comes_in_each_shape() {
        // OpenAI's events, cut in the middle of a line and of a character.
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"Ciao\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\" è\"}}]}\n\ndata: {\"choices\":[],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":2}}\n\ndata: [DONE]\n\n";
        let bytes = sse.as_bytes();
        let cut = sse.find('è').unwrap() + 1;
        let mut s = Stream::default();
        let first = s.feed(&bytes[..20]);
        assert_eq!(first, "");
        s.feed(&bytes[20..cut]);
        s.feed(&bytes[cut..]);
        s.finish(Api::OpenAi);
        assert_eq!(s.text, "Ciao è");
        assert_eq!((s.tokens_in, s.tokens_out), (Some(3), Some(2)));

        // Ollama's lines of JSON, the last without its newline.
        let mut s = Stream::default();
        s.feed(b"{\"message\":{\"content\":\"a\"},\"done\":false}\n");
        s.feed(b"{\"message\":{\"content\":\"b\"},\"done\":true,\"prompt_eval_count\":5,\"eval_count\":6}");
        s.finish(Api::Ollama);
        assert_eq!(s.text, "ab");
        assert_eq!((s.tokens_in, s.tokens_out), (Some(5), Some(6)));

        // Anthropic's events.
        let mut s = Stream::default();
        s.feed(b"event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":7}}}\n\n");
        s.feed(b"event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\n\n");
        s.feed(b"data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":1}}\n\n");
        s.finish(Api::Anthropic);
        assert_eq!(s.text, "hi");
        assert_eq!((s.tokens_in, s.tokens_out), (Some(7), Some(1)));

        // A server that did not stream: the whole reply, read at the end.
        let mut s = Stream::default();
        s.feed(b"{\n  \"choices\": [{\"message\": {\"content\": \"whole\"}}]\n}");
        s.finish(Api::OpenAi);
        assert_eq!(s.text, "whole");

        // An error said in the stream is the error.
        let mut s = Stream::default();
        s.feed(b"data: {\"error\":{\"message\":\"no model\"}}\n");
        s.finish(Api::OpenAi);
        assert_eq!(s.error.as_deref(), Some("no model"));
    }

    #[test]
    fn each_api_gets_its_own_body() {
        let files = [Attachment {
            name: "a.rs".into(),
            text: "fn main() {}".into(),
        }];
        let b = body(&service(Api::Anthropic), "why?", &files);
        assert_eq!(b["system"], "be brief");
        assert_eq!(b["max_tokens"], 4096);
        let user = b["messages"][0]["content"].as_str().unwrap();
        assert!(
            user.starts_with("why?") && user.contains("--- a.rs ---") && user.contains("fn main")
        );
        let b = body(&service(Api::OpenAi), "why?", &[]);
        assert_eq!(b["messages"][0]["role"], "system");
        assert_eq!(b["messages"][1]["content"], "why?");
        let b = body(&service(Api::Ollama), "why?", &[]);
        assert_eq!(b["stream"], false);
        assert!(b.get("max_tokens").is_none());
    }

    #[test]
    fn the_answer_is_read_from_each_reply_and_an_error_is_said() {
        let r =
            json!({"content": [{"type": "text", "text": "Hel"}, {"type": "text", "text": "lo"}]});
        assert_eq!(answer(Api::Anthropic, &r).unwrap(), "Hello");
        let r = json!({"choices": [{"message": {"content": "Hi"}}]});
        assert_eq!(answer(Api::OpenAi, &r).unwrap(), "Hi");
        let r = json!({"message": {"content": "Ciao"}});
        assert_eq!(answer(Api::Ollama, &r).unwrap(), "Ciao");
        let r = json!({"error": {"message": "invalid x-api-key"}});
        assert_eq!(answer(Api::Anthropic, &r).unwrap_err(), "invalid x-api-key");
        assert!(answer(Api::OpenAi, &json!({})).is_err());
        assert_eq!(Api::parse("Claude"), Some(Api::Anthropic));
        assert_eq!(Api::parse("nope"), None);
    }

    #[test]
    fn a_thread_sends_its_earlier_turns_and_the_cost_is_read() {
        let earlier = [Turn {
            question: "first?".into(),
            answer: "one".into(),
        }];
        let b = body_with(&service(Api::Anthropic), &earlier, "and then?", &[]);
        let m = b["messages"].as_array().unwrap();
        assert_eq!(m.len(), 3);
        assert_eq!(m[0]["content"], "first?");
        assert_eq!(m[1]["role"], "assistant");
        assert_eq!(m[2]["content"], "and then?");
        let b = body_with(&service(Api::OpenAi), &earlier, "and then?", &[]);
        assert_eq!(
            b["messages"].as_array().unwrap().len(),
            4,
            "the system prompt first"
        );
        let r = json!({"usage": {"input_tokens": 12, "output_tokens": 30}});
        assert_eq!(usage(Api::Anthropic, &r), (Some(12), Some(30)));
        let r = json!({"usage": {"prompt_tokens": 5, "completion_tokens": 7}});
        assert_eq!(usage(Api::OpenAi, &r), (Some(5), Some(7)));
        let r = json!({"prompt_eval_count": 3, "eval_count": 4});
        assert_eq!(usage(Api::Ollama, &r), (Some(3), Some(4)));
        assert_eq!(usage(Api::Ollama, &json!({})), (None, None));
    }

    /// A streamed answer over a real socket: the pieces reach the caller one by one, before
    /// the reply is over, and the whole is what they add up to.
    #[tokio::test]
    async fn a_streamed_answer_arrives_in_pieces() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = sock.read(&mut buf).unwrap();
                got.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&got);
                if let Some(end) = text.find("\r\n\r\n") {
                    let len = text[..end]
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if got.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            write!(
                sock,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            for piece in ["Hel", "lo"] {
                write!(
                    sock,
                    "data: {{\"choices\":[{{\"delta\":{{\"content\":\"{piece}\"}}}}]}}\n\n"
                )
                .unwrap();
                sock.flush().unwrap();
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            write!(sock, "data: [DONE]\n\n").unwrap();
            String::from_utf8_lossy(&got).into_owned()
        });
        let mut s = service(Api::OpenAi);
        s.url = format!("http://127.0.0.1:{port}/v1/chat/completions");
        let mut pieces = Vec::new();
        let reply = ask_streaming(&s, &[], "hi?", &[], |p| pieces.push(p.to_string()))
            .await
            .unwrap();
        assert_eq!(reply.text, "Hello");
        assert_eq!(pieces, ["Hel", "lo"]);
        let request = server.join().unwrap();
        assert!(request.contains("\"stream\":true"), "{request}");
    }

    /// A whole round trip against a server on this machine that answers as Ollama does, and
    /// checks it was sent the question and the token.
    #[tokio::test]
    async fn a_question_goes_out_and_the_answer_comes_back() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = sock.read(&mut buf).unwrap();
                got.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&got);
                if let Some(end) = text.find("\r\n\r\n") {
                    let len = text[..end]
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if got.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            let reply = r#"{"message":{"role":"assistant","content":"forty-two"}}"#;
            write!(
                sock,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                reply.len(),
                reply
            )
            .unwrap();
            String::from_utf8_lossy(&got).into_owned()
        });
        let mut s = service(Api::Ollama);
        s.url = format!("http://127.0.0.1:{port}/api/chat");
        s.token = Some("sekrit".into());
        let answer = ask(&s, "what is it?", &[]).await.unwrap();
        assert_eq!(answer, "forty-two");
        let request = server.join().unwrap();
        assert!(request.contains("what is it?"), "{request}");
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer sekrit"));
    }
}

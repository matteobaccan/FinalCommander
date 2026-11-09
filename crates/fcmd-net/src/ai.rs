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
    match api {
        Api::Anthropic => (n("/usage/input_tokens"), n("/usage/output_tokens")),
        Api::OpenAi => (n("/usage/prompt_tokens"), n("/usage/completion_tokens")),
        Api::Ollama => (n("/prompt_eval_count"), n("/eval_count")),
    }
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
    let text = match api {
        Api::Anthropic => reply
            .get("content")
            .and_then(Value::as_array)
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|b| b.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("")
            }),
        Api::OpenAi => reply
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .map(str::to_string),
        Api::Ollama => reply
            .pointer("/message/content")
            .and_then(Value::as_str)
            .map(str::to_string),
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

/// Where a service lists its models — the cheapest question that proves the address, the
/// key and the protocol are right without asking the model anything: `…/models` beside an
/// OpenAI or Anthropic endpoint, `…/api/tags` beside Ollama's chat.
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
        let ollama = json!({"models": [{"name": "llama3:8b"}]});
        assert_eq!(model_names(&ollama), ["llama3:8b"]);
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

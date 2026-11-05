//! Asking a language model a question (SPEC 06 §14).
//!
//! Three wire formats cover the services people run: OpenAI's chat completions (which most
//! servers and proxies also speak — LM Studio, vLLM, OpenRouter, Azure's), Anthropic's
//! messages, and Ollama's chat on a local machine. One request, one answer, no streaming:
//! the answer is a document that goes into a file and the viewer, not a chat window.
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

/// The body of the request, in the service's own shape.
pub fn body(service: &Service, question: &str, files: &[Attachment]) -> Value {
    let user = user_text(question, files);
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
                "messages": [{"role": "user", "content": user}],
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
            messages.push(json!({"role": "user", "content": user}));
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
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .user_agent(concat!("fcmd/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let mut request = client
        .post(&service.url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body(service, question, files).to_string());
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
    match answer(service.api, &reply) {
        Ok(a) => Ok(a),
        Err(e) if !status.is_success() => Err(format!("HTTP {status}: {e}")),
        Err(e) => Err(e),
    }
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

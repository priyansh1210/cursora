//! Every request to a provider goes through here (constitution §3):
//! the local-only guard (PRI-1.1), no redirects, no proxy for local hosts, timeouts (LLM-1.5),
//! cancel (LLM-1.3), SSE framing (LLM-1.2) and error classification (LLM-1.4).
use super::{locality, Timeouts};
use crate::chat::{ChatError, ErrorKind, Locality};
use crate::settings::Settings;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::time::Instant;
use url::Url;

/// What a request may do, from the settings at the time it starts.
#[derive(Clone, Debug, Default)]
pub struct Policy {
    pub trusted: Vec<String>,
    pub local_only: bool,
    pub timeouts: Timeouts,
}

impl Policy {
    pub fn of(s: &Settings) -> Self {
        Self { trusted: s.models.trusted_hosts.clone(), local_only: s.local_only, timeouts: s.models.timeouts.clone() }
    }
}

pub struct Request {
    pub url: Url,
    pub headers: Vec<(&'static str, String)>,
    /// `None` sends a GET.
    pub body: Option<Value>,
}

/// Whether the stream should go on after an event.
#[derive(PartialEq)]
pub enum Flow {
    Continue,
    Stop,
}

pub fn error(kind: ErrorKind, message: impl Into<String>) -> ChatError {
    ChatError { kind, message: message.into() }
}

/// Refuses cloud hosts while local-only is on. Runs before any connection is made (PRI-1.1).
pub fn guard(url: &Url, policy: &Policy) -> Result<Locality, ChatError> {
    let loc = locality(url, &policy.trusted);
    if policy.local_only && loc == Locality::Cloud {
        let host = url.host_str().unwrap_or_default();
        return Err(error(ErrorKind::BlockedLocalOnly, format!("Local-only mode is on, so nothing is sent to {host}.")));
    }
    Ok(loc)
}

fn client(loc: Locality, policy: &Policy) -> Result<reqwest::Client, ChatError> {
    let builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none()) // a local server can't bounce a request to the internet
        .connect_timeout(Duration::from_secs(policy.timeouts.connect_secs))
        .user_agent(concat!("Cursora/", env!("CARGO_PKG_VERSION")));
    // A system proxy would carry "local" traffic off the machine.
    let builder = if loc == Locality::Cloud { builder } else { builder.no_proxy() };
    builder.build().map_err(|e| error(ErrorKind::Unknown, e.to_string()))
}

fn build(req: &Request, client: &reqwest::Client) -> reqwest::RequestBuilder {
    let mut b = match &req.body {
        Some(body) => client.post(req.url.clone()).json(body),
        None => client.get(req.url.clone()),
    };
    for (name, value) in &req.headers {
        b = b.header(*name, value);
    }
    b
}

/// Sends `req` and hands each SSE event (name, data) to `on_event` as soon as it arrives.
/// Returns when the stream ends, `on_event` says Stop or fails, a limit passes, or `cancel` is set.
/// On cancel it returns at once, drops the connection, and passes on no further events.
pub async fn stream(req: Request, policy: &Policy, cancel: &AtomicBool, on_event: &mut dyn FnMut(&str, &str) -> Result<Flow, ChatError>) -> Result<(), ChatError> {
    let loc = guard(&req.url, policy)?;
    let t = &policy.timeouts;
    let first = Duration::from_secs(if loc == Locality::Cloud { t.first_reply_cloud_secs } else { t.first_reply_local_secs });
    let first_deadline = Instant::now() + first;
    let client = client(loc, policy)?;
    let Some(sent) = race(cancel, first_deadline, first, build(&req, &client).send()).await? else { return Ok(()) };
    let mut response = sent.map_err(send_error)?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = race(cancel, Instant::now() + first, first, response.text()).await?.and_then(Result::ok).unwrap_or_default();
        return Err(http_error(Some(status), &body));
    }
    let mut sse = Sse::default();
    let mut seen = false;
    loop {
        let (deadline, limit) = if seen { (Instant::now() + Duration::from_secs(t.pause_secs), Duration::from_secs(t.pause_secs)) } else { (first_deadline, first) };
        let Some(chunk) = race(cancel, deadline, limit, response.chunk()).await? else { return Ok(()) };
        let chunk = chunk.map_err(send_error)?;
        let closed = chunk.is_none();
        let events = match chunk {
            Some(bytes) => sse.push(&bytes),
            None => sse.finish(),
        };
        for (name, data) in events {
            if cancel.load(Ordering::SeqCst) {
                return Ok(());
            }
            seen = true;
            if on_event(&name, &data)? == Flow::Stop {
                return Ok(());
            }
        }
        if closed {
            return Ok(()); // the adapter decides whether the answer was complete
        }
    }
}

/// One JSON request, all within `limit` (model lists, health checks, Test connection).
pub async fn json(req: Request, policy: &Policy, limit: Duration) -> Result<Value, ChatError> {
    let loc = guard(&req.url, policy)?;
    let client = client(loc, policy)?;
    let work = async {
        let response = build(&req, &client).send().await.map_err(send_error)?;
        let status = response.status().as_u16();
        let body = response.text().await.map_err(send_error)?;
        if !(200..300).contains(&status) {
            return Err(http_error(Some(status), &body));
        }
        serde_json::from_str(&body).map_err(|_| error(ErrorKind::Unknown, format!("The server didn't answer with JSON: {}", excerpt(&body))))
    };
    tokio::time::timeout(limit, work).await.map_err(|_| timed_out(limit))?
}

/// Waits for `fut` until the deadline, or returns `None` as soon as `cancel` is set (checked every 25 ms).
async fn race<T>(cancel: &AtomicBool, deadline: Instant, limit: Duration, fut: impl Future<Output = T>) -> Result<Option<T>, ChatError> {
    let cancelled = async {
        while !cancel.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    };
    tokio::select! {
        r = tokio::time::timeout_at(deadline, fut) => r.map(Some).map_err(|_| timed_out(limit)),
        _ = cancelled => Ok(None),
    }
}

fn timed_out(limit: Duration) -> ChatError {
    error(ErrorKind::Network, format!("The model didn't reply within {} s.", limit.as_secs_f32().round()))
}

fn send_error(e: reqwest::Error) -> ChatError {
    // Redirects are refused, so a 3xx arrives as a response, not here.
    error(ErrorKind::Network, format!("Couldn't reach the model: {e}"))
}

/// An HTTP error or an error inside a stream, with the provider's own message when it has one.
pub fn http_error(status: Option<u16>, body: &str) -> ChatError {
    let json: Option<Value> = serde_json::from_str(body).ok();
    let message = json
        .as_ref()
        .and_then(|j| ["/error/message", "/error", "/message"].iter().find_map(|p| j.pointer(p).and_then(Value::as_str)))
        .map(str::to_string)
        .unwrap_or_else(|| match status {
            Some(s @ 300..400) => format!("The server answered with a redirect ({s}), which Cursora doesn't follow. Check the base URL."),
            Some(s) if body.trim().is_empty() => format!("The server answered with status {s}."),
            _ => excerpt(body),
        });
    // A stream error may carry its HTTP status inside (OpenRouter: {"error": {"code": 502, …}}).
    let status = status.or_else(|| json.as_ref().and_then(|j| j.pointer("/error/code")).and_then(Value::as_u64).map(|c| c as u16));
    error(classify(status, body), message)
}

fn excerpt(body: &str) -> String {
    body.trim().chars().take(300).collect()
}

/// LLM-1.4: the HTTP status decides when it is clear; otherwise the body's wording does.
pub fn classify(status: Option<u16>, body: &str) -> ErrorKind {
    match status {
        Some(401 | 403) => return ErrorKind::Auth,
        Some(429) => return ErrorKind::RateLimit,
        Some(404) => return ErrorKind::ModelNotFound,
        Some(500..=599) => return ErrorKind::Server,
        _ => {}
    }
    let b = body.to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| b.contains(w));
    if has(&["context length", "context_length", "context window", "maximum context", "exceeds the available context", "prompt is too long", "too many tokens", "too long"]) {
        ErrorKind::ContextTooLong
    } else if has(&["model_not_found", "not_found_error"]) || (b.contains("model") && has(&["not found", "does not exist", "no such model", "unknown model"])) {
        ErrorKind::ModelNotFound
    } else if has(&["api key not valid", "invalid api key", "invalid_api_key", "incorrect api key", "authentication_error", "unauthorized", "permission_denied"]) {
        ErrorKind::Auth
    } else if has(&["rate_limit", "rate limit", "resource_exhausted", "quota"]) {
        ErrorKind::RateLimit
    } else if has(&["overloaded", "server_error", "internal error", "unavailable"]) {
        ErrorKind::Server
    } else {
        ErrorKind::Unknown
    }
}

/// Server-sent events: bytes in, (event name, data) out. Only whole lines are decoded,
/// so a chunk may end anywhere, even inside a character.
#[derive(Default)]
pub struct Sse {
    buf: Vec<u8>,
    name: String,
    data: Vec<String>,
}

impl Sse {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<(String, String)> {
        self.buf.extend_from_slice(bytes);
        let mut events = Vec::new();
        while let Some(end) = self.buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=end).collect();
            let line = String::from_utf8_lossy(&line);
            self.line(line.trim_end_matches(['\n', '\r']), &mut events);
        }
        events
    }

    /// The stream closed: a last event without its blank line still counts.
    pub fn finish(&mut self) -> Vec<(String, String)> {
        let mut events = self.push(b"\n");
        self.line("", &mut events);
        events
    }

    fn line(&mut self, line: &str, events: &mut Vec<(String, String)>) {
        if line.is_empty() {
            if !self.data.is_empty() {
                events.push((std::mem::take(&mut self.name), std::mem::take(&mut self.data).join("\n")));
            }
            self.name.clear();
        } else if let Some(v) = line.strip_prefix("data:") {
            self.data.push(v.strip_prefix(' ').unwrap_or(v).to_string());
        } else if let Some(v) = line.strip_prefix("event:") {
            self.name = v.trim().to_string();
        } // comments (": keep-alive"), id: and retry: are ignored
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;

    pub enum Step {
        Write(&'static str),
        Sleep(u64),
    }

    pub const SSE_HEAD: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";

    /// A one-file HTTP server on 127.0.0.1: answers every connection with `steps`. Returns its port and how many connections it accepted.
    pub fn serve(steps: Vec<Step>) -> (u16, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let count = Arc::new(AtomicUsize::new(0));
        let seen = count.clone();
        let steps = Arc::new(steps);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                seen.fetch_add(1, Ordering::SeqCst);
                let steps = steps.clone();
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut len = 0;
                    loop {
                        let mut line = String::new();
                        if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                            break;
                        }
                        if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                            len = v.trim().parse().unwrap_or(0);
                        }
                    }
                    let _ = reader.read_exact(&mut vec![0; len]);
                    let mut out = stream;
                    for step in steps.iter() {
                        match step {
                            Step::Write(s) => {
                                if out.write_all(s.as_bytes()).and_then(|_| out.flush()).is_err() {
                                    return;
                                }
                            }
                            Step::Sleep(ms) => std::thread::sleep(Duration::from_millis(*ms)),
                        }
                    }
                });
            }
        });
        (port, count)
    }

    pub fn post(port: u16) -> Request {
        Request { url: Url::parse(&format!("http://127.0.0.1:{port}/v1/chat/completions")).unwrap(), headers: vec![], body: Some(serde_json::json!({})) }
    }

    /// 1-second limits, for the tests that wait for a timeout; the others use the defaults, so a busy machine can't fail them.
    fn fast() -> Policy {
        Policy { timeouts: Timeouts { connect_secs: 1, first_reply_local_secs: 1, first_reply_cloud_secs: 1, pause_secs: 1 }, ..Policy::default() }
    }

    fn block<T>(f: impl Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(f)
    }

    fn collect(req: Request, policy: &Policy) -> (Vec<String>, Result<(), ChatError>) {
        let mut got = Vec::new();
        let r = block(stream(req, policy, &AtomicBool::new(false), &mut |_, d| {
            got.push(d.to_string());
            Ok(Flow::Continue)
        }));
        (got, r)
    }

    #[test]
    fn llm_1_2_sse_framing_survives_any_chunking() {
        let body = "event: a\r\ndata: {\"t\":\"héllo\"}\r\n\r\n: keep-alive\n\ndata: line1\ndata: line2\n\ndata: last\n";
        let expected = vec![("a".to_string(), "{\"t\":\"héllo\"}".to_string()), (String::new(), "line1\nline2".into()), (String::new(), "last".into())];
        for size in [1, 2, 3, 5, 7, body.len()] {
            let mut sse = Sse::default();
            let mut got: Vec<_> = body.as_bytes().chunks(size).flat_map(|c| sse.push(c)).collect();
            got.extend(sse.finish());
            assert_eq!(got, expected, "chunk size {size}");
        }
    }

    #[test]
    fn llm_1_3_cancel_returns_within_100_ms_and_passes_nothing_on() {
        let mut steps = vec![Step::Write(SSE_HEAD)];
        for _ in 0..500 {
            steps.push(Step::Write("data: {\"x\":1}\n\n"));
            steps.push(Step::Sleep(10));
        }
        let (port, _) = serve(steps);
        let cancel = AtomicBool::new(false);
        let (mut events, mut at) = (0, None);
        let r = block(stream(post(port), &Policy::default(), &cancel, &mut |_, _| {
            events += 1;
            if events == 3 {
                cancel.store(true, Ordering::SeqCst);
                at = Some(std::time::Instant::now());
            }
            Ok(Flow::Continue)
        }));
        let took = at.unwrap().elapsed();
        assert!(r.is_ok());
        assert_eq!(events, 3, "an event was passed on after cancel");
        assert!(took < Duration::from_millis(100), "took {took:?}");
    }

    #[test]
    fn llm_1_3_cancel_works_while_waiting_for_the_first_reply() {
        let (port, connected) = serve(vec![Step::Sleep(5000)]);
        let cancel = Arc::new(AtomicBool::new(false));
        let at = Arc::new(std::sync::Mutex::new(None));
        let (c, a) = (cancel.clone(), at.clone());
        std::thread::spawn(move || {
            while connected.load(Ordering::SeqCst) == 0 {
                std::thread::sleep(Duration::from_millis(5));
            }
            std::thread::sleep(Duration::from_millis(50)); // the request is out; the server never answers
            *a.lock().unwrap() = Some(std::time::Instant::now());
            c.store(true, Ordering::SeqCst);
        });
        assert!(block(stream(post(port), &Policy::default(), &cancel, &mut |_, _| Ok(Flow::Continue))).is_ok());
        let took = at.lock().unwrap().expect("returned before cancel").elapsed();
        assert!(took < Duration::from_millis(100), "took {took:?}");
    }

    #[test]
    fn llm_1_4_errors_are_classified_from_status_and_body() {
        use ErrorKind::*;
        let cases: &[(Option<u16>, &str, ErrorKind)] = &[
            (Some(401), r#"{"error":{"message":"Incorrect API key provided: sk-ab***","type":"invalid_request_error","code":"invalid_api_key"}}"#, Auth),
            (Some(429), r#"{"error":{"message":"You exceeded your current quota","type":"insufficient_quota"}}"#, RateLimit),
            (Some(404), r#"{"error":{"message":"The model `gpt-9` does not exist or you do not have access to it.","code":"model_not_found"}}"#, ModelNotFound),
            (Some(400), r#"{"error":{"message":"This model's maximum context length is 128000 tokens. However, your messages resulted in 130000 tokens.","code":"context_length_exceeded"}}"#, ContextTooLong),
            (Some(401), r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#, Auth),
            (Some(400), r#"{"type":"error","error":{"type":"invalid_request_error","message":"prompt is too long: 210000 tokens > 200000 maximum"}}"#, ContextTooLong),
            (Some(529), r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#, Server),
            (Some(404), r#"{"type":"error","error":{"type":"not_found_error","message":"model: claude-9"}}"#, ModelNotFound),
            (Some(400), r#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","status":"INVALID_ARGUMENT"}}"#, Auth),
            (Some(429), r#"{"error":{"code":429,"message":"Resource has been exhausted","status":"RESOURCE_EXHAUSTED"}}"#, RateLimit),
            // Recorded from the bundled llama-server (b11570).
            (Some(400), r#"{"error":{"code":400,"message":"request (3030 tokens) exceeds the available context size (2048 tokens), try increasing it","type":"exceed_context_size_error","n_prompt_tokens":3030,"n_ctx":2048}}"#, ContextTooLong),
            (Some(503), "Service Unavailable", Server),
            (None, r#"{"type":"overloaded_error","message":"Overloaded"}"#, Server),
            (None, r#"{"error":{"message":"Rate limit exceeded: free-models-per-day"}}"#, RateLimit),
            (Some(400), r#"{"error":{"message":"Invalid value for 'temperature'"}}"#, Unknown),
            (Some(302), "", Unknown),
        ];
        for (status, body, kind) in cases {
            assert_eq!(classify(*status, body), *kind, "{status:?} {body}");
        }
        // A stream error carrying its own HTTP code, and the provider's message is kept.
        let e = http_error(None, r#"{"error":{"code":502,"message":"Provider returned error"}}"#);
        assert_eq!((e.kind, e.message.as_str()), (Server, "Provider returned error"));
    }

    #[test]
    fn llm_1_5_timeouts_have_the_required_defaults_and_end_a_silent_stream() {
        let t = Timeouts::default();
        assert_eq!((t.connect_secs, t.first_reply_local_secs, t.first_reply_cloud_secs, t.pause_secs), (10, 120, 60, 60));
        // No reply at all.
        let (port, _) = serve(vec![Step::Sleep(3000)]);
        let (_, r) = collect(post(port), &fast());
        let e = r.unwrap_err();
        assert_eq!(e.kind, ErrorKind::Network);
        assert!(e.message.contains("didn't reply within 1 s"), "{}", e.message);
        // A pause mid-stream, after one event.
        let (port, _) = serve(vec![Step::Write(SSE_HEAD), Step::Write("data: one\n\n"), Step::Sleep(3000)]);
        let (got, r) = collect(post(port), &fast());
        assert_eq!(got, ["one"]);
        assert_eq!(r.unwrap_err().kind, ErrorKind::Network);
    }

    #[test]
    fn pri_1_1_local_only_refuses_cloud_hosts_before_connecting() {
        let local_only = Policy { local_only: true, ..Policy::default() };
        let cloud = Request { url: Url::parse("https://api.openai.com/v1/chat/completions").unwrap(), headers: vec![], body: None };
        assert_eq!(collect(cloud, &local_only).1.unwrap_err().kind, ErrorKind::BlockedLocalOnly);
        let lan = Url::parse("http://192.168.1.20:1234/v1/models").unwrap();
        assert_eq!(guard(&lan, &local_only).unwrap_err().kind, ErrorKind::BlockedLocalOnly);
        assert_eq!(guard(&lan, &Policy { trusted: vec!["192.168.1.20".into()], ..local_only.clone() }).unwrap(), Locality::Lan);
        // A local server passes, but can't redirect the request elsewhere.
        let (port, count) = serve(vec![Step::Write("HTTP/1.1 302 Found\r\nLocation: https://example.com/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")]);
        let e = collect(post(port), &local_only).1.unwrap_err();
        assert!(e.message.contains("redirect"), "{}", e.message);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}

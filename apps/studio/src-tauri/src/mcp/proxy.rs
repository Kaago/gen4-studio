//! `studio mcp` without `--project`: forwards the client's stdio to the running app. Each
//! newline-delimited JSON-RPC message from stdin goes as one `POST` to the URL of the
//! discovery file (with its token, and the session once `initialize` opened one); the
//! answer, JSON or SSE, goes back to stdout one message per line. When stdin ends, the
//! session is closed with `DELETE`. When the app ended the session ("Trennen", idle) or
//! moved (switched off and on, started anew: new port and token), the proxy reads the
//! discovery file again, repeats the client's handshake and sends the message once more. Access is the app's to decide (grant, read-only switch,
//! questions); the proxy adds nothing.
//!
//! Messages are forwarded one at a time, in order; one TCP connection per request.

use std::path::{Path, PathBuf};
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HOST};
use hyper::{HeaderMap, Method, Request, StatusCode, Uri};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};

use crate::mcp::discovery::{self, DISCOVERY_VERSION, Discovery};

/// What `studio mcp` says when no running app serves MCP (exit code 2).
pub const NOT_RUNNING: &str = "Gen4 Studio läuft nicht oder „Lokale Agenten zulassen“ ist aus; \
                               kopflos mit --project <Ordner>";

const SESSION_HEADER: &str = "mcp-session-id";
const VERSION_HEADER: &str = "mcp-protocol-version";
/// How long connecting to the app may take (it runs on this machine).
const CONNECT_WAIT: Duration = Duration::from_secs(5);
/// How long the closing `DELETE` may take.
const DELETE_WAIT: Duration = Duration::from_secs(5);
/// JSON-RPC "internal error": the app could not be reached or refused the request.
const INTERNAL_ERROR: i64 = -32603;
/// How much of a refusal's text goes into the error message.
const MESSAGE_LIMIT: usize = 300;

/// The discovery file in `dir`, if the app that wrote it still runs; otherwise
/// [`NOT_RUNNING`] (or a note that app and `studio mcp` do not match).
pub fn locate(dir: &Path) -> Result<Discovery, String> {
    let found = discovery::read(dir).map_err(|_| NOT_RUNNING.to_owned())?;
    if !alive(found.pid) {
        return Err(NOT_RUNNING.to_owned());
    }
    if found.version != DISCOVERY_VERSION {
        return Err(format!(
            "{}: Version {}, erwartet {DISCOVERY_VERSION} (App und studio mcp passen nicht \
             zusammen)",
            dir.join(discovery::DISCOVERY_FILE).display(),
            found.version
        ));
    }
    Ok(found)
}

/// Whether the process `pid` runs (a process of another user counts as running).
#[cfg(unix)]
fn alive(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    if pid <= 0 {
        return false;
    }
    // SAFETY: signal 0 sends nothing, it only checks that the process exists.
    if unsafe { libc::kill(pid, 0) } == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Elsewhere the file is trusted; a dead app shows as a refused connection.
#[cfg(not(unix))]
fn alive(_pid: u32) -> bool {
    true
}

/// Where the app serves MCP, and with which token.
struct Target {
    /// Host and port to connect to.
    host: String,
    port: u16,
    /// The `Host` header (`127.0.0.1:<port>`).
    authority: String,
    path: String,
    token: String,
}

impl Target {
    /// The target of `discovery`: an `http://` URL on a loopback host only (the token must
    /// not leave the machine).
    fn of(discovery: &Discovery) -> Result<Target, String> {
        let invalid = || format!("Ungültige URL in mcp.json: {}", discovery.url);
        let uri: Uri = discovery.url.parse().map_err(|_| invalid())?;
        if uri.scheme_str() != Some("http") {
            return Err(invalid());
        }
        let authority = uri.authority().ok_or_else(invalid)?;
        let host = authority
            .host()
            .trim_start_matches('[')
            .trim_end_matches(']');
        if !["127.0.0.1", "localhost", "::1"].contains(&host) {
            return Err(format!(
                "mcp.json nennt keinen lokalen Server: {}",
                discovery.url
            ));
        }
        let port = authority.port_u16().ok_or_else(invalid)?;
        Ok(Target {
            host: host.to_owned(),
            port,
            authority: authority.as_str().to_owned(),
            path: uri.path().to_owned(),
            token: discovery.token.clone(),
        })
    }

    /// One request (on a connection of its own); the status, headers and whole body.
    async fn send(
        &self,
        method: Method,
        session: &Session,
        body: Bytes,
    ) -> Result<Answer, Failure> {
        let stream = tokio::time::timeout(
            CONNECT_WAIT,
            tokio::net::TcpStream::connect((self.host.as_str(), self.port)),
        )
        .await
        .map_err(|_| Failure::Unreachable("nicht erreichbar (Zeitüberschreitung)".to_owned()))?
        .map_err(|e| Failure::Unreachable(format!("nicht erreichbar: {e}")))?;
        let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
            .await
            .map_err(|e| Failure::Unreachable(format!("Verbindung fehlgeschlagen: {e}")))?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        let mut request = Request::builder()
            .method(method)
            .uri(self.path.as_str())
            .header(HOST, self.authority.as_str())
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json, text/event-stream");
        if let Some(id) = &session.id {
            request = request.header(SESSION_HEADER, id.as_str());
        }
        if let Some(version) = &session.protocol {
            request = request.header(VERSION_HEADER, version.as_str());
        }
        let request = request
            .body(Full::new(body))
            .map_err(|e| Failure::Broken(format!("Anfrage ungültig: {e}")))?;
        let response = sender
            .send_request(request)
            .await
            .map_err(|e| Failure::Broken(format!("Anfrage fehlgeschlagen: {e}")))?;
        let (parts, body) = response.into_parts();
        let body = body
            .collect()
            .await
            .map_err(|e| Failure::Broken(format!("Antwort abgebrochen: {e}")))?
            .to_bytes();
        Ok(Answer {
            status: parts.status,
            headers: parts.headers,
            body,
        })
    }
}

/// The app's answer to one request.
struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

/// Why a request got no answer.
enum Failure {
    /// No connection: nothing reached the app (safe to send again).
    Unreachable(String),
    /// The request may have reached the app (never sent again).
    Broken(String),
}

impl Failure {
    fn text(&self) -> &str {
        match self {
            Failure::Unreachable(text) | Failure::Broken(text) => text,
        }
    }
}

/// The session the app opened for this client, and the protocol version they agreed on.
#[derive(Default)]
struct Session {
    id: Option<String>,
    protocol: Option<String>,
}

/// The second message of the handshake, sent after a renewed `initialize`.
const INITIALIZED: &[u8] = br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

/// The proxy's state: where the app serves (read again from `dir` when it moved), the
/// session, and the client's last `initialize` (to open a new session with it).
struct Proxy {
    dir: PathBuf,
    target: Target,
    session: Session,
    handshake: Option<Bytes>,
}

/// Forwards `input` to the app of `discovery` (found in the app data folder `dir`) and its
/// answers to `output` until `input` ends; then closes the session.
pub async fn run<I, O>(
    dir: PathBuf,
    discovery: Discovery,
    input: I,
    mut output: O,
) -> Result<(), String>
where
    I: AsyncRead + Unpin,
    O: AsyncWrite + Unpin,
{
    let mut proxy = Proxy {
        dir,
        target: Target::of(&discovery)?,
        session: Session::default(),
        handshake: None,
    };
    let mut input = BufReader::new(input);
    let mut line = Vec::new();
    let mut result = Ok(());
    loop {
        line.clear();
        match input.read_until(b'\n', &mut line).await {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                result = Err(format!("stdin nicht lesbar: {e}"));
                break;
            }
        }
        let message = line.trim_ascii();
        if message.is_empty() {
            continue;
        }
        let replies = proxy.forward(message).await;
        if let Err(e) = write_lines(&mut output, &replies).await {
            // The client is gone.
            result = Err(format!("stdout nicht schreibbar: {e}"));
            break;
        }
    }
    proxy.close_session().await;
    result
}

async fn write_lines<O: AsyncWrite + Unpin>(
    output: &mut O,
    lines: &[String],
) -> std::io::Result<()> {
    if lines.is_empty() {
        return Ok(());
    }
    for line in lines {
        output.write_all(line.as_bytes()).await?;
        output.write_all(b"\n").await?;
    }
    output.flush().await
}

/// Whether a request is worth sending again after the proxy found the app anew: nothing
/// reached the app, the token is old (401), or the app ended the session (404).
fn renewable(sent: &Result<Answer, Failure>, had_session: bool) -> bool {
    match sent {
        Err(Failure::Unreachable(_)) => true,
        Err(Failure::Broken(_)) => false,
        Ok(answer) => {
            answer.status == StatusCode::UNAUTHORIZED
                || (answer.status == StatusCode::NOT_FOUND && had_session)
        }
    }
}

impl Proxy {
    /// Ends the session with `DELETE` (at most [`DELETE_WAIT`]); the app may have ended it
    /// already, nothing to tell anyone either way.
    async fn close_session(&mut self) {
        if self.session.id.is_some() {
            let _ = tokio::time::timeout(
                DELETE_WAIT,
                self.target
                    .send(Method::DELETE, &self.session, Bytes::new()),
            )
            .await;
        }
        self.session = Session::default();
    }

    /// Reads the discovery file again (the switch went off and on, the app started anew:
    /// new port and token); false when no app runs.
    fn relocate(&mut self) -> bool {
        match locate(&self.dir).and_then(|found| Target::of(&found)) {
            Ok(target) => {
                self.target = target;
                true
            }
            Err(_) => false,
        }
    }

    /// Takes the session id and protocol version of a successful `initialize` answer.
    fn adopt(&mut self, answer: &Answer, lines: &[String]) {
        self.session.id = answer
            .headers
            .get(SESSION_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        self.session.protocol = lines.iter().find_map(|line| {
            let answer: Value = serde_json::from_str(line).ok()?;
            answer["result"]["protocolVersion"]
                .as_str()
                .map(str::to_owned)
        });
    }

    /// Finds the app again and opens a new session with the client's last `initialize` (its
    /// answer is not passed on, the client has had one) and `notifications/initialized`.
    async fn renew(&mut self) -> bool {
        let Some(handshake) = self.handshake.clone() else {
            return false;
        };
        if !self.relocate() {
            return false;
        }
        self.session = Session::default();
        match self
            .target
            .send(Method::POST, &self.session, handshake)
            .await
        {
            Ok(answer) if answer.status.is_success() => {
                let lines = answer_lines(&answer.headers, &answer.body);
                self.adopt(&answer, &lines);
            }
            _ => return false,
        }
        let _ = self
            .target
            .send(Method::POST, &self.session, Bytes::from_static(INITIALIZED))
            .await;
        true
    }

    /// Sends one message to the app; the lines for stdout (the answer, or an error answer
    /// for a request the app could not take). When the app moved or ended the session, the
    /// proxy finds it again and sends the message once more.
    async fn forward(&mut self, message: &[u8]) -> Vec<String> {
        let parsed: Option<Value> = serde_json::from_slice(message).ok();
        let method = parsed
            .as_ref()
            .and_then(|m| m.get("method"))
            .and_then(Value::as_str);
        // Only a request gets an error answer (not a notification, not a client's answer).
        let id = parsed
            .as_ref()
            .filter(|_| method.is_some())
            .and_then(|m| m.get("id"))
            .filter(|id| !id.is_null())
            .cloned();
        let initialize = method == Some("initialize");
        let body = Bytes::copy_from_slice(message);
        if initialize {
            // A new handshake opens a new session: the old one ends first.
            self.close_session().await;
            self.handshake = Some(body.clone());
        }
        let mut had_session = self.session.id.is_some();
        let mut sent = self
            .target
            .send(Method::POST, &self.session, body.clone())
            .await;
        if renewable(&sent, had_session) {
            let renewed = if initialize {
                self.relocate()
            } else {
                self.renew().await
            };
            if renewed {
                had_session = self.session.id.is_some();
                sent = self.target.send(Method::POST, &self.session, body).await;
            }
        }
        let answer = match sent {
            Ok(answer) => answer,
            Err(e) => return error_answer(id, &format!("Gen4 Studio: {}", e.text())),
        };
        if answer.status.is_success() {
            let lines = answer_lines(&answer.headers, &answer.body);
            if initialize {
                self.adopt(&answer, &lines);
            }
            return lines;
        }
        let text = if answer.status == StatusCode::NOT_FOUND && had_session {
            // The app closed the session and no new one could be opened.
            self.session = Session::default();
            "Sitzung beendet".to_owned()
        } else {
            refusal_text(answer.status, &answer.body)
        };
        error_answer(
            id,
            &format!("Gen4 Studio: HTTP {}: {text}", answer.status.as_u16()),
        )
    }
}

/// The JSON-RPC error answer to the request `id` (none for a notification; the reason then
/// goes to stderr only).
fn error_answer(id: Option<Value>, message: &str) -> Vec<String> {
    let Some(id) = id else {
        eprintln!("studio mcp: {message}");
        return Vec::new();
    };
    let answer = json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": INTERNAL_ERROR, "message": message }
    });
    vec![answer.to_string()]
}

/// The message of a refusal: the `message` of our JSON refusal or of a JSON-RPC error,
/// otherwise the body's text (shortened), otherwise the status's name.
fn refusal_text(status: StatusCode, body: &[u8]) -> String {
    if let Ok(value) = serde_json::from_slice::<Value>(body) {
        let message = value
            .get("message")
            .or_else(|| value.get("error").and_then(|e| e.get("message")))
            .and_then(Value::as_str);
        if let Some(message) = message {
            return message.to_owned();
        }
    }
    let text = String::from_utf8_lossy(body);
    let text = text.trim();
    if text.is_empty() {
        return status.canonical_reason().unwrap_or("Fehler").to_owned();
    }
    let mut short: String = text.chars().take(MESSAGE_LIMIT).collect();
    if short.len() < text.len() {
        short.push('…');
    }
    one_line(&short)
}

/// The messages of a successful answer: none (202, empty), the JSON body, or every `data:`
/// event of an SSE stream; one line each.
fn answer_lines(headers: &HeaderMap, body: &[u8]) -> Vec<String> {
    if body.iter().all(u8::is_ascii_whitespace) {
        return Vec::new();
    }
    let kind = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let text = String::from_utf8_lossy(body);
    if kind.starts_with("text/event-stream") {
        return sse_events(&text);
    }
    vec![one_line(&text)]
}

/// The data of each event of an SSE stream (several `data:` lines of one event joined), one
/// line each; events without data (priming, comments) are skipped.
fn sse_events(text: &str) -> Vec<String> {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    text.split("\n\n")
        .filter_map(|event| {
            let data: Vec<&str> = event
                .lines()
                .filter_map(|line| line.strip_prefix("data:"))
                .map(|data| data.strip_prefix(' ').unwrap_or(data))
                .collect();
            let data = data.join("\n");
            (!data.trim().is_empty()).then(|| one_line(&data))
        })
        .collect()
}

/// `text` as one line: JSON spread over lines is written compactly, anything else gets
/// spaces for its line breaks.
fn one_line(text: &str) -> String {
    let text = text.trim();
    if !text.contains(['\n', '\r']) {
        return text.to_owned();
    }
    match serde_json::from_str::<Value>(text) {
        Ok(value) => value.to_string(),
        Err(_) => text.replace(['\r', '\n'], " "),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn sse_events_one_line_each() {
        let stream = "data: {\"id\":1}\n\n: comment\n\nid: 7\ndata:\n\nevent: message\r\ndata: \
                      {\"id\":\r\ndata: 2}\r\n\r\n";
        assert_eq!(sse_events(stream), vec!["{\"id\":1}", "{\"id\":2}"]);
    }

    #[test]
    fn json_body_is_one_line() {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        assert_eq!(
            answer_lines(&headers, b"{\n  \"id\": 3\n}\n"),
            vec!["{\"id\":3}"]
        );
        assert!(answer_lines(&headers, b"").is_empty());
    }

    #[test]
    fn refusal_text_prefers_the_message() {
        let refusal = br#"{"code":"error","message":"Token fehlt oder ist falsch","asset":null,"problems":[]}"#;
        assert_eq!(
            refusal_text(StatusCode::UNAUTHORIZED, refusal),
            "Token fehlt oder ist falsch"
        );
        assert_eq!(
            refusal_text(StatusCode::BAD_REQUEST, b"Bad Request: no\nway"),
            "Bad Request: no way"
        );
        assert_eq!(refusal_text(StatusCode::NOT_FOUND, b""), "Not Found");
    }

    #[test]
    fn error_answer_only_for_requests() {
        assert!(error_answer(None, "x").is_empty());
        let answer: Value = serde_json::from_str(&error_answer(Some(json!(4)), "x")[0]).unwrap();
        assert_eq!(
            answer,
            json!({ "jsonrpc": "2.0", "id": 4, "error": { "code": -32603, "message": "x" } })
        );
    }

    #[test]
    fn target_is_loopback_only() {
        let mut found = Discovery::new(4711, "c0ffee");
        let target = Target::of(&found).unwrap();
        assert_eq!(
            (target.host.as_str(), target.port, target.path.as_str()),
            ("127.0.0.1", 4711, "/mcp")
        );
        assert_eq!(target.authority, "127.0.0.1:4711");
        found.url = "http://example.com:4711/mcp".to_owned();
        assert!(Target::of(&found).is_err());
        found.url = "https://127.0.0.1:4711/mcp".to_owned();
        assert!(Target::of(&found).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn this_process_is_alive_a_far_pid_is_not() {
        assert!(alive(std::process::id()));
        assert!(!alive(999_999));
        assert!(!alive(0));
        assert!(!alive(u32::MAX));
    }

    #[test]
    fn locate_needs_a_living_app() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(locate(dir.path()).unwrap_err(), NOT_RUNNING);
        let ours = Discovery::new(4711, "c0ffee");
        discovery::write(dir.path(), &ours).unwrap();
        assert_eq!(locate(dir.path()).unwrap(), ours);
        let mut newer = ours.clone();
        newer.version = DISCOVERY_VERSION + 1;
        discovery::write(dir.path(), &newer).unwrap();
        assert!(locate(dir.path()).unwrap_err().contains("Version"));
    }
}

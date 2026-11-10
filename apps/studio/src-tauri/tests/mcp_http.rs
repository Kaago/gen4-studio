//! The app's MCP server over Streamable HTTP, in the test process: `HttpServer::start` on a
//! synthetic project opened in an `AppState` with the grant `write`, raw HTTP requests.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::{Duration, Instant};

use g4_kernel::test_support::{Synthetic, synthetic_project};
use serde_json::{Value, json};
use studio_lib::AppState;
use studio_lib::dispatch::dispatch;
use studio_lib::mcp::discovery::{self, DISCOVERY_FILE};
use studio_lib::mcp::host::StateSource;
use studio_lib::mcp::http::HttpServer;
use studio_lib::mcp::settings::{GrantLevel, ProjectKey};
use studio_lib::policy::UserPolicy;

/// How long a test waits for the server to let a closed session go.
const LIMIT: Duration = Duration::from_secs(10);

struct App {
    /// Keeps the project folder while the test runs.
    _fixture: Synthetic,
    state: Arc<AppState>,
    data: tempfile::TempDir,
    runtime: tokio::runtime::Runtime,
}

impl App {
    /// The synthetic project open in a fresh state, with the grant `write` for it and
    /// „Lokale Agenten zulassen“ on (the switch only: the test starts the server itself).
    fn new() -> App {
        let fixture = synthetic_project();
        let state = Arc::new(AppState::default());
        let dir = fixture.dir.path();
        let policy = UserPolicy::new(dir, &[], None).unwrap();
        dispatch(
            &state,
            &policy,
            "open_project",
            json!({ "path": dir.to_str().unwrap() }),
        )
        .unwrap();
        let key = ProjectKey::of(dir).unwrap();
        state.agents.set_grant(key, GrantLevel::Write).unwrap();
        state.agents.set_enabled(true).unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        App {
            _fixture: fixture,
            state,
            data: tempfile::tempdir().unwrap(),
            runtime,
        }
    }

    fn start(&self) -> HttpServer {
        let source: Arc<dyn StateSource> = Arc::new(Arc::clone(&self.state));
        HttpServer::start(source, self.data.path(), self.runtime.handle()).unwrap()
    }

    fn token(&self) -> String {
        discovery::read(self.data.path()).unwrap().token
    }

    fn connections(&self) -> usize {
        self.state.agents.status().connections.len()
    }
}

struct Reply {
    status: u16,
    head: String,
    body: Vec<u8>,
}

impl Reply {
    fn header(&self, name: &str) -> Option<String> {
        self.head.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name)
                .then(|| value.trim().to_owned())
        })
    }

    /// The JSON-RPC message of the answer: the body as JSON, or the last `data:` event of
    /// an SSE stream.
    fn message(&self) -> Value {
        let text = String::from_utf8_lossy(&self.body);
        if self
            .header("content-type")
            .is_some_and(|t| t.starts_with("text/event-stream"))
        {
            let data = text
                .lines()
                .filter_map(|line| line.strip_prefix("data:"))
                .map(str::trim)
                .rfind(|data| !data.is_empty())
                .unwrap_or_else(|| panic!("no data event in {text}"));
            serde_json::from_str(data).unwrap()
        } else {
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text}"))
        }
    }
}

/// One HTTP/1.1 request with exactly these headers (and `Content-Length`), the whole answer.
fn request(port: u16, method: &str, headers: &[(&str, String)], body: &str) -> Reply {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .unwrap();
    let mut head = format!("{method} /mcp HTTP/1.1\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    stream.write_all(head.as_bytes()).unwrap();
    stream.write_all(body.as_bytes()).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or_else(|| panic!("no head in {}", String::from_utf8_lossy(&raw)));
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    Reply {
        status,
        head,
        body: raw[split + 4..].to_vec(),
    }
}

/// The headers of a client with `token` (and the session, once it has one).
fn headers(port: u16, token: &str, session: Option<&str>) -> Vec<(&'static str, String)> {
    let mut headers = vec![
        ("Host", format!("127.0.0.1:{port}")),
        ("Authorization", format!("Bearer {token}")),
        ("Content-Type", "application/json".to_owned()),
        ("Accept", "application/json, text/event-stream".to_owned()),
    ];
    if let Some(session) = session {
        headers.push(("Mcp-Session-Id", session.to_owned()));
        headers.push(("MCP-Protocol-Version", "2025-06-18".to_owned()));
    }
    headers
}

fn post(port: u16, token: &str, session: Option<&str>, message: &Value) -> Reply {
    request(
        port,
        "POST",
        &headers(port, token, session),
        &message.to_string(),
    )
}

/// `initialize` and `notifications/initialized` as Claude Code sends them; the session id.
fn initialize(port: u16, token: &str) -> String {
    let reply = post(
        port,
        token,
        None,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "claude-code", "version": "2.0.0" }
            }
        }),
    );
    assert_eq!(
        reply.status,
        200,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    let session = reply.header("mcp-session-id").expect("Mcp-Session-Id");
    let message = reply.message();
    assert_eq!(message["id"], 1);
    assert_eq!(message["result"]["protocolVersion"], "2025-06-18");
    let reply = post(
        port,
        token,
        Some(&session),
        &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
    );
    assert_eq!(
        reply.status,
        202,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    session
}

fn call_get_header(port: u16, token: &str, session: &str, id: u64) -> Reply {
    post(
        port,
        token,
        Some(session),
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": "get_header", "arguments": { "id": 0 } }
        }),
    )
}

fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let start = Instant::now();
    while !done() {
        assert!(start.elapsed() < LIMIT, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn initialize_then_call_over_http() {
    let app = App::new();
    let server = app.start();
    let token = app.token();
    let session = initialize(server.port, &token);
    // initialize reported the client to the hub.
    let connections = app.state.agents.status().connections;
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].client, "Claude Code");

    let reply = call_get_header(server.port, &token, &session, 2);
    assert_eq!(reply.status, 200);
    let message = reply.message();
    assert_eq!(message["id"], 2);
    let result = &message["result"];
    assert_ne!(result["isError"], true, "{result}");
    assert!(
        result["structuredContent"]["result"].is_object(),
        "{result}"
    );
    server.stop();
}

#[test]
fn discovery_file_is_0600_and_removed_on_stop() {
    let app = App::new();
    let server = app.start();
    let path = app.data.path().join(DISCOVERY_FILE);
    let found = discovery::read(app.data.path()).unwrap();
    assert_eq!(found.version, 1);
    assert_eq!(found.port, server.port);
    assert_eq!(found.url, format!("http://127.0.0.1:{}/mcp", server.port));
    assert_eq!(found.pid, std::process::id());
    assert_eq!(found.token.len(), 64);
    assert!(found.token.chars().all(|c| c.is_ascii_hexdigit()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    let port = server.port;
    server.stop();
    assert!(!path.exists());
    // Nothing answers on the port any more (or, if something else took it, not us).
    if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
        let _ = stream.write_all(b"POST /mcp HTTP/1.1\r\nHost: x\r\n\r\n");
        let mut raw = Vec::new();
        let _ = stream.read_to_end(&mut raw);
        assert!(!String::from_utf8_lossy(&raw).contains("Host not allowed"));
    }
    // A second start gets a new token.
    let again = app.start();
    assert_ne!(app.token(), found.token);
    again.stop();
}

#[test]
fn bad_token_is_401() {
    let app = App::new();
    let server = app.start();
    let token = app.token();
    let mut wrong = token.clone();
    wrong.replace_range(0..1, if token.starts_with('0') { "1" } else { "0" });
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "probe", "version": "1" }
        }
    });
    let reply = post(server.port, &wrong, None, &init);
    assert_eq!(reply.status, 401);
    assert_eq!(
        reply.message(),
        json!({
            "code": "error",
            "message": "Token is missing or wrong",
            "asset": null,
            "problems": []
        })
    );
    let without: Vec<_> = headers(server.port, &token, None)
        .into_iter()
        .filter(|(name, _)| *name != "Authorization")
        .collect();
    let reply = request(server.port, "POST", &without, &init.to_string());
    assert_eq!(reply.status, 401);
    // A browser page: the origin is refused before the token is looked at.
    let mut browser = without.clone();
    browser.push(("Origin", "http://evil".to_owned()));
    let reply = request(server.port, "POST", &browser, &init.to_string());
    assert_eq!(reply.status, 403);
    assert_eq!(reply.message()["message"], "Origin not allowed");
    // GET opens no server stream.
    let reply = request(server.port, "GET", &headers(server.port, &token, None), "");
    assert_eq!(reply.status, 405);
    // No session was made.
    assert_eq!(app.connections(), 0);
    server.stop();
}

#[test]
fn session_survives_and_delete_ends_it() {
    let app = App::new();
    let server = app.start();
    let token = app.token();
    let session = initialize(server.port, &token);
    // The session carries over several requests (each on its own TCP connection).
    for id in 2..5 {
        let reply = call_get_header(server.port, &token, &session, id);
        assert_eq!(reply.status, 200);
        assert_eq!(reply.message()["id"], id);
    }
    assert_eq!(app.connections(), 1);
    let reply = request(
        server.port,
        "DELETE",
        &headers(server.port, &token, Some(&session)),
        "",
    );
    assert!(reply.status < 300, "DELETE: {}", reply.status);
    // The session's server is dropped: the hub counts no connection.
    wait_until("the connection to end", || app.connections() == 0);
    let reply = call_get_header(server.port, &token, &session, 9);
    assert_eq!(reply.status, 404);
    server.stop();
}

#[test]
fn disconnect_closes_every_session() {
    let app = App::new();
    let server = app.start();
    let token = app.token();
    let first = initialize(server.port, &token);
    let second = initialize(server.port, &token);
    assert_eq!(app.connections(), 2);
    // "Trennen" in the app.
    app.state.agents.disconnect_all();
    for session in [&first, &second] {
        wait_until("the session to close", || {
            call_get_header(server.port, &token, session, 7).status == 404
        });
    }
    assert_eq!(app.connections(), 0);
    // A new session works (and asks again: the client was disconnected, so its call waits
    // for an answer; only initialize is checked here).
    initialize(server.port, &token);
    assert_eq!(app.connections(), 1);
    server.stop();
    // Stopping closed that session too.
    wait_until("the connection to end", || app.connections() == 0);
}

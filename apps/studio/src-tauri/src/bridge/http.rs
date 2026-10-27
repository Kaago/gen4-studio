//! The bridge's HTTP side: `GET /health`, `POST /invoke/<command>` and MCP on `/mcp`, all
//! on loopback and over one `AppState` (an agent over MCP and the web UI share the session).
//!
//! `/invoke` requests are checked in this order: `Host`, token, path, content type,
//! `Origin`, body. `/mcp` follows the app's MCP server: `Host`, `Origin` (any is refused),
//! token (`X-G4-Bridge-Token` or `Authorization: Bearer`), the agents switch (503 while it is
//! off), method (`POST` and `DELETE`), body. The checks keep other web pages (DNS rebinding,
//! cross-site `fetch`) and other local programs from driving the editor.

use std::io::Read;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::header::{AUTHORIZATION, CONNECTION, CONTENT_LENGTH, TRANSFER_ENCODING};
use hyper::{HeaderMap, Method};
use serde_json::{Value, json};
use tiny_http::{Header, Request, Response, Server};

use super::dispatch::{COMMANDS, DispatchError, Reply, dispatch};
use super::roots::Roots;
use crate::errors::CmdError;
use crate::mcp::host::StateSource;
use crate::mcp::http::{self as mcp_http, Guard, MCP_PATH, McpService, precheck, same_secret};
use crate::state::AppState;

/// Name of the header that carries the shared secret.
const TOKEN_HEADER: &str = "X-G4-Bridge-Token";
/// Number of threads that answer requests.
const WORKERS: usize = 4;
/// Largest request body that is read.
const MAX_BODY: u64 = 64 * 1024 * 1024;
/// Threads of the runtime MCP runs on (rmcp's session workers; tool calls go to its
/// blocking pool).
const MCP_THREADS: usize = 2;
/// How long stopping waits for MCP work still running.
const MCP_SHUTDOWN: Duration = Duration::from_secs(1);
/// `/mcp` while "Lokale Agenten zulassen" is off.
const AGENTS_OFF: &str = "Lokale Agenten sind aus (Einstellungen › Agenten)";
/// `/mcp` calls running at once, each on a thread of its own (a call can wait 120 s for
/// the user, so it must not hold one of the workers the UI needs to answer).
const MCP_CALLS: usize = 16;
const TOO_MANY_CALLS: &str = "Zu viele gleichzeitige Agenten-Aufrufe";

pub struct ServeConfig {
    /// Port on `127.0.0.1`; 0 picks a free one (see `BridgeServer::addr`).
    pub port: u16,
    pub token: String,
    /// The only `Origin` a browser request may carry (requests without one are accepted).
    pub origin: String,
    pub roots: Roots,
    /// Folder the agent settings (`agents.json`) and the base ROMs (`base-roms.json`) are
    /// kept in, the member hashes of the base ROMs below `cache/`; `None` keeps them in
    /// memory.
    pub app_data: Option<PathBuf>,
}

pub struct BridgeServer {
    pub addr: SocketAddr,
    server: Arc<Server>,
    stopping: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
    shared: Arc<Shared>,
    rt: tokio::runtime::Runtime,
}

struct Shared {
    state: Arc<AppState>,
    /// MCP over the same state; its requests run on `rt` (rmcp spawns session workers there).
    mcp: McpService,
    rt: tokio::runtime::Handle,
    roots: Roots,
    token: String,
    origin: String,
    port: u16,
    /// `/mcp` calls running now (at most [`MCP_CALLS`]).
    mcp_calls: AtomicUsize,
}

impl BridgeServer {
    /// Binds `127.0.0.1:<port>` and starts the worker threads, all sharing one fresh
    /// `AppState` (its agent settings and base ROMs from `config.app_data`) and one MCP
    /// service over it.
    pub fn start(config: ServeConfig) -> Result<BridgeServer, String> {
        let state = Arc::new(AppState::default());
        if let Some(dir) = &config.app_data {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("App-Datenordner {} nicht anlegbar: {e}", dir.display()))?;
            state.agents.configure(Some(dir.clone()));
        }
        // Without an app data folder too: `G4_BASE_ROM_DIR` and the originals of `G4_ROMS_DIR`.
        // The member hashes of the base ROMs are cached in `<app_data>/cache` (the app's
        // cache folder stands in); without one they are computed in memory.
        state
            .base_roms
            .configure_cache(config.app_data.as_ref().map(|dir| dir.join("cache")));
        state.base_roms.configure(config.app_data);
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(MCP_THREADS)
            .thread_name("bridge-mcp")
            .enable_all()
            .build()
            .map_err(|e| format!("Keine Laufzeit für MCP: {e}"))?;
        let mcp = {
            let _enter = rt.enter();
            mcp_http::service(Arc::new(Arc::clone(&state)) as Arc<dyn StateSource>)
        };
        // "Trennen" in the UI closes every MCP session.
        state
            .agents
            .set_session_closer(mcp.session_closer(rt.handle().clone()));
        let server = Server::http(("127.0.0.1", config.port))
            .map_err(|e| format!("Port {} nicht verfügbar: {e}", config.port))?;
        let addr = server
            .server_addr()
            .to_ip()
            .ok_or("Server hat keine IP-Adresse")?;
        let server = Arc::new(server);
        let stopping = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Shared {
            state,
            mcp,
            rt: rt.handle().clone(),
            roots: config.roots,
            token: config.token,
            origin: config.origin,
            port: addr.port(),
            mcp_calls: AtomicUsize::new(0),
        });
        let workers = (0..WORKERS)
            .map(|_| {
                let (server, stopping, shared) = (server.clone(), stopping.clone(), shared.clone());
                std::thread::spawn(move || worker(&server, &stopping, &shared))
            })
            .collect();
        Ok(BridgeServer {
            addr,
            server,
            stopping,
            workers,
            shared,
            rt,
        })
    }

    /// Stops accepting requests: denies the agents' open questions and closes their sessions
    /// first (a call waiting for the user ends now), then waits for the workers (requests in
    /// flight finish first) and closes the runtime.
    pub fn stop(self) {
        self.stopping.store(true, Ordering::SeqCst);
        self.shared.state.agents.disconnect_all();
        self.rt.block_on(self.shared.mcp.close_all());
        // `unblock` frees one waiting `recv` per call.
        for _ in &self.workers {
            self.server.unblock();
        }
        for worker in self.workers {
            let _ = worker.join();
        }
        self.rt.shutdown_timeout(MCP_SHUTDOWN);
    }
}

fn worker(server: &Server, stopping: &AtomicBool, shared: &Arc<Shared>) {
    loop {
        match server.recv() {
            Ok(request) => {
                if stopping.load(Ordering::SeqCst) {
                    return;
                }
                handle(shared, request);
            }
            Err(_) if stopping.load(Ordering::SeqCst) => return,
            Err(_) => {}
        }
    }
}

/// Status, content type and body for a dispatch result.
pub fn response_parts(reply: Reply) -> (u16, &'static str, Vec<u8>) {
    match reply {
        Reply::Json(value) => (200, "application/json", value.to_string().into_bytes()),
        Reply::Bytes(bytes) => (200, "application/octet-stream", bytes),
    }
}

type Parts = (u16, &'static str, Vec<u8>);

fn failure(status: u16, message: &str) -> Parts {
    let error = CmdError::from(message.to_owned());
    (status, "application/json", json_bytes(&error))
}

fn json_bytes(value: &impl serde::Serialize) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec())
}

fn header<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

fn is_json(content_type: &str) -> bool {
    content_type
        .split(';')
        .next()
        .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
}

type Answer = Response<std::io::Cursor<Vec<u8>>>;

fn plain((status, content_type, body): Parts) -> Answer {
    let mut response = Response::from_data(body).with_status_code(status);
    if let Ok(h) = Header::from_bytes("Content-Type", content_type) {
        response = response.with_header(h);
    }
    response
}

fn handle(shared: &Arc<Shared>, mut request: Request) {
    if request.url().split('?').next() == Some(MCP_PATH) {
        match prepare_mcp(shared, &mut request) {
            Ok(forwarded) => spawn_mcp(shared, request, forwarded),
            Err(refusal) => {
                let _ = request.respond(refusal);
            }
        }
        return;
    }
    let response = plain(answer(shared, &mut request));
    let _ = request.respond(response);
}

/// One of the [`MCP_CALLS`] slots; dropping it frees the slot.
struct McpSlot(Arc<Shared>);

impl McpSlot {
    fn take(shared: &Arc<Shared>) -> Option<McpSlot> {
        let running = shared.mcp_calls.fetch_add(1, Ordering::SeqCst);
        if running >= MCP_CALLS {
            shared.mcp_calls.fetch_sub(1, Ordering::SeqCst);
            return None;
        }
        Some(McpSlot(Arc::clone(shared)))
    }
}

impl Drop for McpSlot {
    fn drop(&mut self) {
        self.0.mcp_calls.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Answers a checked `/mcp` request on a thread of its own, so the worker goes back to
/// `recv` at once and `/invoke` (the UI answering a question) stays free.
fn spawn_mcp(shared: &Arc<Shared>, request: Request, forwarded: hyper::Request<Full<Bytes>>) {
    let Some(slot) = McpSlot::take(shared) else {
        let _ = request.respond(refused(503, TOO_MANY_CALLS));
        return;
    };
    // Should the thread not start, the request is dropped with it and its connection closes.
    let _ = std::thread::Builder::new()
        .name("bridge-mcp-call".to_owned())
        .spawn(move || {
            let response = run_mcp(&slot.0, forwarded);
            let _ = request.respond(response);
            drop(slot);
        });
}

/// A refused `/mcp` request (a `CmdError`, as the app's server answers).
fn refused(status: u16, message: &str) -> Answer {
    let mut response = plain(failure(status, message));
    if status == 405
        && let Ok(h) = Header::from_bytes("Allow", "POST, DELETE")
    {
        response = response.with_header(h);
    }
    response
}

/// The request's headers as hyper's (a header hyper cannot represent is left out).
fn hyper_headers(request: &Request) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for h in request.headers() {
        let name = hyper::header::HeaderName::from_bytes(h.field.as_str().as_bytes());
        let value = hyper::header::HeaderValue::from_bytes(h.value.as_bytes());
        if let (Ok(name), Ok(value)) = (name, value) {
            headers.append(name, value);
        }
    }
    headers
}

/// `/mcp` up to rmcp: the app's precheck (with the bridge's token header as an alternative
/// to `Authorization: Bearer`), the agents switch, the body; the request for rmcp or the
/// refusal.
fn prepare_mcp(
    shared: &Shared,
    request: &mut Request,
) -> Result<hyper::Request<Full<Bytes>>, Answer> {
    let Ok(method) = Method::from_bytes(request.method().as_str().as_bytes()) else {
        return Err(refused(405, "Methode nicht erlaubt"));
    };
    let url = request.url().to_owned();
    let path = url.split('?').next().unwrap_or("");
    let headers = hyper_headers(request);
    let mut checked = headers.clone();
    if header(request, TOKEN_HEADER).is_some_and(|token| same_secret(token, &shared.token))
        && let Ok(bearer) = format!("Bearer {}", shared.token).parse()
    {
        checked.insert(AUTHORIZATION, bearer);
    }
    match precheck(&method, path, &checked, shared.port, &shared.token) {
        Guard::Refuse(status @ (401 | 403), message) => return Err(refused(status, message)),
        _ if !shared.state.agents.settings().enabled => return Err(refused(503, AGENTS_OFF)),
        Guard::Refuse(status, message) => return Err(refused(status, message)),
        Guard::Ok => {}
    }
    let mut raw = Vec::new();
    if let Err(e) = request.as_reader().take(MAX_BODY + 1).read_to_end(&mut raw) {
        return Err(refused(400, &format!("Körper nicht lesbar: {e}")));
    }
    if raw.len() as u64 > MAX_BODY {
        return Err(refused(413, "Körper zu groß"));
    }
    let mut forwarded = hyper::Request::new(Full::new(Bytes::from(raw)));
    *forwarded.method_mut() = method;
    match url.parse() {
        Ok(uri) => *forwarded.uri_mut() = uri,
        Err(_) => return Err(refused(400, "Pfad nicht lesbar")),
    }
    *forwarded.headers_mut() = headers;
    Ok(forwarded)
}

/// rmcp's answer to a checked `/mcp` request, on the bridge's runtime.
fn run_mcp(shared: &Shared, forwarded: hyper::Request<Full<Bytes>>) -> Answer {
    // A panic in rmcp must not take the thread down without an answer.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        shared.rt.block_on(async {
            let (parts, body) = mcp_http::handle(&shared.mcp, forwarded).await.into_parts();
            let bytes = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(never) => match never {},
            };
            (parts, bytes)
        })
    }));
    let Ok((parts, bytes)) = outcome else {
        return refused(500, "MCP ist abgestürzt");
    };
    let mut response = Response::from_data(bytes.to_vec()).with_status_code(parts.status.as_u16());
    for (name, value) in &parts.headers {
        // tiny_http frames the body itself.
        if name == CONTENT_LENGTH || name == TRANSFER_ENCODING || name == CONNECTION {
            continue;
        }
        if let Ok(h) = Header::from_bytes(name.as_str(), value.as_bytes()) {
            response.add_header(h);
        }
    }
    response
}

fn answer(shared: &Shared, request: &mut Request) -> Parts {
    let port = shared.port;
    let host = header(request, "Host").unwrap_or("").to_ascii_lowercase();
    if host != format!("127.0.0.1:{port}") && host != format!("localhost:{port}") {
        return failure(403, "Host nicht erlaubt");
    }
    match header(request, TOKEN_HEADER) {
        Some(token) if same_secret(token, &shared.token) => {}
        _ => return failure(401, "Token fehlt oder ist falsch"),
    }
    let method = request.method().as_str().to_owned();
    let url = request.url().to_owned();
    let path = url.split('?').next().unwrap_or("");
    if method == "GET" && path == "/health" {
        let body = json!({ "ok": true, "commands": COMMANDS.len() });
        return (200, "application/json", json_bytes(&body));
    }
    let Some(name) = path.strip_prefix("/invoke/").filter(|_| method == "POST") else {
        return failure(404, "Nicht gefunden");
    };
    if !header(request, "Content-Type").is_some_and(is_json) {
        return failure(415, "Content-Type muss application/json sein");
    }
    if let Some(origin) = header(request, "Origin")
        && origin != shared.origin
    {
        return failure(403, "Origin nicht erlaubt");
    }
    let mut raw = Vec::new();
    if let Err(e) = request.as_reader().take(MAX_BODY + 1).read_to_end(&mut raw) {
        return failure(400, &format!("Körper nicht lesbar: {e}"));
    }
    if raw.len() as u64 > MAX_BODY {
        return failure(413, "Körper zu groß");
    }
    let args: Value = if raw.iter().all(u8::is_ascii_whitespace) {
        json!({})
    } else {
        match serde_json::from_slice(&raw) {
            Ok(value) => value,
            Err(e) => return failure(400, &format!("Körper ist kein JSON: {e}")),
        }
    };
    let name = name.to_owned();
    // A panicking command must not take its worker thread down with it.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dispatch(&shared.state, &shared.roots, &name, args)
    }));
    // The switch going off ends the agents' sessions, as stopping the app's server does.
    if name == "agent_set_enabled" && !shared.state.agents.settings().enabled {
        shared.rt.block_on(shared.mcp.close_all());
    }
    match outcome {
        Ok(Ok(reply)) => response_parts(reply),
        Ok(Err(DispatchError::Unknown(name))) => {
            failure(404, &format!("Unbekannter Befehl: {name}"))
        }
        Ok(Err(DispatchError::Cmd(error))) => (400, "application/json", json_bytes(&error)),
        Err(_) => failure(500, "Befehl ist abgestürzt"),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpStream;

    const TOKEN: &str = "secret";
    const ORIGIN: &str = "http://127.0.0.1:1430";

    struct Reply1 {
        status: u16,
        content_type: String,
        head: String,
        body: Vec<u8>,
    }

    impl Reply1 {
        fn json(&self) -> Value {
            serde_json::from_slice(&self.body).unwrap()
        }

        /// The value of a response header (any case).
        fn header(&self, name: &str) -> Option<String> {
            self.head.lines().find_map(|l| {
                let (n, value) = l.split_once(':')?;
                n.eq_ignore_ascii_case(name)
                    .then(|| value.trim().to_owned())
            })
        }

        /// The one JSON-RPC message of an MCP answer, as JSON or as an SSE `data:` line.
        fn message(&self) -> Value {
            let text = String::from_utf8_lossy(&self.body);
            let data = text
                .lines()
                .filter_map(|l| l.strip_prefix("data:"))
                .map(str::trim)
                .find(|d| !d.is_empty());
            serde_json::from_str(data.unwrap_or(&text)).unwrap()
        }
    }

    fn start_with(dir: &std::path::Path) -> BridgeServer {
        BridgeServer::start(ServeConfig {
            port: 0,
            token: TOKEN.into(),
            origin: ORIGIN.into(),
            roots: Roots::new(&[dir.to_path_buf()]).unwrap(),
            app_data: None,
        })
        .unwrap()
    }

    /// Sends one HTTP/1.1 request with exactly the given headers and reads the whole answer.
    fn request(
        server: &BridgeServer,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Reply1 {
        let mut stream = TcpStream::connect(server.addr).unwrap();
        // A hung server fails the test instead of hanging it.
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(60)))
            .unwrap();
        let mut head = format!("{method} {path} HTTP/1.1\r\n");
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
        let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = String::from_utf8_lossy(&raw[..split]).into_owned();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        let content_type = head
            .lines()
            .find_map(|l| {
                let (name, value) = l.split_once(':')?;
                name.eq_ignore_ascii_case("content-type")
                    .then(|| value.trim().to_owned())
            })
            .unwrap_or_default();
        Reply1 {
            status,
            content_type,
            head,
            body: raw[split + 4..].to_vec(),
        }
    }

    fn good_headers(server: &BridgeServer) -> Vec<(&'static str, String)> {
        vec![
            ("Host", format!("127.0.0.1:{}", server.addr.port())),
            (TOKEN_HEADER, TOKEN.into()),
            ("Content-Type", "application/json".into()),
        ]
    }

    fn send(
        server: &BridgeServer,
        method: &str,
        path: &str,
        headers: &[(&str, String)],
        body: &str,
    ) -> Reply1 {
        let headers: Vec<(&str, &str)> = headers.iter().map(|(n, v)| (*n, v.as_str())).collect();
        request(server, method, path, &headers, body)
    }

    fn post(server: &BridgeServer, path: &str, body: &str) -> Reply1 {
        send(server, "POST", path, &good_headers(server), body)
    }

    fn without(headers: Vec<(&'static str, String)>, name: &str) -> Vec<(&'static str, String)> {
        headers.into_iter().filter(|(n, _)| *n != name).collect()
    }

    fn with(
        mut headers: Vec<(&'static str, String)>,
        name: &'static str,
        value: &str,
    ) -> Vec<(&'static str, String)> {
        headers.retain(|(n, _)| *n != name);
        headers.push((name, value.into()));
        headers
    }

    fn server() -> (tempfile::TempDir, BridgeServer) {
        let dir = tempfile::tempdir().unwrap();
        let server = start_with(dir.path());
        (dir, server)
    }

    #[test]
    fn binds_to_loopback_only() {
        let (_dir, server) = server();
        assert_eq!(server.addr.ip().to_string(), "127.0.0.1");
        server.stop();
    }

    #[test]
    fn health_answers_with_the_command_count() {
        let (_dir, server) = server();
        let reply = send(&server, "GET", "/health", &good_headers(&server), "");
        assert_eq!(reply.status, 200);
        assert_eq!(reply.content_type, "application/json");
        assert_eq!(
            reply.json(),
            json!({ "ok": true, "commands": COMMANDS.len() })
        );
        server.stop();
    }

    #[test]
    fn localhost_host_is_accepted() {
        let (_dir, server) = server();
        let headers = with(
            good_headers(&server),
            "Host",
            &format!("localhost:{}", server.addr.port()),
        );
        assert_eq!(send(&server, "GET", "/health", &headers, "").status, 200);
        server.stop();
    }

    #[test]
    fn wrong_host_is_403() {
        let (_dir, server) = server();
        let host = format!("evil.example:{}", server.addr.port());
        let headers = with(good_headers(&server), "Host", &host);
        assert_eq!(send(&server, "GET", "/health", &headers, "").status, 403);
        // The right name on another port is refused too.
        let headers = with(good_headers(&server), "Host", "127.0.0.1:1");
        assert_eq!(send(&server, "GET", "/health", &headers, "").status, 403);
        server.stop();
    }

    #[test]
    fn missing_token_is_401() {
        let (_dir, server) = server();
        let headers = without(good_headers(&server), TOKEN_HEADER);
        assert_eq!(send(&server, "GET", "/health", &headers, "").status, 401);
        server.stop();
    }

    #[test]
    fn wrong_token_is_401() {
        let (_dir, server) = server();
        let headers = with(good_headers(&server), TOKEN_HEADER, "nope");
        assert_eq!(send(&server, "GET", "/health", &headers, "").status, 401);
        server.stop();
    }

    #[test]
    fn foreign_origin_is_403() {
        let (_dir, server) = server();
        let headers = with(good_headers(&server), "Origin", "http://evil.example");
        assert_eq!(
            send(&server, "POST", "/invoke/dirty", &headers, "{}").status,
            403
        );
        server.stop();
    }

    #[test]
    fn own_origin_is_accepted() {
        let (_dir, server) = server();
        let headers = with(good_headers(&server), "Origin", ORIGIN);
        // `dirty` fails without a project, but only after every check has passed.
        assert_eq!(
            send(&server, "POST", "/invoke/dirty", &headers, "{}").status,
            400
        );
        server.stop();
    }

    #[test]
    fn text_plain_is_415() {
        let (_dir, server) = server();
        let headers = with(good_headers(&server), "Content-Type", "text/plain");
        assert_eq!(
            send(&server, "POST", "/invoke/dirty", &headers, "{}").status,
            415
        );
        let headers = without(good_headers(&server), "Content-Type");
        assert_eq!(
            send(&server, "POST", "/invoke/dirty", &headers, "{}").status,
            415
        );
        let headers = with(
            good_headers(&server),
            "Content-Type",
            "application/json; charset=utf-8",
        );
        assert_eq!(
            send(&server, "POST", "/invoke/dirty", &headers, "{}").status,
            400
        );
        server.stop();
    }

    #[test]
    fn unknown_command_is_404() {
        let (_dir, server) = server();
        assert_eq!(post(&server, "/invoke/no_such_command", "{}").status, 404);
        assert_eq!(
            send(&server, "GET", "/elsewhere", &good_headers(&server), "").status,
            404
        );
        assert_eq!(
            send(&server, "GET", "/invoke/dirty", &good_headers(&server), "").status,
            404
        );
        server.stop();
    }

    #[test]
    fn malformed_body_is_400() {
        let (_dir, server) = server();
        assert_eq!(post(&server, "/invoke/dirty", "{not json").status, 400);
        server.stop();
    }

    #[test]
    fn cmd_error_is_400_with_code() {
        let (_dir, server) = server();
        let reply = post(&server, "/invoke/dirty", "");
        assert_eq!(reply.status, 400);
        assert_eq!(reply.content_type, "application/json");
        let body = reply.json();
        assert_eq!(body["code"], "error");
        assert_eq!(body["message"], "no project open");
        server.stop();
    }

    #[test]
    fn success_is_200_json() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let server = start_with(fixture.dir.path());
        let args = json!({ "path": fixture.dir.path().to_str().unwrap() }).to_string();
        let reply = post(&server, "/invoke/open_project", &args);
        assert_eq!(
            reply.status,
            200,
            "{}",
            String::from_utf8_lossy(&reply.body)
        );
        assert_eq!(reply.content_type, "application/json");
        // The state is shared between requests: the project is now open.
        let reply = post(&server, "/invoke/dirty", "{}");
        assert_eq!(reply.status, 200);
        server.stop();
    }

    #[test]
    fn bytes_reply_is_octet_stream() {
        assert_eq!(
            response_parts(Reply::Bytes(vec![1, 2, 3])),
            (200, "application/octet-stream", vec![1, 2, 3])
        );
        assert_eq!(
            response_parts(Reply::Json(json!(null))),
            (200, "application/json", b"null".to_vec())
        );
    }

    const MCP_OFF: &str = "Lokale Agenten sind aus (Einstellungen › Agenten)";

    fn mcp_headers(server: &BridgeServer, session: Option<&str>) -> Vec<(&'static str, String)> {
        let mut headers = vec![
            ("Host", format!("127.0.0.1:{}", server.addr.port())),
            (TOKEN_HEADER, TOKEN.into()),
            ("Content-Type", "application/json".into()),
            ("Accept", "application/json, text/event-stream".into()),
        ];
        if let Some(session) = session {
            headers.push(("Mcp-Session-Id", session.into()));
            headers.push(("MCP-Protocol-Version", "2025-11-25".into()));
        }
        headers
    }

    fn initialize_body() -> String {
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "bridge-test", "version": "1" }
            }
        })
        .to_string()
    }

    /// `initialize` and `notifications/initialized` over `/mcp`; the session id.
    fn mcp_session(server: &BridgeServer) -> String {
        let reply = send(
            server,
            "POST",
            "/mcp",
            &mcp_headers(server, None),
            &initialize_body(),
        );
        assert_eq!(
            reply.status,
            200,
            "{}",
            String::from_utf8_lossy(&reply.body)
        );
        assert_eq!(
            reply.message()["result"]["serverInfo"]["name"],
            "gen4-studio"
        );
        let session = reply.header("mcp-session-id").unwrap();
        let note = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        let reply = send(
            server,
            "POST",
            "/mcp",
            &mcp_headers(server, Some(&session)),
            &note.to_string(),
        );
        assert_eq!(reply.status, 202);
        session
    }

    fn enable_agents(server: &BridgeServer) {
        let reply = post(server, "/invoke/agent_set_enabled", r#"{"on":true}"#);
        assert_eq!(
            reply.status,
            200,
            "{}",
            String::from_utf8_lossy(&reply.body)
        );
    }

    #[test]
    fn mcp_is_503_while_agents_are_off() {
        let (_dir, server) = server();
        for method in ["POST", "GET", "DELETE"] {
            let reply = send(
                &server,
                method,
                "/mcp",
                &mcp_headers(&server, None),
                &initialize_body(),
            );
            assert_eq!(reply.status, 503, "{method}");
            assert_eq!(reply.json()["message"], MCP_OFF);
        }
        // The token still comes first.
        let headers = without(mcp_headers(&server, None), TOKEN_HEADER);
        assert_eq!(send(&server, "POST", "/mcp", &headers, "{}").status, 401);
        server.stop();
    }

    #[test]
    fn mcp_refuses_any_origin() {
        let (_dir, server) = server();
        for enabled in [false, true] {
            if enabled {
                enable_agents(&server);
            }
            // Even the UI's own origin: agents are no browsers.
            for origin in [ORIGIN, "http://evil.example", "null"] {
                let headers = with(mcp_headers(&server, None), "Origin", origin);
                let reply = send(&server, "POST", "/mcp", &headers, &initialize_body());
                assert_eq!(reply.status, 403, "{origin}");
                assert_eq!(reply.json()["message"], "Origin nicht erlaubt");
                // Before the token.
                let headers = without(headers, TOKEN_HEADER);
                assert_eq!(send(&server, "POST", "/mcp", &headers, "{}").status, 403);
            }
        }
        server.stop();
    }

    #[test]
    fn mcp_takes_bearer_and_refuses_get() {
        let (_dir, server) = server();
        enable_agents(&server);
        let bearer = with(
            without(mcp_headers(&server, None), TOKEN_HEADER),
            "Authorization",
            &format!("Bearer {TOKEN}"),
        );
        let reply = send(&server, "POST", "/mcp", &bearer, &initialize_body());
        assert_eq!(
            reply.status,
            200,
            "{}",
            String::from_utf8_lossy(&reply.body)
        );
        assert!(reply.header("mcp-session-id").is_some());
        let wrong = with(bearer.clone(), "Authorization", "Bearer nope");
        assert_eq!(send(&server, "POST", "/mcp", &wrong, "{}").status, 401);
        let wrong_host = with(bearer.clone(), "Host", "evil.example");
        assert_eq!(send(&server, "POST", "/mcp", &wrong_host, "{}").status, 403);
        let reply = send(&server, "GET", "/mcp", &bearer, "");
        assert_eq!(reply.status, 405);
        assert_eq!(reply.header("allow").as_deref(), Some("POST, DELETE"));
        // `/invoke` still wants its own header.
        let invoke = with(bearer, "Content-Type", "application/json");
        assert_eq!(
            send(&server, "POST", "/invoke/dirty", &invoke, "{}").status,
            401
        );
        server.stop();
    }

    #[test]
    fn mcp_shares_the_session_with_invoke() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let server = start_with(fixture.dir.path());
        let args = json!({ "path": fixture.dir.path().to_str().unwrap() }).to_string();
        assert_eq!(post(&server, "/invoke/open_project", &args).status, 200);
        enable_agents(&server);
        let reply = post(&server, "/invoke/agent_set_grant", r#"{"level":"write"}"#);
        assert_eq!(
            reply.status,
            200,
            "{}",
            String::from_utf8_lossy(&reply.body)
        );
        let session = mcp_session(&server);
        // The UI sees the connection.
        let status = post(&server, "/invoke/agent_status", "{}").json();
        assert_eq!(status["bridge"], true);
        assert_eq!(status["connections"][0]["client"], "bridge-test");
        let call = json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": "data_edit", "arguments": {
                "kind": "species", "index": 1,
                "patch": { "Personal": { "field": "attack", "value": 123 } }
            } }
        });
        let reply = send(
            &server,
            "POST",
            "/mcp",
            &mcp_headers(&server, Some(&session)),
            &call.to_string(),
        );
        assert_eq!(
            reply.status,
            200,
            "{}",
            String::from_utf8_lossy(&reply.body)
        );
        let result = &reply.message()["result"];
        assert_ne!(result["isError"], true, "{result}");
        let history = post(&server, "/invoke/history", "{}").json();
        assert_eq!(
            history["undo_agents"].as_array().unwrap().last().unwrap(),
            "bridge-test"
        );
        server.stop();
    }

    #[test]
    fn turning_agents_off_closes_the_mcp_sessions() {
        let (_dir, server) = server();
        enable_agents(&server);
        let session = mcp_session(&server);
        assert_eq!(
            post(&server, "/invoke/agent_set_enabled", r#"{"on":false}"#).status,
            200
        );
        enable_agents(&server);
        let ping = json!({ "jsonrpc": "2.0", "id": 3, "method": "ping" });
        let reply = send(
            &server,
            "POST",
            "/mcp",
            &mcp_headers(&server, Some(&session)),
            &ping.to_string(),
        );
        assert_eq!(reply.status, 404);
        server.stop();
    }

    #[test]
    fn app_data_keeps_the_agent_settings() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("app-data");
        let config = |data: &std::path::Path| ServeConfig {
            port: 0,
            token: TOKEN.into(),
            origin: ORIGIN.into(),
            roots: Roots::new(&[dir.path().to_path_buf()]).unwrap(),
            app_data: Some(data.to_path_buf()),
        };
        let server = BridgeServer::start(config(&data)).unwrap();
        enable_agents(&server);
        server.stop();
        assert!(data.join("agents.json").is_file());
        let server = BridgeServer::start(config(&data)).unwrap();
        let status = post(&server, "/invoke/agent_status", "{}").json();
        assert_eq!(status["enabled"], true);
        server.stop();
    }

    #[test]
    fn app_data_configures_the_base_roms() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("app-data");
        let remembered = crate::base_roms::BaseRomSettings {
            version: crate::base_roms::BASE_ROMS_VERSION,
            roms: vec![crate::base_roms::BaseRomEntry {
                sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4".into(),
                path: "/roms/pt.nds".into(),
                size: 1,
                mtime_ms: 2,
                game_code: "CPUD".into(),
                label: None,
                verified: "2026-10-27T10:00:00Z".into(),
            }],
        };
        remembered.save(&data).unwrap();
        let server = BridgeServer::start(ServeConfig {
            port: 0,
            token: TOKEN.into(),
            origin: ORIGIN.into(),
            roots: Roots::new(&[dir.path().to_path_buf()]).unwrap(),
            app_data: Some(data.clone()),
        })
        .unwrap();
        assert_eq!(server.shared.state.base_roms.settings(), remembered);
        server.stop();
    }

    #[test]
    fn app_data_configures_the_base_hash_cache() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("app-data");
        let config = |app_data: Option<PathBuf>| ServeConfig {
            port: 0,
            token: TOKEN.into(),
            origin: ORIGIN.into(),
            roots: Roots::new(&[dir.path().to_path_buf()]).unwrap(),
            app_data,
        };
        let server = BridgeServer::start(config(Some(data.clone()))).unwrap();
        assert_eq!(
            server.shared.state.base_roms.cache_dir(),
            Some(data.join("cache"))
        );
        server.stop();
        // Without an app data folder the hashes stay in memory.
        let server = BridgeServer::start(config(None)).unwrap();
        assert_eq!(server.shared.state.base_roms.cache_dir(), None);
        server.stop();
    }

    /// Env variable that marks the child run of
    /// [`bridge_without_app_data_skips_originals_and_searches_the_base_rom_dir`]; it holds the
    /// test folder.
    const ORIGINALS_CHILD: &str = "G4_TEST_BRIDGE_ORIGINALS_CHILD";

    /// The environment is the process's: the test runs itself again as a child process with
    /// `G4_ROMS_DIR` and `G4_BASE_ROM_DIR` set, the parent's environment stays as it is.
    #[test]
    fn bridge_without_app_data_skips_originals_and_searches_the_base_rom_dir() {
        let name = "bridge::http::tests::\
                    bridge_without_app_data_skips_originals_and_searches_the_base_rom_dir";
        let Some(tmp) = std::env::var_os(ORIGINALS_CHILD).map(std::path::PathBuf::from) else {
            let dir = tempfile::tempdir().unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", name, "--nocapture", "--test-threads=1"])
                .env(ORIGINALS_CHILD, dir.path())
                .env("G4_ROMS_DIR", dir.path().join("Roms"))
                .env("G4_BASE_ROM_DIR", dir.path().join("basis"))
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success() && stdout.contains("1 passed"),
                "{stdout}\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        };
        // `<dir>/<stem>_DSPRE_contents` (game code CPUE) and its sibling `<stem>.nds`, a
        // header-only ROM whose last byte is `variant`; answers the folder and the mark.
        let project = |dir: &std::path::Path, stem: &str, variant: u8| {
            let root = dir.join(format!("{stem}_DSPRE_contents"));
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(root.join("header.yaml"), "gamecode: CPUE\n").unwrap();
            let nds = dir.join(format!("{stem}.nds"));
            let mut bytes = vec![0u8; 0x400];
            bytes[0x00..0x0A].copy_from_slice(b"POKEMON PL");
            bytes[0x0C..0x10].copy_from_slice(b"CPUE");
            bytes[0x3FF] = variant;
            std::fs::write(&nds, bytes).unwrap();
            let info = g4_kernel::start::inspect_rom(&nds).unwrap();
            let mark = g4_kernel::start::BaseMark {
                version: g4_kernel::start::BASE_MARK_VERSION,
                origin: g4_kernel::start::ORIGIN_DSPRE.into(),
                sha1: info.sha1,
                game_code: info.game_code,
                title: info.title,
                revision: info.revision,
                size: info.size,
                template: None,
                created: "2026-10-27T10:00:00Z".into(),
                assigned: None,
                studio_version: "0.0.0".into(),
                secure_area_crc: 0,
                secure_area_sha1: String::new(),
            };
            (root, nds, mark)
        };
        let roms = tmp.join("Roms");
        let (original_root, _, original_mark) = project(&roms, "Spiel", 1);
        let (copy_root, copy_nds, copy_mark) = project(&roms.join("_testkopien"), "Spiel", 1);
        let (elsewhere, _, _) = project(&tmp.join("woanders"), "Anderes", 2);
        let (_, basis_nds, basis_mark) = project(&tmp.join("basis"), "Basis", 3);
        let server = BridgeServer::start(ServeConfig {
            port: 0,
            token: TOKEN.into(),
            origin: ORIGIN.into(),
            roots: Roots::new(std::slice::from_ref(&tmp)).unwrap(),
            app_data: None,
        })
        .unwrap();
        let base = &server.shared.state.base_roms;
        // The original next to its project is no candidate.
        assert_eq!(base.locate(&original_root, &original_mark).unwrap(), None);
        // The copy under `_testkopien` is.
        let found = base.locate(&copy_root, &copy_mark).unwrap().unwrap();
        assert_eq!(found.source, g4_kernel::base::BaseSource::Sibling);
        assert_eq!(
            found.path.canonicalize().unwrap(),
            copy_nds.canonicalize().unwrap()
        );
        // `G4_BASE_ROM_DIR` is searched.
        let found = base.locate(&elsewhere, &basis_mark).unwrap().unwrap();
        assert_eq!(found.source, g4_kernel::base::BaseSource::Dir);
        assert_eq!(
            found.path.canonicalize().unwrap(),
            basis_nds.canonicalize().unwrap()
        );
        server.stop();
    }

    /// Parks `count` `tools/call get_header` calls (one session each) on the connect question
    /// of the open project (agents on, no grant) and runs `then` while they wait; answers
    /// what the calls got.
    fn with_parked_calls(
        server: &BridgeServer,
        count: usize,
        then: impl FnOnce(u64),
    ) -> Vec<Reply1> {
        let sessions: Vec<String> = (0..count).map(|_| mcp_session(server)).collect();
        std::thread::scope(|scope| {
            let calls: Vec<_> = sessions
                .iter()
                .map(|session| {
                    scope.spawn(move || {
                        let call = json!({
                            "jsonrpc": "2.0", "id": 7, "method": "tools/call",
                            "params": { "name": "get_header", "arguments": { "id": 0 } }
                        });
                        send(
                            server,
                            "POST",
                            "/mcp",
                            &mcp_headers(server, Some(session)),
                            &call.to_string(),
                        )
                    })
                })
                .collect();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                let parked = server.shared.mcp_calls.load(Ordering::SeqCst) == count;
                let question = &post(server, "/invoke/agent_status", "{}").json()["question"];
                if parked && !question.is_null() {
                    then(question["id"].as_u64().unwrap());
                    break;
                }
                assert!(std::time::Instant::now() < deadline, "calls not parked");
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            calls.into_iter().map(|c| c.join().unwrap()).collect()
        })
    }

    fn open_without_grant() -> (g4_kernel::test_support::Synthetic, BridgeServer) {
        let fixture = g4_kernel::test_support::synthetic_project();
        let server = start_with(fixture.dir.path());
        let args = json!({ "path": fixture.dir.path().to_str().unwrap() }).to_string();
        assert_eq!(post(&server, "/invoke/open_project", &args).status, 200);
        enable_agents(&server);
        (fixture, server)
    }

    /// Waits (at most 2 s) until every MCP call has given back its slot.
    fn wait_for_no_mcp_calls(shared: &Shared) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while shared.mcp_calls.load(Ordering::SeqCst) != 0 {
            assert!(
                std::time::Instant::now() < deadline,
                "mcp_calls stayed at {}",
                shared.mcp_calls.load(Ordering::SeqCst)
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn mcp_calls_waiting_on_a_question_leave_invoke_free() {
        let (_fixture, server) = open_without_grant();
        // More calls than workers.
        let replies = with_parked_calls(&server, WORKERS + 2, |question| {
            let started = std::time::Instant::now();
            let poll = post(&server, "/invoke/agent_poll", r#"{"since":0}"#);
            assert_eq!(poll.status, 200);
            assert!(started.elapsed() < std::time::Duration::from_secs(1));
            let answer = json!({ "question": question, "answer": "read" }).to_string();
            let reply = post(&server, "/invoke/agent_answer", &answer);
            assert_eq!(
                reply.status,
                200,
                "{}",
                String::from_utf8_lossy(&reply.body)
            );
        });
        for reply in replies {
            assert_eq!(reply.status, 200);
            let result = &reply.message()["result"];
            assert_ne!(result["isError"], true, "{result}");
        }
        // The call thread answers before it drops its slot: poll, do not read once.
        wait_for_no_mcp_calls(&server.shared);
        server.stop();
    }

    #[test]
    fn mcp_calls_above_the_cap_are_503() {
        let (_fixture, server) = open_without_grant();
        let extra = mcp_session(&server);
        let replies = with_parked_calls(&server, MCP_CALLS, |question| {
            let reply = send(
                &server,
                "POST",
                "/mcp",
                &mcp_headers(&server, Some(&extra)),
                &json!({ "jsonrpc": "2.0", "id": 8, "method": "ping" }).to_string(),
            );
            assert_eq!(reply.status, 503);
            assert_eq!(reply.json()["message"], TOO_MANY_CALLS);
            let answer = json!({ "question": question, "answer": "read" }).to_string();
            assert_eq!(post(&server, "/invoke/agent_answer", &answer).status, 200);
        });
        assert!(replies.iter().all(|r| r.status == 200));
        server.stop();
    }

    #[test]
    fn stop_denies_waiting_calls() {
        let (_fixture, server) = open_without_grant();
        let sessions: Vec<String> = (0..2).map(|_| mcp_session(&server)).collect();
        let addr = server.addr;
        let calls: Vec<_> = sessions
            .into_iter()
            .map(|session| {
                std::thread::spawn(move || {
                    let mut stream = TcpStream::connect(addr).unwrap();
                    let body = json!({
                        "jsonrpc": "2.0", "id": 9, "method": "tools/call",
                        "params": { "name": "get_header", "arguments": { "id": 0 } }
                    })
                    .to_string();
                    let head = format!(
                        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n{TOKEN_HEADER}: {TOKEN}\r\n\
                         Content-Type: application/json\r\nAccept: application/json, text/event-stream\r\n\
                         Mcp-Session-Id: {session}\r\nMCP-Protocol-Version: 2025-11-25\r\n\
                         Content-Length: {}\r\nConnection: close\r\n\r\n",
                        addr.port(),
                        body.len()
                    );
                    stream.write_all(head.as_bytes()).unwrap();
                    stream.write_all(body.as_bytes()).unwrap();
                    let mut raw = Vec::new();
                    let _ = stream.read_to_end(&mut raw);
                })
            })
            .collect();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while server.shared.state.agents.status().question.is_none()
            || server.shared.mcp_calls.load(Ordering::SeqCst) < 2
        {
            assert!(std::time::Instant::now() < deadline, "calls not parked");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let started = std::time::Instant::now();
        let shared = Arc::clone(&server.shared);
        server.stop();
        for call in calls {
            call.join().unwrap();
        }
        // Denied at once, not after the question's 120 s.
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
        assert!(shared.state.agents.status().question.is_none());
        wait_for_no_mcp_calls(&shared);
    }
}

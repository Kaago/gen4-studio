//! MCP over Streamable HTTP in the running app: rmcp's service with one session per client
//! behind our own precheck, served by hyper on `127.0.0.1` with a fresh port and token.
//!
//! Requests are checked in this order: `Host`, `Origin` (any is refused: agents are no
//! browsers), token (`Authorization: Bearer <token>`), path (`/mcp` only), method (`POST`
//! and `DELETE`; `GET` would open a server stream nobody needs), body size.

use std::convert::Infallible;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::body::Incoming;
use hyper::header::{ALLOW, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, HOST, ORIGIN};
use hyper::{HeaderMap, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rmcp::transport::streamable_http_server::session::SessionManager;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};

use crate::errors::CmdError;
use crate::mcp::discovery::{self, Discovery};
use crate::mcp::host::StateSource;
use crate::mcp::hub::ServerControl;
use crate::mcp::server::Server;

/// The one path MCP is served on.
pub const MCP_PATH: &str = "/mcp";
/// Largest request body that is read.
pub const MAX_BODY: usize = 64 * 1024 * 1024;
/// A session nobody used for this long is closed.
pub const SESSION_IDLE: Duration = Duration::from_secs(30 * 60);
/// How much of a refused request's body is read and dropped (see [`drain`]), and how long.
const DRAIN_LIMIT: usize = 1024 * 1024;
const DRAIN_WAIT: Duration = Duration::from_secs(2);
/// Size of the token in bytes (it is written as hex).
const TOKEN_BYTES: usize = 32;

const HOST_REFUSED: &str = "Host nicht erlaubt";
const ORIGIN_REFUSED: &str = "Origin nicht erlaubt";
const TOKEN_REFUSED: &str = "Token fehlt oder ist falsch";
const NOT_FOUND: &str = "Nicht gefunden";
const METHOD_REFUSED: &str = "Methode nicht erlaubt";
const BODY_TOO_LARGE: &str = "Körper zu groß";

/// rmcp's Streamable HTTP service over the user's session, with its session manager (to
/// close every session on "Trennen" and when the server stops).
#[derive(Clone)]
pub struct McpService {
    inner: StreamableHttpService<Server, LocalSessionManager>,
    sessions: Arc<LocalSessionManager>,
}

impl McpService {
    /// Closes every open session: its server is dropped, which ends its connection in the
    /// hub, and a later request with its id gets 404.
    pub async fn close_all(&self) {
        close_sessions(&self.sessions).await;
    }

    /// Ends the open streams; the sessions are closed by [`McpService::close_all`].
    fn cancel(&self) {
        self.inner.config.cancellation_token.cancel();
    }
}

async fn close_sessions(sessions: &LocalSessionManager) {
    let ids: Vec<_> = sessions.sessions.read().await.keys().cloned().collect();
    for id in ids {
        // A session that ended meanwhile is gone already.
        let _ = sessions.close_session(&id).await;
    }
}

/// The service for `source`: a new [`Server::shared`] for each session (one connection in
/// the hub per client), sessions closed after [`SESSION_IDLE`]. Answers of a session are SSE
/// streams that end with the response: no keep-alive pings, no priming events.
pub fn service(source: Arc<dyn StateSource>) -> McpService {
    let mut sessions = LocalSessionManager::default();
    sessions.session_config.keep_alive = Some(SESSION_IDLE);
    sessions.session_config.sse_retry = None;
    let sessions = Arc::new(sessions);
    // rmcp's own host check stays on (loopback names on any port); ours is stricter.
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(true)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        .with_sse_retry(None)
        .with_max_request_body_bytes(MAX_BODY);
    let inner = StreamableHttpService::new(
        move || Ok(Server::shared(Arc::clone(&source))),
        Arc::clone(&sessions),
        config,
    );
    McpService { inner, sessions }
}

/// What the precheck says about a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guard {
    Ok,
    /// Status and German message.
    Refuse(u16, &'static str),
}

fn header(headers: &HeaderMap, name: hyper::header::HeaderName) -> Option<&str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

/// Checks a request to the server on `127.0.0.1:<port>` before its body is read: `Host`,
/// `Origin`, token, path, method, announced body size (in this order).
pub fn precheck(method: &Method, path: &str, headers: &HeaderMap, port: u16, token: &str) -> Guard {
    let host = header(headers, HOST).unwrap_or("").to_ascii_lowercase();
    if host != format!("127.0.0.1:{port}") && host != format!("localhost:{port}") {
        return Guard::Refuse(403, HOST_REFUSED);
    }
    if headers.contains_key(ORIGIN) {
        return Guard::Refuse(403, ORIGIN_REFUSED);
    }
    let sent = header(headers, AUTHORIZATION).and_then(|value| value.strip_prefix("Bearer "));
    if !sent.is_some_and(|sent| same_secret(sent.trim(), token)) {
        return Guard::Refuse(401, TOKEN_REFUSED);
    }
    if path != MCP_PATH {
        return Guard::Refuse(404, NOT_FOUND);
    }
    if method != Method::POST && method != Method::DELETE {
        return Guard::Refuse(405, METHOD_REFUSED);
    }
    let announced =
        header(headers, CONTENT_LENGTH).and_then(|value| value.trim().parse::<u64>().ok());
    if announced.is_some_and(|length| length > MAX_BODY as u64) {
        return Guard::Refuse(413, BODY_TOO_LARGE);
    }
    Guard::Ok
}

/// Equality that does not stop at the first differing byte.
pub fn same_secret(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// The JSON answer of a refused request (as the bridge's: a `CmdError`).
pub fn refusal(status: u16, message: &str) -> Response<Full<Bytes>> {
    let body =
        serde_json::to_vec(&CmdError::from(message.to_owned())).unwrap_or_else(|_| b"{}".to_vec());
    let mut response = Response::new(Full::new(Bytes::from(body)));
    *response.status_mut() =
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        hyper::header::HeaderValue::from_static("application/json"),
    );
    if status == 405 {
        headers.insert(
            ALLOW,
            hyper::header::HeaderValue::from_static("POST, DELETE"),
        );
    }
    response
}

/// Lets rmcp answer a request that passed the precheck, with the whole body collected (an
/// SSE answer of a session ends after the response). Runs inside a tokio runtime: rmcp
/// spawns the session workers there.
pub async fn handle(service: &McpService, request: Request<Full<Bytes>>) -> Response<Full<Bytes>> {
    let (parts, body) = service.inner.handle(request).await.into_parts();
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(never) => match never {},
    };
    Response::from_parts(parts, Full::new(bytes))
}

/// Reads and drops (at most [`DRAIN_LIMIT`], for at most [`DRAIN_WAIT`]) the body of a
/// request the precheck refused, after the decision: closing a connection with unread data
/// makes the system reset it, which can discard the answer before the client reads it.
async fn drain(body: Incoming) {
    let _ = tokio::time::timeout(DRAIN_WAIT, Limited::new(body, DRAIN_LIMIT).collect()).await;
}

/// One request on the app's server: precheck, body (at most [`MAX_BODY`]), rmcp.
async fn respond(
    service: &McpService,
    port: u16,
    token: &str,
    request: Request<Incoming>,
) -> Response<Full<Bytes>> {
    let guard = precheck(
        request.method(),
        request.uri().path(),
        request.headers(),
        port,
        token,
    );
    if let Guard::Refuse(status, message) = guard {
        drain(request.into_body()).await;
        return refusal(status, message);
    }
    let (parts, body) = request.into_parts();
    let bytes = match Limited::new(body, MAX_BODY).collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) if e.downcast_ref::<LengthLimitError>().is_some() => {
            return refusal(413, BODY_TOO_LARGE);
        }
        Err(e) => return refusal(400, &format!("Körper nicht lesbar: {e}")),
    };
    handle(service, Request::from_parts(parts, Full::new(bytes))).await
}

/// A fresh token: [`TOKEN_BYTES`] random bytes as hex.
fn new_token() -> Result<String, String> {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|e| format!("Kein Zufall für den Token: {e}"))?;
    let mut token = String::with_capacity(TOKEN_BYTES * 2);
    for byte in bytes {
        let _ = write!(token, "{byte:02x}");
    }
    Ok(token)
}

/// Accepts connections on `listener` until the service is cancelled; each connection is
/// served by hyper (HTTP/1.1) in a task of its own.
async fn accept(listener: std::net::TcpListener, service: McpService, port: u16, token: Arc<str>) {
    let Ok(listener) = tokio::net::TcpListener::from_std(listener) else {
        return;
    };
    let stopped = service.inner.config.cancellation_token.clone();
    loop {
        let stream = tokio::select! {
            () = stopped.cancelled() => return,
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => stream,
                Err(_) => {
                    // Out of descriptors or similar: try again a little later.
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }
            },
        };
        let (service, token, stopped) = (service.clone(), Arc::clone(&token), stopped.clone());
        tokio::spawn(async move {
            let answer = hyper::service::service_fn(move |request| {
                let (service, token) = (service.clone(), Arc::clone(&token));
                async move { Ok::<_, Infallible>(respond(&service, port, &token, request).await) }
            });
            let connection = hyper::server::conn::http1::Builder::new()
                .serve_connection(TokioIo::new(stream), answer);
            tokio::select! {
                _ = connection => {}
                () = stopped.cancelled() => {}
            }
        });
    }
}

/// The app's MCP server on `127.0.0.1`, with its discovery file.
pub struct HttpServer {
    pub port: u16,
    token: String,
    dir: PathBuf,
    service: McpService,
    rt: tokio::runtime::Handle,
}

impl HttpServer {
    /// Binds `127.0.0.1:0` with a fresh token, writes `mcp.json` into `data_dir`, lets
    /// "Trennen" in the hub close every session and serves on `rt`.
    pub fn start(
        source: Arc<dyn StateSource>,
        data_dir: &Path,
        rt: &tokio::runtime::Handle,
    ) -> Result<HttpServer, String> {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0))
            .map_err(|e| format!("Kein Port frei auf 127.0.0.1: {e}"))?;
        let port = listener
            .set_nonblocking(true)
            .and_then(|()| listener.local_addr())
            .map_err(|e| format!("Port nicht nutzbar: {e}"))?
            .port();
        let token = new_token()?;
        discovery::write(data_dir, &Discovery::new(port, &token))?;
        let service = service(Arc::clone(&source));
        // Only the session manager: a closer left behind after `stop` holds no server.
        let sessions = Arc::clone(&service.sessions);
        let closer_rt = rt.clone();
        source.state().agents.set_session_closer(Arc::new(move || {
            let sessions = Arc::clone(&sessions);
            closer_rt.spawn(async move { close_sessions(&sessions).await });
        }));
        rt.spawn(accept(
            listener,
            service.clone(),
            port,
            Arc::from(token.as_str()),
        ));
        Ok(HttpServer {
            port,
            token,
            dir: data_dir.to_path_buf(),
            service,
            rt: rt.clone(),
        })
    }

    /// Stops serving, closes every session (in the background on the runtime) and removes
    /// `mcp.json` (unless a later server wrote its own).
    pub fn stop(self) {
        if discovery::read(&self.dir).is_ok_and(|found| found.token == self.token) {
            discovery::remove(&self.dir);
        }
        self.service.cancel();
        let service = self.service;
        self.rt.spawn(async move { service.close_all().await });
    }
}

/// The app's switch for the server (see [`ServerControl`]), and its stop on exit.
pub struct AppServer {
    source: Arc<dyn StateSource>,
    dir: Option<PathBuf>,
    rt: tokio::runtime::Handle,
    running: Mutex<Option<HttpServer>>,
}

impl AppServer {
    /// A switch for a server over `source` that writes its discovery file into `dir` (the
    /// app data folder; without one the server cannot start) and serves on `rt`.
    pub fn new(
        source: Arc<dyn StateSource>,
        dir: Option<PathBuf>,
        rt: tokio::runtime::Handle,
    ) -> AppServer {
        AppServer {
            source,
            dir,
            rt,
            running: Mutex::new(None),
        }
    }

    /// Starts the server for `on` (a running one keeps its port), stops it otherwise
    /// (nothing to do when it does not run); the port it serves on.
    pub fn switch(&self, on: bool) -> Result<Option<u16>, String> {
        let mut running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        if !on {
            if let Some(server) = running.take() {
                server.stop();
            }
            return Ok(None);
        }
        if let Some(server) = running.as_ref() {
            return Ok(Some(server.port));
        }
        let dir = self
            .dir
            .as_deref()
            .ok_or("Kein App-Datenordner: der Server hat keinen Ort für mcp.json")?;
        let server = HttpServer::start(Arc::clone(&self.source), dir, &self.rt)?;
        let port = server.port;
        *running = Some(server);
        Ok(Some(port))
    }

    /// Stops the server if it runs (the app exits); the setting stays as it is.
    pub fn shutdown(&self) {
        let server = self
            .running
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(server) = server {
            server.stop();
        }
    }

    /// The control the hub calls for the switch.
    pub fn control(self: &Arc<Self>) -> ServerControl {
        let server = Arc::clone(self);
        Arc::new(move |on| server.switch(on))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use hyper::header::{AUTHORIZATION, CONTENT_LENGTH, HOST, ORIGIN};

    const PORT: u16 = 4711;
    const TOKEN: &str = "c0ffee";

    fn good() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, format!("127.0.0.1:{PORT}").parse().unwrap());
        headers.insert(AUTHORIZATION, format!("Bearer {TOKEN}").parse().unwrap());
        headers
    }

    fn check(method: Method, path: &str, headers: &HeaderMap) -> Guard {
        precheck(&method, path, headers, PORT, TOKEN)
    }

    #[test]
    fn precheck_order() {
        assert_eq!(check(Method::POST, "/mcp", &good()), Guard::Ok);
        assert_eq!(check(Method::DELETE, "/mcp", &good()), Guard::Ok);
        let mut localhost = good();
        localhost.insert(HOST, format!("localhost:{PORT}").parse().unwrap());
        assert_eq!(check(Method::POST, "/mcp", &localhost), Guard::Ok);

        // Host first: a wrong host is refused even with every other mistake.
        let mut wrong_host = HeaderMap::new();
        wrong_host.insert(HOST, "evil.example:80".parse().unwrap());
        wrong_host.insert(ORIGIN, "http://evil".parse().unwrap());
        assert_eq!(
            check(Method::GET, "/x", &wrong_host),
            Guard::Refuse(403, "Host nicht erlaubt")
        );
        let mut other_port = good();
        other_port.insert(HOST, "127.0.0.1:1".parse().unwrap());
        assert_eq!(
            check(Method::POST, "/mcp", &other_port),
            Guard::Refuse(403, "Host nicht erlaubt")
        );
        let mut no_host = good();
        no_host.remove(HOST);
        assert_eq!(
            check(Method::POST, "/mcp", &no_host),
            Guard::Refuse(403, "Host nicht erlaubt")
        );

        // Then the token.
        let mut no_token = good();
        no_token.remove(AUTHORIZATION);
        assert_eq!(
            check(Method::GET, "/x", &no_token),
            Guard::Refuse(401, "Token fehlt oder ist falsch")
        );
        let mut wrong_token = good();
        wrong_token.insert(AUTHORIZATION, "Bearer c0ffef".parse().unwrap());
        assert_eq!(
            check(Method::POST, "/mcp", &wrong_token),
            Guard::Refuse(401, "Token fehlt oder ist falsch")
        );
        let mut not_bearer = good();
        not_bearer.insert(AUTHORIZATION, TOKEN.parse().unwrap());
        assert_eq!(
            check(Method::POST, "/mcp", &not_bearer),
            Guard::Refuse(401, "Token fehlt oder ist falsch")
        );

        // Then the path, then the method.
        assert_eq!(
            check(Method::GET, "/x", &good()),
            Guard::Refuse(404, "Nicht gefunden")
        );
        assert_eq!(
            check(Method::GET, "/mcp", &good()),
            Guard::Refuse(405, "Methode nicht erlaubt")
        );
        assert_eq!(
            check(Method::PUT, "/mcp", &good()),
            Guard::Refuse(405, "Methode nicht erlaubt")
        );

        // Last the announced body size.
        let mut huge = good();
        huge.insert(CONTENT_LENGTH, (MAX_BODY + 1).to_string().parse().unwrap());
        assert_eq!(
            check(Method::POST, "/mcp", &huge),
            Guard::Refuse(413, "Körper zu groß")
        );
        let mut largest = good();
        largest.insert(CONTENT_LENGTH, MAX_BODY.to_string().parse().unwrap());
        assert_eq!(check(Method::POST, "/mcp", &largest), Guard::Ok);
    }

    #[test]
    fn origin_is_refused_before_token() {
        // A web page in a browser: right host, any origin, no (or a guessed) token.
        for origin in ["http://evil", "http://127.0.0.1:1430", "null"] {
            let mut headers = good();
            headers.insert(ORIGIN, origin.parse().unwrap());
            assert_eq!(
                check(Method::POST, "/mcp", &headers),
                Guard::Refuse(403, "Origin nicht erlaubt"),
                "{origin}"
            );
            headers.remove(AUTHORIZATION);
            assert_eq!(
                check(Method::POST, "/mcp", &headers),
                Guard::Refuse(403, "Origin nicht erlaubt"),
                "{origin}"
            );
        }
    }

    #[test]
    fn control_is_idempotent() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let data = tempfile::tempdir().unwrap();
        let state = Arc::new(crate::state::AppState::default());
        let source: Arc<dyn StateSource> = Arc::new(Arc::clone(&state));
        let server = Arc::new(AppServer::new(
            source,
            Some(data.path().to_path_buf()),
            runtime.handle().clone(),
        ));
        let control = server.control();
        // Off while stopped is nothing.
        assert_eq!(control(false).unwrap(), None);
        let port = control(true).unwrap().unwrap();
        let token = discovery::read(data.path()).unwrap().token;
        // On while running keeps the server, its port and its token.
        assert_eq!(control(true).unwrap(), Some(port));
        assert_eq!(discovery::read(data.path()).unwrap().token, token);
        assert_eq!(control(false).unwrap(), None);
        assert!(discovery::read(data.path()).is_err());
        assert_eq!(control(false).unwrap(), None);
        // The hub's switch goes through the control.
        state.agents.set_server_control(server.control());
        state.agents.set_enabled(true).unwrap();
        let port = state.agents.status().port.unwrap();
        assert_eq!(discovery::read(data.path()).unwrap().port, port);
        server.shutdown();
        assert!(discovery::read(data.path()).is_err());
        // Without a data folder the server does not start.
        let source: Arc<dyn StateSource> = Arc::new(state);
        let homeless = AppServer::new(source, None, runtime.handle().clone());
        assert!(homeless.switch(true).is_err());
        assert_eq!(homeless.switch(false).unwrap(), None);
    }

    #[test]
    fn token_is_64_hex_and_fresh() {
        let (a, b) = (new_token().unwrap(), new_token().unwrap());
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn refusal_is_a_cmd_error() {
        let response = refusal(401, "Token fehlt oder ist falsch");
        assert_eq!(response.status(), 401);
        assert_eq!(
            response.headers()["content-type"].to_str().unwrap(),
            "application/json"
        );
    }
}

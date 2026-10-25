//! The bridge's HTTP side: `GET /health` and `POST /invoke/<command>` on loopback.
//!
//! Requests are checked in this order: `Host`, token, path, content type, `Origin`, body.
//! The checks keep other web pages (DNS rebinding, cross-site `fetch`) and other local
//! programs from driving the editor.

use std::io::Read;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use serde_json::{Value, json};
use tiny_http::{Header, Request, Response, Server};

use super::dispatch::{COMMANDS, DispatchError, Reply, dispatch};
use super::roots::Roots;
use crate::errors::CmdError;
use crate::mcp::http::same_secret;
use crate::state::AppState;

/// Name of the header that carries the shared secret.
const TOKEN_HEADER: &str = "X-G4-Bridge-Token";
/// Number of threads that answer requests.
const WORKERS: usize = 4;
/// Largest request body that is read.
const MAX_BODY: u64 = 64 * 1024 * 1024;

pub struct ServeConfig {
    /// Port on `127.0.0.1`; 0 picks a free one (see `BridgeServer::addr`).
    pub port: u16,
    pub token: String,
    /// The only `Origin` a browser request may carry (requests without one are accepted).
    pub origin: String,
    pub roots: Roots,
}

pub struct BridgeServer {
    pub addr: SocketAddr,
    server: Arc<Server>,
    stopping: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

struct Shared {
    state: AppState,
    roots: Roots,
    token: String,
    origin: String,
    port: u16,
}

impl BridgeServer {
    /// Binds `127.0.0.1:<port>` and starts the worker threads, all sharing one fresh `AppState`.
    pub fn start(config: ServeConfig) -> Result<BridgeServer, String> {
        let server = Server::http(("127.0.0.1", config.port))
            .map_err(|e| format!("Port {} nicht verfügbar: {e}", config.port))?;
        let addr = server
            .server_addr()
            .to_ip()
            .ok_or("Server hat keine IP-Adresse")?;
        let server = Arc::new(server);
        let stopping = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Shared {
            state: AppState::default(),
            roots: config.roots,
            token: config.token,
            origin: config.origin,
            port: addr.port(),
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
        })
    }

    /// Stops accepting requests and waits for the workers (requests in flight finish first).
    pub fn stop(self) {
        self.stopping.store(true, Ordering::SeqCst);
        // `unblock` frees one waiting `recv` per call.
        for _ in &self.workers {
            self.server.unblock();
        }
        for worker in self.workers {
            let _ = worker.join();
        }
    }
}

fn worker(server: &Server, stopping: &AtomicBool, shared: &Shared) {
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

fn handle(shared: &Shared, mut request: Request) {
    let (status, content_type, body) = answer(shared, &mut request);
    let mut response = Response::from_data(body).with_status_code(status);
    if let Ok(h) = Header::from_bytes("Content-Type", content_type) {
        response = response.with_header(h);
    }
    let _ = request.respond(response);
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
        body: Vec<u8>,
    }

    impl Reply1 {
        fn json(&self) -> Value {
            serde_json::from_slice(&self.body).unwrap()
        }
    }

    fn start_with(dir: &std::path::Path) -> BridgeServer {
        BridgeServer::start(ServeConfig {
            port: 0,
            token: TOKEN.into(),
            origin: ORIGIN.into(),
            roots: Roots::new(&[dir.to_path_buf()]).unwrap(),
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
}

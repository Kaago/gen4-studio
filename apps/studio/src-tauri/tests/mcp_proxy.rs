//! `studio mcp` without `--project`: the binary as a child process forwards an rmcp client's
//! stdio to the app's MCP server over Streamable HTTP. The server runs in the test process
//! (`HttpServer::start` on a synthetic project with the grant `write`), its discovery file
//! in a temporary app data folder that `--app-data` names.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use g4_kernel::test_support::{Synthetic, synthetic_project};
use rmcp::ServiceExt;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ClientCapabilities, ClientConfig, Implementation,
};
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use serde_json::{Value, json};
use studio_lib::AppState;
use studio_lib::dispatch::{Reply, dispatch};
#[cfg(unix)]
use studio_lib::mcp::discovery::Discovery;
use studio_lib::mcp::discovery::{self, APP_DATA_ENV};
use studio_lib::mcp::host::StateSource;
use studio_lib::mcp::http::HttpServer;
use studio_lib::mcp::settings::{GrantLevel, ProjectKey};
use studio_lib::policy::UserPolicy;

const STUDIO: &str = env!("CARGO_BIN_EXE_studio");
/// How long a test waits for the proxy or the server before it calls it hung.
const LIMIT: Duration = Duration::from_secs(60);
const NOT_RUNNING: &str = "Gen4 Studio is not running or \"Allow local agents\" is off; \
                           headless with --project <folder>";

type Client = RunningService<RoleClient, ClientConfig>;

struct App {
    /// Keeps the project folder while the test runs.
    _fixture: Synthetic,
    state: Arc<AppState>,
    policy: UserPolicy,
    data: tempfile::TempDir,
}

impl App {
    /// The synthetic project open in a fresh state, with the grant `write` for it and
    /// "Allow local agents" on (the switch only: the test starts the server itself).
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
        App {
            _fixture: fixture,
            state,
            policy,
            data: tempfile::tempdir().unwrap(),
        }
    }

    /// The server on the test's runtime.
    fn start(&self) -> HttpServer {
        let source: Arc<dyn StateSource> = Arc::new(Arc::clone(&self.state));
        HttpServer::start(source, self.data.path(), &tokio::runtime::Handle::current()).unwrap()
    }

    fn clients(&self) -> Vec<String> {
        let status = self.state.agents.status();
        status.connections.into_iter().map(|c| c.client).collect()
    }

    /// A command on the app's session, as the UI runs it.
    fn command(&self, name: &str, args: Value) -> Value {
        match dispatch(&self.state, &self.policy, name, args).unwrap() {
            Reply::Json(value) => value,
            Reply::Bytes(_) => panic!("{name}: expected JSON"),
        }
    }
}

/// `studio mcp --app-data <data>` (the environment's app data folder does not count).
fn proxy(data: &Path) -> tokio::process::Command {
    tokio::process::Command::new(STUDIO).configure(|cmd| {
        // The texts the tests compare are English whatever `LANG` of the host says.
        cmd.env("G4_LANG", "en")
            .arg("mcp")
            .arg("--app-data")
            .arg(data)
            .env_remove(APP_DATA_ENV);
    })
}

/// An rmcp client named "proxy-test" over the proxy's stdio.
async fn connect(data: &Path) -> Client {
    let transport = TokioChildProcess::new(proxy(data)).unwrap();
    let info = ClientConfig::new(
        ClientCapabilities::default(),
        Implementation::new("proxy-test", "1.0.0"),
    );
    tokio::time::timeout(LIMIT, info.serve(transport))
        .await
        .expect("initialize timed out")
        .expect("initialize failed")
}

async fn call_tool(client: &Client, tool: &str, args: Value) -> CallToolResult {
    let Value::Object(args) = args else {
        panic!("arguments must be an object");
    };
    let params = CallToolRequestParams::new(tool.to_owned()).with_arguments(args);
    tokio::time::timeout(LIMIT, client.call_tool(params))
        .await
        .expect("call timed out")
        .unwrap_or_else(|e| panic!("{tool} failed: {e}"))
}

/// The `result` of a successful call.
fn result(answer: CallToolResult) -> Value {
    assert_eq!(answer.is_error, Some(false), "{answer:?}");
    answer.structured_content.expect("structured content")["result"].clone()
}

async fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let start = Instant::now();
    while !done() {
        assert!(start.elapsed() < LIMIT, "timed out waiting for {what}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn proxy_reaches_the_running_app() {
    let app = App::new();
    let server = app.start();
    let client = connect(app.data.path()).await;
    let info = client.peer_info().expect("server info");
    assert_eq!(
        info.server_info.as_ref().expect("implementation").name,
        "gen4-studio"
    );
    let answer = call_tool(&client, "get_header", json!({ "id": 0 })).await;
    // The same header the app's session holds.
    assert_eq!(
        result(answer),
        app.command("get_header", json!({ "id": 0 }))
    );
    // The hub shows the client by the name it gave (through the proxy).
    assert_eq!(app.clients(), vec!["proxy-test".to_owned()]);
    client.cancel().await.unwrap();
    server.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn proxy_write_is_an_agent_step_in_the_app_session() {
    let app = App::new();
    let server = app.start();
    let client = connect(app.data.path()).await;
    let edit = json!({ "kind": "move", "index": 1,
                       "patch": { "Move": { "field": "power", "value": 65 } },
                       "label": "Stärke 65" });
    result(call_tool(&client, "data_edit", edit).await);
    let history = app.command("history", json!({}));
    assert_eq!(
        history["undo"].as_array().unwrap().last(),
        Some(&json!("Stärke 65")),
        "{history}"
    );
    assert_eq!(
        history["undo_agents"].as_array().unwrap().last(),
        Some(&json!("proxy-test")),
        "{history}"
    );
    client.cancel().await.unwrap();
    server.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn closing_stdin_ends_the_session() {
    let app = App::new();
    let server = app.start();
    let client = connect(app.data.path()).await;
    result(call_tool(&client, "get_header", json!({ "id": 0 })).await);
    assert_eq!(app.clients(), vec!["proxy-test".to_owned()]);
    // rmcp closes the child's stdin and waits for it to exit (it kills it only after a
    // grace period): the proxy ends the session with DELETE on its way out.
    client.cancel().await.unwrap();
    wait_until("the connection to end", || app.clients().is_empty()).await;
    server.stop();
}

/// Answers the hub's next question with `answer` from a thread of its own (a client the user
/// disconnected is asked again on its next call).
fn answer_next_question(
    state: &Arc<AppState>,
    answer: &'static str,
) -> std::thread::JoinHandle<()> {
    let state = Arc::clone(state);
    std::thread::spawn(move || {
        let start = Instant::now();
        loop {
            if let Some(question) = state.agents.status().question {
                state.agents.answer(question.id, answer).unwrap();
                return;
            }
            assert!(start.elapsed() < LIMIT, "no question raised");
            std::thread::sleep(Duration::from_millis(10));
        }
    })
}

/// After "Trennen" the proxy opens a new session with the client's handshake: the next call
/// works (once the user let the client in again) and the hub shows it connected again.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ended_session_is_renewed() {
    let app = App::new();
    let server = app.start();
    let client = connect(app.data.path()).await;
    result(call_tool(&client, "get_header", json!({ "id": 0 })).await);
    app.state.agents.disconnect_all();
    wait_until("the connection to end", || app.clients().is_empty()).await;
    let answering = answer_next_question(&app.state, "write");
    let answer = call_tool(&client, "get_header", json!({ "id": 0 })).await;
    assert_eq!(
        result(answer),
        app.command("get_header", json!({ "id": 0 }))
    );
    answering.join().unwrap();
    assert_eq!(app.clients(), vec!["proxy-test".to_owned()]);
    client.cancel().await.unwrap();
    wait_until("the connection to end", || app.clients().is_empty()).await;
    server.stop();
}

/// The switch off and on again (or a new start of the app): new port, new token, same
/// folder. The proxy reads the discovery file again and carries on.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn restarted_server_is_found_again() {
    let app = App::new();
    let server = app.start();
    let old = discovery::read(app.data.path()).unwrap();
    let client = connect(app.data.path()).await;
    result(call_tool(&client, "get_header", json!({ "id": 0 })).await);
    server.stop();
    wait_until("the connection to end", || app.clients().is_empty()).await;
    let server = app.start();
    assert_ne!(discovery::read(app.data.path()).unwrap().token, old.token);
    let answer = call_tool(&client, "get_header", json!({ "id": 0 })).await;
    assert_eq!(
        result(answer),
        app.command("get_header", json!({ "id": 0 }))
    );
    assert_eq!(app.clients(), vec!["proxy-test".to_owned()]);
    client.cancel().await.unwrap();
    wait_until("the connection to end", || app.clients().is_empty()).await;
    server.stop();
}

/// The app is gone for good (switched off, no new server): a call is an error answer within
/// a few seconds, not a hang.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn app_gone_is_an_error_answer() {
    let app = App::new();
    let server = app.start();
    let client = connect(app.data.path()).await;
    result(call_tool(&client, "get_header", json!({ "id": 0 })).await);
    server.stop();
    let params = CallToolRequestParams::new("get_header".to_owned())
        .with_arguments(json!({ "id": 0 }).as_object().unwrap().clone());
    let error = tokio::time::timeout(Duration::from_secs(10), client.call_tool(params))
        .await
        .expect("call timed out")
        .expect_err("the app is gone");
    let text = error.to_string();
    assert!(text.contains("Gen4 Studio:"), "{text}");
    // Closing stdin still ends the proxy promptly (its DELETE has nowhere to go).
    tokio::time::timeout(Duration::from_secs(10), client.cancel())
        .await
        .expect("the proxy did not end")
        .unwrap();
}

/// A client that sends `initialize` a second time gets a new session; the proxy ends the old
/// one first, so the hub does not keep a stale connection.
#[test]
fn second_initialize_replaces_the_session() {
    use std::io::{BufRead, BufReader, Write};
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let _entered = runtime.enter();
    let app = App::new();
    let server = app.start();
    let mut child = Command::new(STUDIO)
        .env("G4_LANG", "en")
        .arg("mcp")
        .arg("--app-data")
        .arg(app.data.path())
        .env_remove(APP_DATA_ENV)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    for round in 1..=2 {
        let init = json!({ "jsonrpc": "2.0", "id": round, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": { "name": "proxy-test", "version": "1" } } });
        writeln!(stdin, "{init}").unwrap();
        stdin.flush().unwrap();
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        let answer: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(answer["id"], round, "{answer}");
        assert_eq!(answer["result"]["serverInfo"]["name"], "gen4-studio");
        let start = Instant::now();
        while app.clients().len() != 1 {
            assert!(
                start.elapsed() < LIMIT,
                "round {round}: {:?}",
                app.clients()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    drop(stdin);
    assert!(child.wait().unwrap().success());
    let start = Instant::now();
    while !app.clients().is_empty() {
        assert!(start.elapsed() < LIMIT, "{:?}", app.clients());
        std::thread::sleep(Duration::from_millis(20));
    }
    server.stop();
}

/// Runs `studio mcp` with `args` and no client; answers the exit code and stderr.
fn run_bad(args: &[&str]) -> (Option<i32>, String) {
    let output = Command::new(STUDIO)
        .env("G4_LANG", "en")
        .arg("mcp")
        .args(args)
        .env_remove(APP_DATA_ENV)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn missing_discovery_file_is_exit_2() {
    let data = tempfile::tempdir().unwrap();
    let (code, stderr) = run_bad(&["--app-data", data.path().to_str().unwrap()]);
    assert_eq!(code, Some(2), "{stderr}");
    assert!(stderr.contains(NOT_RUNNING), "{stderr}");
}

/// Elsewhere the PID is not checked (a dead app shows as a refused connection).
#[cfg(unix)]
#[test]
fn stale_discovery_file_is_exit_2() {
    let data = tempfile::tempdir().unwrap();
    let mut stale = Discovery::new(1, "00");
    stale.pid = 999_999;
    discovery::write(data.path(), &stale).unwrap();
    let (code, stderr) = run_bad(&["--app-data", data.path().to_str().unwrap()]);
    assert_eq!(code, Some(2), "{stderr}");
    assert!(stderr.contains(NOT_RUNNING), "{stderr}");
}

/// Access is the app's to decide: the headless flags do not go with the proxy.
#[test]
fn headless_flags_need_project() {
    let data = tempfile::tempdir().unwrap();
    let dir = data.path().to_str().unwrap();
    for flag in ["--read-only", "--allow-irreversible", "--expert"] {
        let (code, stderr) = run_bad(&["--app-data", dir, flag]);
        assert_eq!(code, Some(2), "{flag}: {stderr}");
        assert!(
            stderr.contains(&format!("{flag} applies only headless with --project")),
            "{stderr}"
        );
    }
    for flag in ["--emulator", "--allow-dir"] {
        let (code, stderr) = run_bad(&["--app-data", dir, flag, dir]);
        assert_eq!(code, Some(2), "{flag}: {stderr}");
        assert!(
            stderr.contains(&format!("{flag} applies only headless with --project")),
            "{stderr}"
        );
    }
}

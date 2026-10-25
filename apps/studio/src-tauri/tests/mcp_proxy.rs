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
use studio_lib::mcp::discovery::{self, APP_DATA_ENV, Discovery};
use studio_lib::mcp::host::StateSource;
use studio_lib::mcp::http::HttpServer;
use studio_lib::mcp::settings::{GrantLevel, ProjectKey};
use studio_lib::policy::UserPolicy;

const STUDIO: &str = env!("CARGO_BIN_EXE_studio");
/// How long a test waits for the proxy or the server before it calls it hung.
const LIMIT: Duration = Duration::from_secs(60);
const NOT_RUNNING: &str = "Gen4 Studio läuft nicht oder „Lokale Agenten zulassen“ ist aus; \
                           kopflos mit --project <Ordner>";

type Client = RunningService<RoleClient, ClientConfig>;

struct App {
    /// Keeps the project folder while the test runs.
    _fixture: Synthetic,
    state: Arc<AppState>,
    policy: UserPolicy,
    data: tempfile::TempDir,
}

impl App {
    /// The synthetic project open in a fresh state, with the grant `write` for it.
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
        cmd.arg("mcp")
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

/// After "Trennen" the session is gone: a call through the proxy is an error, not a hang.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ended_session_is_an_error_answer() {
    let app = App::new();
    let server = app.start();
    let client = connect(app.data.path()).await;
    result(call_tool(&client, "get_header", json!({ "id": 0 })).await);
    app.state.agents.disconnect_all();
    wait_until("the connection to end", || app.clients().is_empty()).await;
    let params = CallToolRequestParams::new("get_header".to_owned())
        .with_arguments(json!({ "id": 0 }).as_object().unwrap().clone());
    let error = tokio::time::timeout(LIMIT, client.call_tool(params))
        .await
        .expect("call timed out")
        .expect_err("the session ended");
    let text = error.to_string();
    assert!(text.contains("Gen4 Studio: HTTP 404"), "{text}");
    client.cancel().await.unwrap();
    server.stop();
}

/// Runs `studio mcp` with `args` and no client; answers the exit code and stderr.
fn run_bad(args: &[&str]) -> (Option<i32>, String) {
    let output = Command::new(STUDIO)
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
            stderr.contains(&format!("{flag} gilt nur kopflos mit --project")),
            "{stderr}"
        );
    }
    for flag in ["--emulator", "--allow-dir"] {
        let (code, stderr) = run_bad(&["--app-data", dir, flag, dir]);
        assert_eq!(code, Some(2), "{flag}: {stderr}");
        assert!(
            stderr.contains(&format!("{flag} gilt nur kopflos mit --project")),
            "{stderr}"
        );
    }
}

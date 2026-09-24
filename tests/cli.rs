//! Drives the built binary against an in-process mock of the Bugsink API. Every test gets its
//! own config directory and the plaintext store, so nothing touches a real keystore or account.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct Recorded {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Option<Value>,
}

type Route = (&'static str, &'static str, u16, Value);

struct Mock {
    url: String,
    log: Arc<Mutex<Vec<Recorded>>>,
}

impl Mock {
    /// Routes are (method, path-with-query, status, body). Unmatched requests get a 404.
    fn start(routes: Vec<Route>) -> Mock {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}/api/canonical/0/");
        let log = Arc::new(Mutex::new(Vec::new()));
        let log2 = log.clone();

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let routes = routes.clone();
                let log = log2.clone();

                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let mut parts = line.split_whitespace();
                    let method = parts.next().unwrap_or("").to_string();
                    let raw_path = parts.next().unwrap_or("");
                    let path = raw_path.trim_start_matches("/api/canonical/0/").to_string();

                    let mut headers = Vec::new();
                    let mut len = 0usize;
                    let mut is_markdown_accept = false;

                    loop {
                        let mut h = String::new();
                        reader.read_line(&mut h).unwrap();
                        let h = h.trim_end();
                        if h.is_empty() {
                            break;
                        }
                        if let Some((k, v)) = h.split_once(':') {
                            let (k, v) = (k.trim().to_lowercase(), v.trim().to_string());
                            if k == "content-length" {
                                len = v.parse().unwrap_or(0);
                            }
                            if k == "accept" && v.contains("text/markdown") {
                                is_markdown_accept = true;
                            }
                            headers.push((k, v));
                        }
                    }

                    let mut buf = vec![0; len];
                    reader.read_exact(&mut buf).unwrap();
                    let body = (len > 0).then(|| serde_json::from_slice(&buf).unwrap_or(Value::Null));
                    log.lock().unwrap().push(Recorded { method: method.clone(), path: path.clone(), headers, body });

                    let matched = routes.iter().find(|(m, p, _, _)| *m == method && *p == path);

                    let (status, resp) = if let Some((_, _, s, b)) = matched {
                        (*s, b.clone())
                    } else {
                        (404, json!({"detail": "Not found."}))
                    };

                    let (content_type, text) = if is_markdown_accept && resp.is_string() {
                        ("text/markdown", resp.as_str().unwrap().to_string())
                    } else if status == 204 {
                        ("application/json", String::new())
                    } else {
                        ("application/json", resp.to_string())
                    };

                    let _ = write!(
                        stream,
                        "HTTP/1.1 {status} X\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                        text.len()
                    );
                });
            }
        });

        Mock { url, log }
    }

    fn requests(&self) -> Vec<Recorded> {
        self.log.lock().unwrap().clone()
    }

    fn last(&self, method: &str) -> Recorded {
        self.requests().into_iter().rev().find(|r| r.method == method).expect("no such request")
    }
}

struct Env {
    dir: PathBuf,
    api: String,
}

impl Env {
    fn new(mock: &Mock) -> Env {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "bugsink-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Env { dir, api: mock.url.clone() }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_bugsink"))
            .args(args)
            .env("BUGSINK_CONFIG_DIR", &self.dir)
            .env("BUGSINK_SECRET_STORE", "plaintext")
            .env("BUGSINK_ALLOW_PLAINTEXT_STORE", "1")
            .output()
            .unwrap()
    }

    fn json(&self, args: &[&str]) -> (i32, Value, Value) {
        let mut all = args.to_vec();
        all.push("--json");
        let out = self.run(&all);
        let parse = |b: &[u8]| {
            let s = String::from_utf8_lossy(b);
            let s = s.lines().filter(|l| !l.starts_with("warning:")).collect::<Vec<_>>().join("\n");
            serde_json::from_str(&s).unwrap_or(Value::Null)
        };
        (out.status.code().unwrap_or(1), parse(&out.stdout), parse(&out.stderr))
    }

    /// Adds account `work` with key `token_test` and endpoint `mock.url`.
    fn with_account(self) -> Env {
        let (code, out, err) =
            self.json(&["accounts", "add", "work", "--endpoint", &self.api, "--api-token", "token_test"]);
        assert_eq!(code, 0, "{err}");
        assert_eq!(out["status"], "added");
        self
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn probe_route() -> Route {
    ("GET", "projects/?limit=1", 200, json!({"results": [{"id": 1, "name": "Default", "slug": "default"}]}))
}

#[test]
fn agent_readme_as_data() {
    let mock = Mock::start(vec![]);
    let env = Env::new(&mock);
    let (code, out, _) = env.json(&["agent-readme"]);
    assert_eq!(code, 0);
    assert_eq!(out["tool"], "bugsink");
    assert_eq!(out["exitCodes"]["7"], "no_account - run bugsink accounts list");

    let md = String::from_utf8(env.run(&["agent-readme"]).stdout).unwrap();
    assert!(md.contains("# bugsink — agent operating manual"));
}

#[test]
fn accounts_lifecycle() {
    let mock = Mock::start(vec![probe_route(), probe_route()]);
    let env = Env::new(&mock).with_account();

    assert_eq!(mock.last("GET").headers.iter().find(|(k, _)| k == "authorization").unwrap().1, "Bearer token_test");

    let (code, out, _) = env.json(&["accounts", "list"]);
    assert_eq!(code, 0);
    assert_eq!(out["count"], 1);
    assert_eq!(out["accounts"][0]["keyStatus"], "stored");
    assert_eq!(out["secretStore"], "plaintext");

    let (_, out, _) = env.json(&["accounts", "list", "--check"]);
    assert_eq!(out["accounts"][0]["keyStatus"], "valid");

    // Adding existing account requires --force
    let (code, _, err) = env.json(&["accounts", "add", "WORK", "--endpoint", &env.api, "--api-token", "new_token"]);
    assert_eq!(code, 6);
    assert!(err["remediation"].as_str().unwrap().contains("--force"));

    let (code, out, _) = env.json(&["accounts", "test", "Work"]);
    assert_eq!(code, 0);
    assert_eq!(out["keyStatus"], "valid");

    // Remove without --yes in non-terminal fails
    let (code, _, err) = env.json(&["accounts", "remove", "work"]);
    assert_eq!(code, 6);
    assert_eq!(err["remediation"], "bugsink accounts remove work --yes");

    let (code, out, _) = env.json(&["accounts", "remove", "work", "--yes"]);
    assert_eq!(code, 0);
    assert_eq!(out["status"], "removed");

    let (_, out, _) = env.json(&["accounts", "list"]);
    assert_eq!(out["count"], 0);
}

#[test]
fn api_token_from_stdin() {
    let mock = Mock::start(vec![probe_route()]);
    let env = Env::new(&mock);
    let mut child = Command::new(env!("CARGO_BIN_EXE_bugsink"))
        .args(["accounts", "add", "piped", "--endpoint", &env.api, "--api-token-stdin", "--json"])
        .env("BUGSINK_CONFIG_DIR", &env.dir)
        .env("BUGSINK_SECRET_STORE", "plaintext")
        .env("BUGSINK_ALLOW_PLAINTEXT_STORE", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child.stdin.take().unwrap().write_all(b"piped_token_123\n").unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));

    let auth = mock.last("GET").headers.into_iter().find(|(k, _)| k == "authorization").unwrap().1;
    assert_eq!(auth, "Bearer piped_token_123");
}

#[test]
fn config_is_readable_yaml_without_secrets() {
    let mock = Mock::start(vec![probe_route()]);
    let env = Env::new(&mock).with_account();
    let yaml = std::fs::read_to_string(env.dir.join("config.yaml")).unwrap();
    assert!(yaml.contains("work:"), "{yaml}");
    assert!(!yaml.contains("token_test"), "the secret leaked into config.yaml");
}

#[test]
fn account_is_required_when_unspecified() {
    let mock = Mock::start(vec![probe_route()]);
    let env = Env::new(&mock).with_account();

    let (code, _, err) = env.json(&["projects", "list"]);
    assert_eq!(code, 7);
    assert_eq!(err["code"], "no_account");
    assert!(err["detail"].as_str().unwrap().contains("work"));

    let (code, _, err) = env.json(&["projects", "list", "-a", "nope"]);
    assert_eq!(code, 7);
    assert_eq!(err["remediation"], "bugsink accounts list");
}

#[test]
fn http_errors_map_to_exit_codes() {
    let mock = Mock::start(vec![
        probe_route(),
        ("POST", "projects/", 401, json!({"detail": "Malformed Bearer token, must be 40 lowercase hex chars."})),
        ("GET", "teams/", 429, json!({"detail": "Request was throttled."})),
        ("GET", "teams/00000000-0000-0000-0000-000000000999/", 404, json!({"detail": "Not found."})),
        ("GET", "projects/", 500, json!({"detail": "Internal Server Error"})),
    ]);
    let env = Env::new(&mock).with_account();

    let (code, _, err) = env.json(&[
        "projects",
        "create",
        "--team",
        "00000000-0000-0000-0000-000000000000",
        "--name",
        "demo",
        "-a",
        "work",
    ]);
    assert_eq!(code, 3);
    assert!(err["error"].as_str().unwrap().contains("Malformed Bearer token"));

    assert_eq!(env.json(&["teams", "list", "-a", "work"]).0, 5);
    assert_eq!(env.json(&["teams", "get", "00000000-0000-0000-0000-000000000999", "-a", "work"]).0, 4);
    assert_eq!(env.json(&["projects", "list", "-a", "work"]).0, 2);
}

#[test]
fn parse_errors_are_envelopes() {
    let mock = Mock::start(vec![]);
    let env = Env::new(&mock);

    let (code, _, err) = env.json(&["issues", "list", "-a", "work"]);
    assert_eq!(code, 6);
    assert_eq!(err["code"], "invalid_input");
    assert!(err["error"].as_str().unwrap().contains("--project"), "{err}");

    let out = env.run(&["--help"]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn yaml_is_the_default() {
    let mock = Mock::start(vec![
        probe_route(),
        ("GET", "projects/", 200, json!({"results": [{"id": 1, "name": "Main Project", "slug": "main-project"}]})),
    ]);
    let env = Env::new(&mock).with_account();
    let out = env.run(&["projects", "list", "-a", "work"]);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("id: 1"), "{stdout}");
    assert!(stdout.contains("name: Main Project"), "{stdout}");
}

#[test]
fn issues_operations_and_stacktrace() {
    let mock = Mock::start(vec![
        probe_route(),
        // projects/ for resolving project slug "demo"
        ("GET", "projects/", 200, json!({"results": [{"id": 42, "name": "Demo", "slug": "demo"}]})),
        // issues list for project 42
        (
            "GET",
            "issues/?project=42",
            200,
            json!({
                "results": [
                    {"id": "issue-1", "is_resolved": false, "is_muted": false},
                    {"id": "issue-2", "is_resolved": true, "is_muted": false},
                    {"id": "issue-3", "is_resolved": false, "is_muted": true},
                ]
            }),
        ),
        // events/?issue=issue-1&order=desc for resolve_latest_event_id
        ("GET", "events/?issue=issue-1&order=desc", 200, json!({"results": [{"id": "ev-uuid-99"}]})),
        // markdown stacktrace
        (
            "GET",
            "events/ev-uuid-99/stacktrace/",
            200,
            json!(
                "```\nTraceback (most recent call last):\n  File 'app.py', line 10\n    raise ValueError('broken')\n```"
            ),
        ),
        // resolve issue
        ("POST", "issues/issue-1/resolve/", 200, json!({"id": "issue-1", "is_resolved": true})),
        // mute issue
        ("POST", "issues/issue-1/mute/", 200, json!({"id": "issue-1", "is_muted": true})),
        // comment on issue
        ("POST", "issue-comments/", 201, json!({"id": 1, "comment": "fixed"})),
        // delete issue
        ("DELETE", "issues/issue-1/", 204, json!(null)),
    ]);
    let env = Env::new(&mock).with_account();

    // 1. List with status open (filters client-side)
    let (code, out, _) = env.json(&["issues", "list", "--project", "demo", "--status", "open", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out.as_array().unwrap().len(), 1);
    assert_eq!(out[0]["id"], "issue-1");

    // 2. Stacktrace (markdown text in stdout)
    let out = env.run(&["issues", "stacktrace", "issue-1", "-a", "work"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Traceback"));

    // Stacktrace as JSON
    let (code, out, _) = env.json(&["issues", "stacktrace", "issue-1", "-a", "work"]);
    assert_eq!(code, 0);
    assert!(out["stacktrace"].as_str().unwrap().contains("Traceback"));

    // 3. Resolve
    let (code, out, _) = env.json(&["issues", "resolve", "issue-1", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["status"], "resolved");

    // 4. Mute
    let (code, out, _) = env.json(&["issues", "mute", "issue-1", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["status"], "muted");

    // 5. Comment
    let (code, out, _) = env.json(&["issues", "comment", "issue-1", "--text", "fixed", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["status"], "commented");

    // 6. Delete
    let (code, out, _) = env.json(&["issues", "delete", "issue-1", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["status"], "deleted");
}

#[test]
fn login_command_lifecycle() {
    let mock = Mock::start(vec![probe_route(), probe_route(), probe_route()]);
    let env = Env::new(&mock);

    // Login with default name "default"
    let (code, out, err) = env.json(&["login", "--endpoint", &env.api, "--api-token", "login_token_default"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out["status"], "logged_in");
    assert_eq!(out["name"], "default");

    // Re-login without --force fails
    let (code, _, err) = env.json(&["login", "--endpoint", &env.api, "--api-token", "new_token"]);
    assert_eq!(code, 6);
    assert!(err["remediation"].as_str().unwrap().contains("--force"));

    // Re-login with --force succeeds
    let (code, out, _) = env.json(&["login", "--endpoint", &env.api, "--api-token", "new_token", "--force"]);
    assert_eq!(code, 0);
    assert_eq!(out["status"], "logged_in");

    // Login with custom name
    let (code, out, _) = env.json(&["login", "staging", "--endpoint", &env.api, "--api-token", "token_staging"]);
    assert_eq!(code, 0);
    assert_eq!(out["name"], "staging");
}

#[test]
fn projects_and_teams_crud() {
    let mock = Mock::start(vec![
        probe_route(),
        ("POST", "teams/", 201, json!({"id": "team-123", "name": "Core Team", "visibility": "discoverable"})),
        ("PATCH", "teams/team-123/", 200, json!({"id": "team-123", "name": "Renamed Team"})),
        ("GET", "teams/", 200, json!({"results": [{"id": "team-123", "name": "Renamed Team"}]})),
        ("POST", "projects/", 201, json!({"id": 101, "name": "Backend", "team": "team-123"})),
        (
            "GET",
            "projects/101/?expand=team",
            200,
            json!({"id": 101, "name": "Backend", "team": {"id": "team-123", "name": "Renamed Team"}}),
        ),
        (
            "PATCH",
            "projects/101/",
            200,
            json!({"id": 101, "name": "Backend Service", "retention_max_event_count": 50000}),
        ),
    ]);
    let env = Env::new(&mock).with_account();

    // 1. Create Team
    let (code, out, _) = env.json(&["teams", "create", "--name", "Core Team", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["id"], "team-123");

    // 2. Update Team
    let (code, out, err) = env.json(&["teams", "update", "team-123", "--name", "Renamed Team", "-a", "work"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out["name"], "Renamed Team");

    // 3. Create Project with team resolution
    let (code, out, _) = env.json(&["projects", "create", "--team", "Renamed Team", "--name", "Backend", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["id"], 101);

    // 4. Get Project with expand-team
    let (code, out, _) = env.json(&["projects", "get", "101", "--expand-team", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["team"]["name"], "Renamed Team");

    // 5. Update Project
    let (code, out, _) =
        env.json(&["projects", "update", "101", "--name", "Backend Service", "--retention", "50000", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["name"], "Backend Service");
}

#[test]
fn releases_and_events_crud() {
    let mock = Mock::start(vec![
        probe_route(),
        // Create Release
        ("POST", "releases/", 201, json!({"id": "rel-1", "version": "v1.0.0", "project": 1})),
        // List Releases
        ("GET", "releases/?project=1", 200, json!({"results": [{"id": "rel-1", "version": "v1.0.0"}]})),
        // Get Release
        ("GET", "releases/rel-1/", 200, json!({"id": "rel-1", "version": "v1.0.0"})),
        // Events List
        (
            "GET",
            "events/?issue=MYPROJECT-1&order=desc",
            200,
            json!({"results": [{"id": "ev-10", "data": {"runtime": "python"}}]}),
        ),
        // Events Get with --no-data
        (
            "GET",
            "events/ev-10/",
            200,
            json!({"id": "ev-10", "data": {"runtime": "python"}, "message": "ZeroDivisionError"}),
        ),
        // Events Stacktrace
        ("GET", "events/ev-10/stacktrace/", 200, json!("```\nZeroDivisionError: division by zero\n```")),
    ]);
    let env = Env::new(&mock).with_account();

    // 1. Create release
    let (code, out, _) = env.json(&["releases", "create", "--project", "1", "--version", "v1.0.0", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["version"], "v1.0.0");

    // 2. List releases
    let (code, out, _) = env.json(&["releases", "list", "--project", "1", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out.as_array().unwrap().len(), 1);

    // 3. Get release
    let (code, out, _) = env.json(&["releases", "get", "rel-1", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["id"], "rel-1");

    // 4. List events
    let (code, out, _) = env.json(&["events", "list", "--issue", "MYPROJECT-1", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out.as_array().unwrap().len(), 1);

    // 5. Get event with --no-data
    let (code, out, _) = env.json(&["events", "get", "ev-10", "--no-data", "-a", "work"]);
    assert_eq!(code, 0);
    assert_eq!(out["id"], "ev-10");
    assert!(out.get("data").is_none());

    // 6. Event stacktrace
    let (code, out, _) = env.json(&["events", "stacktrace", "ev-10", "-a", "work"]);
    assert_eq!(code, 0);
    assert!(out["stacktrace"].as_str().unwrap().contains("ZeroDivisionError"));
}

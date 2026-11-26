//! Test-only (S131-4): a participant home (watane host 19.7, `Mode::Home`) whose app is a fake
//! `Confirm`, for the stage C tests of `console/test/host/stage_c.mjs` and `load_b.mjs --home`.
//!
//! `stage-c-home --home <dir> --fake-claude <path> [--port <n>] [--grant-all] <mod folder>...`
//! opens the home, installs the folders (approved, as the app would after its consent view),
//! serves it, and prints the host's ready JSON line. The app approves every install, names
//! `<path>` as the participant's `claude` (AccountSource::claude_binary, host 19.9), and holds
//! each consent question until the test answers it (`--grant-all`: grants each at once).
//!
//! stdout, one JSON line each: `{"event":"asked","i":n,"what":..,"origin":..,"via":..,"profile":..}`
//! per question, `{"event":"dropped","i":n}` when the host drops an unanswered one, and
//! `{"event":"done","op":..,"error":null|text}` per command.
//! stdin, one JSON line each: `{"op":"answer","i":n,"answer":"granted"|"refused"}`,
//! `{"op":"withdraw","local":..,"what":..}`, `{"op":"consents","local":..}` (done carries `consents`),
//! `{"op":"quit"}`.
use std::future::Future;
use std::io::Write;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncBufReadExt;
use watane_host::{
    AccountError, AccountSource, Answer, Confirm, Consent, ConsentAnswer, ConsentRequest, Host,
    InstallRequest, JudgeAccount, Limits, LocalName, Mode, Requester, Source,
};

type Held = tokio::sync::oneshot::Sender<ConsentAnswer>;

fn emit(v: serde_json::Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}

struct Accounts(PathBuf);
impl AccountSource for Accounts {
    fn openai_token(&self) -> Pin<Box<dyn Future<Output = Result<String, AccountError>> + Send>> {
        Box::pin(async { Err(AccountError::NoAccount) })
    }
    fn judge(&self, _: &str) -> Result<JudgeAccount, AccountError> {
        Err(AccountError::NoAccount)
    }
    fn claude_binary(&self) -> Result<PathBuf, AccountError> {
        Ok(self.0.clone())
    }
}

struct App {
    claude: PathBuf,
    grant_all: bool,
    held: Mutex<Vec<Option<Held>>>,
}
/// Reports a question the host dropped before the test answered it (a withdrawal, a cancel).
struct Dropped(usize, Arc<App>);
impl Drop for Dropped {
    fn drop(&mut self) {
        let open = self.1.held.lock().unwrap()[self.0].take().is_some();
        if open {
            emit(serde_json::json!({"event": "dropped", "i": self.0}));
        }
    }
}
struct Fake(Arc<App>);
impl Confirm for Fake {
    fn install(&self, _: InstallRequest) -> Pin<Box<dyn Future<Output = Answer> + Send>> {
        Box::pin(async { Answer::Approve { withhold: vec![] } })
    }
    fn accounts(&self) -> Option<Arc<dyn AccountSource>> {
        Some(Arc::new(Accounts(self.0.claude.clone())))
    }
    fn consent(
        &self,
        request: ConsentRequest,
    ) -> Pin<Box<dyn Future<Output = ConsentAnswer> + Send>> {
        let app = Arc::clone(&self.0);
        let (tx, rx) = tokio::sync::oneshot::channel();
        let i = {
            let mut held = app.held.lock().unwrap();
            held.push(Some(tx));
            held.len() - 1
        };
        emit(serde_json::json!({
            "event": "asked", "i": i, "what": request.consent.to_string(),
            "origin": request.origin.to_string(), "via": request.via.to_string(),
            "profile": request.profile,
        }));
        if app.grant_all {
            let tx = app.held.lock().unwrap()[i].take();
            return Box::pin(async move {
                drop(tx);
                ConsentAnswer::Granted
            });
        }
        Box::pin(async move {
            let _watch = Dropped(i, Arc::clone(&app));
            rx.await.unwrap_or(ConsentAnswer::Refused)
        })
    }
}

struct Args {
    home: PathBuf,
    claude: PathBuf,
    port: u16,
    grant_all: bool,
    folders: Vec<PathBuf>,
}
fn args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let (mut home, mut claude, mut port, mut grant_all, mut folders) = (None, None, 0, false, vec![]);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--home" => home = it.next().map(PathBuf::from),
            "--fake-claude" => claude = it.next().map(PathBuf::from),
            "--port" => port = it.next().and_then(|p| p.parse().ok()).ok_or("--port <n>")?,
            "--grant-all" => grant_all = true,
            _ if a.starts_with("--") => return Err(format!("unknown flag {a}")),
            _ => folders.push(PathBuf::from(a)),
        }
    }
    Ok(Args {
        home: home.ok_or("--home <dir> is required")?,
        claude: claude.ok_or("--fake-claude <path> is required")?,
        port,
        grant_all,
        folders,
    })
}

fn command(host: &Host, app: &App, line: &str) -> (String, Result<serde_json::Value, String>) {
    let v: serde_json::Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return ("?".into(), Err(format!("not JSON: {e}"))),
    };
    let op = v["op"].as_str().unwrap_or("?").to_owned();
    let local = || LocalName::parse(v["local"].as_str().unwrap_or("")).map_err(|e| e.to_string());
    let result = match op.as_str() {
        "answer" => (|| {
            let i = v["i"].as_u64().ok_or("no i")? as usize;
            let answer = match v["answer"].as_str() {
                Some("granted") => ConsentAnswer::Granted,
                Some("refused") => ConsentAnswer::Refused,
                _ => return Err("answer is granted or refused".to_owned()),
            };
            let tx = app.held.lock().unwrap().get_mut(i).and_then(Option::take);
            let tx = tx.ok_or(format!("question {i} is not open"))?;
            tx.send(answer).map_err(|_| format!("question {i} was dropped"))?;
            Ok(serde_json::Value::Null)
        })(),
        "withdraw" => (|| {
            let consent = Consent::parse(v["what"].as_str().unwrap_or("")).map_err(|e| e.to_string())?;
            host.withdraw_consent(&local()?, &consent).map_err(|e| e.to_string())?;
            Ok(serde_json::Value::Null)
        })(),
        "consents" => (|| {
            let local = local()?;
            let text = std::fs::read_to_string(host_home(host).join("installed.json")).map_err(|e| e.to_string())?;
            let all: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            let m = all["mods"].as_array().into_iter().flatten().find(|m| m["local"] == local.to_string());
            Ok(m.and_then(|m| m.get("consents").cloned()).unwrap_or(serde_json::json!([])))
        })(),
        _ => Err(format!("unknown op {op}")),
    };
    (op, result)
}
fn host_home(_: &Host) -> PathBuf {
    HOME.get().cloned().unwrap_or_default()
}
static HOME: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

#[tokio::main]
async fn main() {
    let a = match args() {
        Ok(a) => a,
        Err(e) => {
            emit(serde_json::json!({"event": "error", "message": e}));
            std::process::exit(64);
        }
    };
    let _ = HOME.set(a.home.clone());
    let app = Arc::new(App { claude: a.claude, grant_all: a.grant_all, held: Mutex::default() });
    let fail = |what: &str, e: &dyn std::fmt::Display| -> ! {
        emit(serde_json::json!({"event": "error", "message": format!("{what}: {e}")}));
        std::process::exit(1)
    };
    let confirm: Arc<dyn Confirm> = Arc::new(Fake(Arc::clone(&app)));
    let host = Host::open(&a.home, Mode::Home { app: Some(confirm) }, Limits::default())
        .unwrap_or_else(|e| fail("open", &e));
    if !a.folders.is_empty() {
        let plan = host
            .plan(a.folders.iter().cloned().map(Source::Folder).collect())
            .unwrap_or_else(|e| fail("plan", &e));
        host.install(plan, Requester::App).await.unwrap_or_else(|e| fail("install", &e));
    }
    let served = host.serve(a.port, axum::Router::new()).await.unwrap_or_else(|e| fail("serve", &e));
    emit(serde_json::to_value(served.ready()).unwrap_or_else(|e| fail("ready", &e)));
    let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .unwrap_or_else(|e| fail("signal", &e));
    loop {
        tokio::select! {
            line = lines.next_line() => {
                let Ok(Some(line)) = line else { break };
                if line.contains("\"quit\"") { break; }
                let (op, r) = command(&host, &app, &line);
                match r {
                    Ok(v) => emit(serde_json::json!({"event": "done", "op": op, "error": null, "value": v})),
                    Err(e) => emit(serde_json::json!({"event": "done", "op": op, "error": e})),
                }
            }
            _ = term.recv() => break,
        }
    }
    served.shutdown().await;
}

//! Agent spike host: the native shim. Signs in with ChatGPT and adds the token at the
//! wasi:http egress (OpenAI), or starts the unmodified `claude` binary and gives each
//! stream-json line to a fresh instance of the guest (Anthropic).

use anyhow::{Context, Result, bail};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
use wasmtime_wasi_http::{
    DEFAULT_FORBIDDEN_HEADERS, RequestOptions, WasiBody, WasiHttpCtx, WasiHttpCtxView,
    WasiHttpHooks, WasiHttpView,
};

wasmtime::component::bindgen!({ path: "../wit", world: "agent" });

const AUTH: &str = "https://auth.openai.com/api/accounts";
const RESOURCE: &str = "https://api.openai.com/v1";
const SCOPE: &str = "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";

struct Hooks {
    token: Option<String>,
    log: Vec<String>,
}

impl WasiHttpHooks for Hooks {
    // The guest may not set Authorization at all: only the host adds it.
    fn is_forbidden_header(&mut self, name: &http::HeaderName) -> bool {
        *name == http::header::AUTHORIZATION || DEFAULT_FORBIDDEN_HEADERS.contains(name)
    }

    fn send_request(
        &mut self,
        mut request: http::Request<WasiBody>,
        options: Option<RequestOptions>,
        _fut: Box<dyn Future<Output = Result<(), wasmtime_wasi_http::Error>> + Send>,
    ) -> Box<
        dyn Future<
                Output = Result<
                    (
                        http::Response<WasiBody>,
                        Box<dyn Future<Output = Result<(), wasmtime_wasi_http::Error>> + Send>,
                    ),
                    wasmtime_wasi_http::Error,
                >,
            > + Send,
    > {
        use wasmtime_wasi_http::Error::HttpRequestDenied;
        let host = request.uri().host().unwrap_or_default().to_ascii_lowercase();
        // ponytail: one hard-coded allowed host; watane takes it from mod.toml [http].
        let allowed = request.uri().scheme_str() == Some("https") && host == "api.openai.com";
        // Decided here, before any future runs: the record is what left the host, or not.
        self.log.push(format!("{} {}", if allowed { "send" } else { "deny" }, request.uri()));
        let token = self.token.clone();
        Box::new(async move {
            if !allowed {
                return Err(HttpRequestDenied);
            }
            let token = token.ok_or(HttpRequestDenied)?;
            let value = format!("Bearer {token}").parse().map_err(|_| HttpRequestDenied)?;
            request.headers_mut().insert(http::header::AUTHORIZATION, value);
            let (res, io) = wasmtime_wasi_http::default_send_request(request, options).await?;
            let io: Box<dyn Future<Output = _> + Send> = Box::new(io);
            Ok((res.map(http_body_util::BodyExt::boxed_unsync), io))
        })
    }
}

struct Ctx {
    wasi: WasiCtx,
    http: WasiHttpCtx,
    table: ResourceTable,
    hooks: Hooks,
    started: Instant,
}

impl WasiView for Ctx {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView { ctx: &mut self.wasi, table: &mut self.table }
    }
}

impl WasiHttpView for Ctx {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView { ctx: &mut self.http, table: &mut self.table, hooks: &mut self.hooks }
    }
}

/// The stand-in tools: what other mods would answer. Both vendors reach the same ones.
fn tool(name: &str, args: &Value) -> Result<String, String> {
    match name {
        "add" => Ok((args["a"].as_i64().unwrap_or(0) + args["b"].as_i64().unwrap_or(0)).to_string()),
        "now" => Ok(format!("{} seconds since 1970", now())),
        _ => Err(format!("no tool {name}")),
    }
}

impl spike::agent::tools::Host for Ctx {
    fn call(&mut self, name: String, arguments: String) -> Result<String, String> {
        tool(&name, &serde_json::from_str(&arguments).map_err(|e| e.to_string())?)
    }
}

/// A minimal MCP server (streamable HTTP, JSON answers only) on loopback, with a
/// token for this run: the shape of watane's `/mcp/<toolset>`. Returns the port.
fn serve_mcp(token: String) -> Result<u16> {
    let server = tiny_http::Server::http("127.0.0.1:0").map_err(|e| anyhow::anyhow!("{e}"))?;
    let port = server.server_addr().to_ip().context("no ip")?.port();
    std::thread::spawn(move || {
        for mut req in server.incoming_requests() {
            let authorized = req.headers().iter().any(|h| {
                h.field.equiv("authorization") && h.value.as_str() == format!("Bearer {token}")
            });
            let reply = |code: u16, body: String| {
                tiny_http::Response::from_string(body).with_status_code(code).with_header(
                    "content-type: application/json".parse::<tiny_http::Header>().unwrap(),
                )
            };
            if !authorized {
                let _ = req.respond(reply(401, "{}".into()));
                continue;
            }
            if *req.method() != tiny_http::Method::Post {
                let _ = req.respond(reply(405, "{}".into()));
                continue;
            }
            let mut body = String::new();
            let _ = req.as_reader().read_to_string(&mut body);
            let msg: Value = serde_json::from_str(&body).unwrap_or_default();
            let Some(id) = msg.get("id").cloned() else {
                let _ = req.respond(reply(202, String::new())); // a notification
                continue;
            };
            let p = &msg["params"];
            let result = match msg["method"].as_str().unwrap_or_default() {
                "initialize" => json!({
                    "protocolVersion": p["protocolVersion"],
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "watane-spike", "version": "0.0.0"}
                }),
                "tools/list" => json!({"tools": [
                    {"name": "add", "description": "Add two integers.",
                     "inputSchema": {"type": "object", "properties": {"a": {"type": "integer"}, "b": {"type": "integer"}}, "required": ["a", "b"]}},
                    {"name": "now", "description": "The current time on the participant's machine.",
                     "inputSchema": {"type": "object", "properties": {}}}
                ]}),
                "tools/call" => match tool(p["name"].as_str().unwrap_or_default(), &p["arguments"]) {
                    Ok(t) => json!({"content": [{"type": "text", "text": t}]}),
                    Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
                },
                _ => json!({}),
            };
            let out = json!({"jsonrpc": "2.0", "id": id, "result": result});
            let _ = req.respond(reply(200, out.to_string()));
        }
    });
    Ok(port)
}

impl spike::agent::events::Host for Ctx {
    fn emit(&mut self, update: String) {
        println!("[{:>6.2}s] {update}", self.started.elapsed().as_secs_f64());
    }
}

struct Guest {
    engine: Engine,
    pre: AgentPre<Ctx>,
}

impl Guest {
    fn load() -> Result<Self> {
        let engine = Engine::default();
        let path = env!("CARGO_MANIFEST_DIR").to_owned()
            + "/../guest/target/wasm32-wasip2/release/agent_spike_guest.wasm";
        let component = Component::from_file(&engine, &path).map_err(|e| anyhow::anyhow!("{path}: {e}"))?;
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        wasmtime_wasi_http::p2::add_only_http_to_linker_sync(&mut linker)?;
        Agent::add_to_linker::<_, HasSelf<Ctx>>(&mut linker, |c| c)?;
        let pre = AgentPre::new(linker.instantiate_pre(&component)?)?;
        Ok(Self { engine, pre })
    }

    /// A new instance for each call, as in watane's reactor model.
    fn instance(&self, token: Option<String>, started: Instant) -> Result<(Store<Ctx>, Agent)> {
        let ctx = Ctx {
            wasi: WasiCtx::builder().inherit_stderr().build(),
            http: WasiHttpCtx::new(),
            table: ResourceTable::new(),
            hooks: Hooks { token, log: Vec::new() },
            started,
        };
        let mut store = Store::new(&self.engine, ctx);
        let agent = self.pre.instantiate(&mut store)?;
        Ok((store, agent))
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn home() -> Result<PathBuf> {
    let dir = PathBuf::from(std::env::var("HOME")?).join(".config/watane-agent-spike");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn save(path: &PathBuf, v: &Value) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let tmp = path.with_extension("tmp");
    std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?
        .write_all(serde_json::to_string_pretty(v)?.as_bytes())?;
    Ok(std::fs::rename(tmp, path)?)
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn dec(s: &str) -> String {
    let b = s.replace('+', " ").into_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or(""), 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn random() -> String {
    // ponytail: two v4 UUIDs (OS randomness) as 32 random bytes.
    let mut b = uuid::Uuid::new_v4().as_bytes().to_vec();
    b.extend(uuid::Uuid::new_v4().as_bytes());
    B64.encode(b)
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().http_status_as_error(false).build().into()
}

fn token_request(form: &[(&str, &str)]) -> Result<Value> {
    let mut res = agent().post(format!("{AUTH}/oauth/token")).send_form(form.iter().copied())?;
    let status = res.status();
    let body: Value = res.body_mut().read_json()?;
    if !status.is_success() {
        bail!("token endpoint {status}: {body}");
    }
    Ok(body)
}

fn login() -> Result<()> {
    let dir = home()?;
    let host_id_path = dir.join("host-id");
    let host_id = match std::fs::read_to_string(&host_id_path) {
        Ok(id) => id.trim().to_owned(),
        Err(_) => {
            let id = format!("urn:uuid:{}", uuid::Uuid::new_v4());
            std::fs::write(&host_id_path, &id)?;
            id
        }
    };
    let creds_path = dir.join("openai.json");
    let saved: Option<Value> = std::fs::read(&creds_path).ok().and_then(|b| serde_json::from_slice(&b).ok());
    let saved_client = saved.as_ref().and_then(|s| s["client_id"].as_str().map(str::to_owned));

    let listener = TcpListener::bind("127.0.0.1:1455").or_else(|_| TcpListener::bind("127.0.0.1:0"))?;
    let redirect = format!("http://127.0.0.1:{}/auth/callback", listener.local_addr()?.port());
    let (verifier, state, nonce) = (random(), random(), random());
    let challenge = B64.encode(Sha256::digest(verifier.as_bytes()));
    let mut params = vec![
        ("client_id", saved_client.clone().unwrap_or("dynamic_agent_client".into())),
        ("ext_agent_host_id", host_id.clone()),
        ("response_type", "code".into()),
        ("redirect_uri", redirect.clone()),
        ("scope", SCOPE.into()),
        ("resource", RESOURCE.into()),
        ("state", state.clone()),
        ("nonce", nonce),
        ("code_challenge_method", "S256".into()),
        ("code_challenge", challenge),
    ];
    if saved_client.is_none() {
        params.push(("agent_name_hint", "watane agent spike".into()));
    }
    let query: Vec<String> = params.iter().map(|(k, v)| format!("{k}={}", enc(v))).collect();
    let url = format!("{AUTH}/authorize?{}", query.join("&"));
    println!("Opening the browser for Sign in with ChatGPT. If it does not open:\n{url}");
    let _ = std::process::Command::new("open").arg(&url).status();

    let (mut stream, _) = listener.accept()?;
    let mut line = String::new();
    BufReader::new(stream.try_clone()?).read_line(&mut line)?;
    stream.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\nconnection: close\r\n\r\nDone. You can close this tab.")?;
    let target = line.split_whitespace().nth(1).unwrap_or_default();
    let q: std::collections::HashMap<String, String> = target
        .split_once('?')
        .map(|(_, q)| q)
        .unwrap_or_default()
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_owned(), dec(v)))
        .collect();
    if q.get("state") != Some(&state) {
        bail!("state does not match");
    }
    if let Some(error) = q.get("error") {
        bail!("sign-in refused: {error}");
    }
    let client_id = match (q.get("client_id"), &saved_client) {
        (Some(c), Some(s)) if c != s => bail!("callback client_id differs from the saved one"),
        (Some(c), _) => c.clone(),
        (None, Some(s)) => s.clone(),
        (None, None) => bail!("registration incomplete: no issued client_id"),
    };
    let code = q.get("code").context("no code")?;
    let mut t = token_request(&[
        ("grant_type", "authorization_code"),
        ("client_id", &client_id),
        ("code", code),
        ("code_verifier", &verifier),
        ("redirect_uri", &redirect),
        ("resource", RESOURCE),
    ])?;
    // ponytail: no ID-token signature/nonce check in the spike; a real host verifies against the JWKS.
    let scope = t["scope"].as_str().unwrap_or_default().to_owned();
    if !scope.split(' ').any(|s| s == "chatgpt.tokens.use.direct") {
        bail!("plan usage not granted (scope: {scope})");
    }
    t["client_id"] = json!(client_id);
    t["ext_agent_host_id"] = json!(host_id);
    t["saved_at"] = json!(now());
    save(&creds_path, &t)?;
    println!("Signed in; plan usage granted. Credentials in {}", creds_path.display());
    Ok(())
}

fn access_token() -> Result<String> {
    let path = home()?.join("openai.json");
    let mut c: Value = serde_json::from_slice(&std::fs::read(&path).context("run `login` first")?)?;
    let expires = c["saved_at"].as_u64().unwrap_or(0) + c["expires_in"].as_u64().unwrap_or(0);
    if now() + 120 >= expires {
        let client_id = c["client_id"].as_str().unwrap_or_default().to_owned();
        let refresh = c["refresh_token"].as_str().unwrap_or_default().to_owned();
        let mut t = token_request(&[
            ("grant_type", "refresh_token"),
            ("client_id", &client_id),
            ("refresh_token", &refresh),
            ("resource", RESOURCE),
        ])?;
        for k in ["client_id", "ext_agent_host_id", "id_token"] {
            if t.get(k).is_none() {
                t[k] = c[k].clone();
            }
        }
        t["saved_at"] = json!(now());
        save(&path, &t)?;
        c = t;
    }
    Ok(c["access_token"].as_str().context("no access_token")?.to_owned())
}

fn models() -> Result<()> {
    let mut res = agent()
        .get(format!("{RESOURCE}/models"))
        .header("authorization", format!("Bearer {}", access_token()?))
        .call()?;
    let v: Value = res.body_mut().read_json()?;
    for m in v["models"].as_array().into_iter().flatten().filter(|m| m["visibility"] == "list") {
        println!("{}  ({})", m["slug"].as_str().unwrap_or("?"), m["display_name"].as_str().unwrap_or(""));
    }
    if v["models"].is_null() {
        println!("{v}");
    }
    Ok(())
}

fn openai(model: &str, prompt: &str) -> Result<()> {
    let guest = Guest::load()?;
    let started = Instant::now();
    let (mut store, agent) = guest.instance(Some(access_token()?), started)?;
    let result = agent.call_openai(&mut store, model, prompt)?;
    println!("--- {:.2}s in one call ---", started.elapsed().as_secs_f64());
    match result {
        Ok(answer) => println!("{answer}"),
        Err(e) => bail!("guest: {e}"),
    }
    Ok(())
}

fn claude(prompt: &str) -> Result<()> {
    let guest = Guest::load()?;
    let dir = std::env::temp_dir().join(format!("agent-spike-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("hello.txt"), "hello from the spike\n")?;
    // A toolset token for this run only, as watane gives one per agent run.
    let token = random();
    let port = serve_mcp(token.clone())?;
    let mcp = json!({"mcpServers": {"watane": {
        "type": "http",
        "url": format!("http://127.0.0.1:{port}/mcp"),
        "headers": {"Authorization": format!("Bearer {token}")}
    }}});
    let mut child = std::process::Command::new("claude")
        .args(["-p", prompt, "--output-format", "stream-json", "--verbose"])
        .args(["--strict-mcp-config", "--mcp-config", &mcp.to_string()])
        .args(["--allowedTools", "mcp__watane__add mcp__watane__now"])
        .current_dir(&dir)
        // The subscription login of the participant's own `claude`, never an API key.
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("ANTHROPIC_AUTH_TOKEN")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .context("start claude")?;
    let started = Instant::now();
    let (mut lines, mut instance_time) = (0u32, 0f64);
    for line in BufReader::new(child.stdout.take().context("stdout")?).lines() {
        let line = line?;
        let t = Instant::now();
        let (mut store, agent) = guest.instance(None, started)?;
        let updates = agent.call_claude_line(&mut store, &line)?;
        instance_time += t.elapsed().as_secs_f64();
        lines += 1;
        match updates {
            Ok(updates) => updates.iter().for_each(|u| println!("[{:>6.2}s] {u}", started.elapsed().as_secs_f64())),
            Err(e) => println!("[{:>6.2}s] unparsed line ({e}): {}", started.elapsed().as_secs_f64(), &line[..line.len().min(160)]),
        }
        if let Ok(v) = serde_json::from_str::<Value>(&line)
            && v["type"] == "result"
        {
            println!("--- result: {} (subtype {}, api key source: see init line)", v["result"], v["subtype"]);
        }
    }
    let status = child.wait()?;
    println!(
        "--- claude exited {status}; {lines} lines, {:.2}s total, {:.1} ms per fresh instance",
        started.elapsed().as_secs_f64(),
        instance_time * 1000.0 / f64::from(lines.max(1))
    );
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

/// A guest that tries to misuse the egress, and a caller without the MCP token.
/// Uses a fake bearer, so no real token leaves even if a check fails.
fn hostile() -> Result<()> {
    let guest = Guest::load()?;
    let h = |k: &str, v: &str| vec![(k.to_owned(), v.to_owned())];
    let models = "https://api.openai.com/v1/models";
    // Judged on what the host's egress hook decided, not on the guest's error text:
    // wstd races body and response, so a denial may reach the guest as "connection reset".
    // `None`: refused before the hook (a forbidden header). `Some(Err(()))`: denied by the
    // hook, nothing sent. `Some(Ok(status))`: sent, with the host's (fake) token.
    let cases: Vec<(&str, &str, Vec<(String, String)>, Option<Result<u16, ()>>)> = vec![
        ("other host", "https://example.com/", vec![], Some(Err(()))),
        ("look-alike host", "https://api.openai.com.evil.example/", vec![], Some(Err(()))),
        ("plain http", "http://api.openai.com/v1/models", vec![], Some(Err(()))),
        ("loopback", "https://127.0.0.1:1455/", vec![], Some(Err(()))),
        ("own Authorization", models, h("authorization", "Bearer guest-chosen"), None),
        ("Host header swap", models, h("host", "evil.example"), None),
        ("Proxy-Authorization", models, h("proxy-authorization", "x"), None),
        ("allowed host gets the host's token", models, vec![], Some(Ok(401))),
        ("allowed host, upper case", "https://API.OPENAI.COM/v1/models", vec![], Some(Ok(401))),
    ];
    let mut failed = 0;
    for (case, url, headers, want) in cases {
        let (mut store, agent) = guest.instance(Some("spike-fake-token".into()), Instant::now())?;
        let got = agent.call_probe(&mut store, url, &headers)?;
        let log = std::mem::take(&mut store.data_mut().hooks.log);
        let ok = match want {
            None => log.is_empty() && got.as_ref().is_err_and(|e| e.contains("Forbidden")),
            Some(Err(())) => log.len() == 1 && log[0].starts_with("deny ") && got.is_err(),
            Some(Ok(status)) => log.len() == 1 && log[0].starts_with("send ") && got == Ok(status),
        };
        failed += u32::from(!ok);
        let got = format!("{got:?}");
        println!("{} {case}: host {log:?}, guest {}", if ok { "PASS" } else { "FAIL" }, &got[..got.len().min(90)]);
    }
    let token = random();
    let url = format!("http://127.0.0.1:{}/mcp", serve_mcp(token.clone())?);
    let init = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"});
    for (case, bearer, want) in [("MCP without token", None, 401), ("MCP wrong token", Some("nope".to_owned()), 401), ("MCP right token", Some(token), 200)] {
        let mut req = agent().post(&url);
        if let Some(b) = bearer {
            req = req.header("authorization", format!("Bearer {b}"));
        }
        let status = req.send_json(&init)?.status().as_u16();
        failed += u32::from(status != want);
        println!("{} {case}: {status}", if status == want { "PASS" } else { "FAIL" });
    }
    if failed > 0 {
        bail!("{failed} hostile checks failed");
    }
    Ok(())
}

/// V23 probe: POST a JSON body to `{RESOURCE}<path>` with the plan token; print the status and,
/// for a stream, each event type with any error and the final usage.
fn post(path: &str, body_file: &str) -> Result<()> {
    let body = std::fs::read_to_string(body_file)?;
    let mut res = agent()
        .post(format!("{RESOURCE}{path}"))
        .header("authorization", format!("Bearer {}", access_token()?))
        .header("content-type", "application/json")
        .send(body)?;
    println!("status {}", res.status());
    let text = res.body_mut().read_to_string()?;
    if !text.contains("\ndata: ") && !text.starts_with("event:") {
        println!("{}", text.chars().take(2000).collect::<String>());
        return Ok(());
    }
    for line in text.lines().filter_map(|l| l.strip_prefix("data: ")) {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        let t = v["type"].as_str().unwrap_or("?");
        if t.ends_with(".delta") {
            continue;
        }
        match t {
            "response.completed" | "response.failed" | "response.incomplete" => {
                println!("{t} usage={} error={}", v["response"]["usage"], v["response"]["error"]);
                for item in v["response"]["output"].as_array().into_iter().flatten() {
                    println!("  output item {}", item["type"]);
                    if std::env::var_os("PROBE_DUMP").is_some() {
                        println!("  item-json {item}");
                    }
                }
            }
            "error" => println!("error {v}"),
            _ => println!("{t}"),
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).map(String::as_str);
    match (arg(0), arg(1), arg(2)) {
        (Some("login"), _, _) => login(),
        (Some("models"), _, _) => models(),
        (Some("openai"), Some(model), Some(prompt)) => openai(model, prompt),
        (Some("claude"), Some(prompt), _) => claude(prompt),
        (Some("hostile"), _, _) => hostile(),
        (Some("post"), Some(path), Some(body)) => post(path, body),
        _ => bail!("usage: agent-spike-host login | models | openai <model> <prompt> | claude <prompt> | hostile"),
    }
}

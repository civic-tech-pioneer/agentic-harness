//! Cost of one call into a mod through watane-host's real call path (Host::call / call_async).
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use watane_host::{Caller, Host, Limits, LocalName, Mode};

const N: usize = 2000;
const WARM: usize = 200;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../watane/host/tests/fixtures/mods").join(name)
}

fn host(home: &Path, names: &[&str]) -> Arc<Host> {
    let host = Host::open(home, Mode::Preview, Limits::default()).unwrap();
    host.install_folders(&names.iter().map(|n| fixture(n)).collect::<Vec<_>>(), 0).unwrap();
    host
}

fn report(label: &str, mut t: Vec<Duration>) {
    let total: Duration = t.iter().sum();
    t.sort();
    let p = |q: f64| t[((t.len() as f64 * q) as usize).min(t.len() - 1)].as_secs_f64() * 1e6;
    println!(
        "{label:<44} p50 {:>8.1} us  p95 {:>8.1} us  p99 {:>8.1} us  max {:>8.1} us  {:>7.0} calls/s",
        p(0.50), p(0.95), p(0.99), p(1.0), t.len() as f64 / total.as_secs_f64()
    );
}

/// Warm up, then time N sequential calls; `args(i)` builds the arguments of call i.
fn bench(label: &str, h: &Host, local: &str, f: &str, args: impl Fn(usize) -> Value) {
    let local = LocalName::parse(local).unwrap();
    for i in 0..WARM {
        h.call(&Caller::Page, &local, f, args(i)).unwrap();
    }
    let t = (WARM..WARM + N)
        .map(|i| {
            let a = args(i);
            let s = Instant::now();
            h.call(&Caller::Page, &local, f, a).unwrap();
            s.elapsed()
        })
        .collect();
    report(label, t);
}

fn main() {
    // Host::call needs a 16 MiB stack.
    std::thread::Builder::new().stack_size(16 << 20).spawn(run).unwrap().join().unwrap();
}

/// Raw redb baseline: one small insert per commit, as the host's commit does, at both durabilities.
fn redb_baseline() {
    const T: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("t");
    let dir = tempfile::tempdir().unwrap();
    let db = redb::Database::create(dir.path().join("x.redb")).unwrap();
    for (label, d) in [
        ("(redb) raw commit, Durability::Immediate", redb::Durability::Immediate),
        ("(redb) raw commit, Durability::None", redb::Durability::None),
    ] {
        let mut t = Vec::with_capacity(N);
        for i in 0..WARM + N {
            let s = Instant::now();
            let mut w = db.begin_write().unwrap();
            w.set_durability(d).unwrap();
            w.open_table(T).unwrap().insert("k", format!("{i:064}").as_bytes()).unwrap();
            w.commit().unwrap();
            if i >= WARM {
                t.push(s.elapsed());
            }
        }
        report(label, t);
    }
}

fn run() {
    redb_baseline();
    let s200 = "x".repeat(200);
    let s4k = "x".repeat(4096);

    let home = tempfile::tempdir().unwrap();
    let h = host(home.path(), &["echo"]);
    bench("(a) echo id-string 200 B", &h, "echo", "api.id-string", |_| json!({"x": s200}));
    bench("(d) echo id-string 4 KiB", &h, "echo", "api.id-string", |_| json!({"x": s4k}));
    bench("(a0) echo id-u32 (no payload)", &h, "echo", "api.id-u32", |i| json!({"x": i}));
    bench("(b) echo put 64 B, new value each call", &h, "echo", "api.put", |i| {
        json!({"key": "k", "value": format!("{i:064}").into_bytes()})
    });
    bench("(b=) echo put 64 B, same value (no change)", &h, "echo", "api.put", |_| {
        json!({"key": "k", "value": vec![7u8; 64]})
    });
    bench("(r) echo read 64 B (read txn, no write)", &h, "echo", "api.read", |_| json!({"key": "k"}));

    // call_async: what /call and MCP use; a fresh 16 MiB thread per call.
    {
        let rt = tokio::runtime::Builder::new_multi_thread().build().unwrap();
        let local = LocalName::parse("echo").unwrap();
        let mut t = Vec::with_capacity(N);
        rt.block_on(async {
            for i in 0..WARM + N {
                let s = Instant::now();
                h.call_async(Caller::Page, local.clone(), "api.id-string".into(), json!({"x": s200}))
                    .await
                    .unwrap();
                if i >= WARM {
                    t.push(s.elapsed());
                }
            }
        });
        report("(a-async) call_async echo id-string 200 B", t);
    }

    // Serialisation: 4 threads hammer one mod at once.
    for threads in [1usize, 4] {
        let local = LocalName::parse("echo").unwrap();
        let per = N / threads;
        let start = Instant::now();
        std::thread::scope(|sc| {
            for _ in 0..threads {
                let (h, local, s200) = (&h, &local, &s200);
                std::thread::Builder::new()
                    .stack_size(16 << 20)
                    .spawn_scoped(sc, move || {
                        for _ in 0..per {
                            h.call(&Caller::Page, local, "api.id-string", json!({"x": s200})).unwrap();
                        }
                    })
                    .unwrap();
            }
        });
        let el = start.elapsed();
        println!(
            "(par) {threads} thread(s) x {per} id-string on one mod: {:.0} calls/s total",
            (per * threads) as f64 / el.as_secs_f64()
        );
    }
    drop(h);

    // (c): shelf.add puts the growing title list and emits `added`; recs subscribes and puts too.
    let home = tempfile::tempdir().unwrap();
    let h = host(home.path(), &["recs", "shelf"]);
    bench("(c) shelf add: put + event -> recs put", &h, "shelf", "items.add", |i| {
        json!({"title": format!("{i}")})
    });
    let got = h
        .call(&Caller::Page, &LocalName::parse("recs").unwrap(), "api.received", json!(null))
        .unwrap();
    println!("    recs received {} events", got.as_array().unwrap().len());
    drop(h);

    let home = tempfile::tempdir().unwrap();
    let h = host(home.path(), &["shelf"]);
    bench("(c0) shelf add: put + event, no subscriber", &h, "shelf", "items.add", |i| {
        json!({"title": format!("{i}")})
    });
}

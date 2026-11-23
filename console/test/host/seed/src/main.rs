//! Test-only (S36-5): writes one key of one mod's state into a watane home whose host is stopped.
//!
//! No kernel call writes `access/<identity>` (TECHNICAL 9.1), so the stage B load test gives its
//! second owner (`local/loadclient`) the built-in `scratch` profile by writing that key here. This
//! reaches into the host's private state layout (watane `host/src/state.rs` at slice/s100: one redb
//! table `s/<first 32 hex of blake3(identity)>` of `&str -> &[u8]`, and the `sizes` table of each
//! mod's bytes); it is a test fixture, never a way the product writes state.
//!
//! Use: `seed-state <home> <identity> <key> <value>`.
use redb::{Database, ReadableTable, TableDefinition};

const SIZES: TableDefinition<&str, u64> = TableDefinition::new("sizes");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, home, identity, key, value] = args.as_slice() else {
        eprintln!("use: seed-state <home> <identity> <key> <value>");
        std::process::exit(64);
    };
    let id = blake3::hash(identity.as_bytes()).to_hex()[..32].to_string();
    let db = Database::open(std::path::Path::new(home).join("state/state.redb"))
        .unwrap_or_else(|e| fail(&format!("open: {e}")));
    let txn = db.begin_write().unwrap_or_else(|e| fail(&format!("write: {e}")));
    {
        let name = format!("s/{id}");
        let mut table = txn
            .open_table(TableDefinition::<&str, &[u8]>::new(&name))
            .unwrap_or_else(|e| fail(&format!("table: {e}")));
        let mut sizes = txn.open_table(SIZES).unwrap_or_else(|e| fail(&format!("sizes: {e}")));
        let mut size = sizes
            .get(id.as_str())
            .unwrap_or_else(|e| fail(&format!("size: {e}")))
            .map_or(0, |v| v.value());
        if let Some(old) = table.get(key.as_str()).unwrap_or_else(|e| fail(&format!("get: {e}"))) {
            size -= (key.len() + old.value().len()) as u64;
        }
        size += (key.len() + value.len()) as u64;
        table
            .insert(key.as_str(), value.as_bytes())
            .unwrap_or_else(|e| fail(&format!("put: {e}")));
        sizes
            .insert(id.as_str(), size)
            .unwrap_or_else(|e| fail(&format!("sizes: {e}")));
    }
    txn.commit().unwrap_or_else(|e| fail(&format!("commit: {e}")));
}

fn fail(why: &str) -> ! {
    eprintln!("seed-state: {why}");
    std::process::exit(1)
}

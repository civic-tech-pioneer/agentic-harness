//! Test-only (S36-5): passes `start`, `say`, `interrupt` and `cancel` to the agent kernel as this mod.
wit_bindgen::generate!({ world: "loadclient", path: "../wit", generate_all });

use exports::mods::loadclient::api::Guest;
use mods::agent::sessions;

struct Api;
export!(Api);

impl Guest for Api {
    fn start(key: String, input: String) -> Result<String, String> {
        sessions::start(&key, "scratch", "{}", &input).map_err(|e| format!("{e:?}"))
    }
    fn say(session: String, key: String, input: String) -> Result<u64, String> {
        sessions::say(&session, &key, &input).map_err(|e| format!("{e:?}"))
    }
    fn interrupt(session: String) -> Result<(), String> {
        sessions::interrupt(&session, None).map_err(|e| format!("{e:?}"))
    }
    fn start_openai(key: String, input: String) -> Result<String, String> {
        sessions::start(&key, "Tools", "{}", &input).map_err(|e| format!("{e:?}"))
    }
    fn start_in_folder(key: String, input: String, folder: String) -> Result<String, String> {
        sessions::start_in_folder(&key, "scratch", "{}", &input, &folder).map_err(|e| format!("{e:?}"))
    }
    fn read(session: String, after: u64, max: u32) -> Result<Vec<String>, String> {
        sessions::read(&session, after, None, max)
            .map(|page| page.entries)
            .map_err(|e| format!("{e:?}"))
    }
    fn note() -> String {
        "preview tool result".to_owned()
    }
    fn cancel(session: String) {
        sessions::cancel(&session)
    }
}

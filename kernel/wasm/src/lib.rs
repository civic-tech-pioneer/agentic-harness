//! The agent kernel as a watane mod (TECHNICAL 1, 2.1, 2.3): the exports of `wit/world.wit` over
//! the `agent-kernel` crate, whose `Host` and `State` this crate implements over host 19's imports.
//!
//! `map` holds the host-independent mappings and is tested natively; `component` is the glue.

wit_bindgen::generate!({
    world: "agent",
    path: "wit",
    generate_all,
});

pub mod map;

#[cfg(target_arch = "wasm32")]
mod component;

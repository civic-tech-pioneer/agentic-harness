# agent-spike

A mod calls agents on the participant's subscription, from inside the sandbox.
Findings: `../../research_notes/calling-agents.md`.

```bash
(cd guest && cargo build --target wasm32-wasip2 --release)
(cd host && cargo build --release)
host/target/release/agent-spike-host claude "<prompt>"          # unmodified claude, own login
host/target/release/agent-spike-host login                      # Sign in with ChatGPT
host/target/release/agent-spike-host models
host/target/release/agent-spike-host openai gpt-6.2-sol "<prompt>"
host/target/release/agent-spike-host hostile                    # egress and MCP checks, exits 1 on a failure
```

Credentials: `~/.config/watane-agent-spike/` (0600).

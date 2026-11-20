"""V23: what the Responses API allows on Sign in with ChatGPT plan tokens: prompt caching with
prompt_cache_key, server-side compaction (context_management, a compaction_trigger item,
POST /responses/compact). Uses the spike host's token (`agent-spike-host login` first).

    python3 -I proto/agent-spike/chatgpt-probes.py [model]
"""

import json, os, subprocess, sys, tempfile

HOST = os.path.join(os.path.dirname(os.path.abspath(__file__)), "host/target/release/agent-spike-host")
MODEL = sys.argv[1] if len(sys.argv) > 1 else "gpt-6-luna"
LONG = " ".join(f"Rule {i}: the watane probe agent answers briefly and never uses tools it was not given."
                for i in range(300))  # about 6k tokens, well above the 1024-token cache minimum


def post(path, body):
    f = tempfile.NamedTemporaryFile("w", suffix=".json", delete=False)
    json.dump(body, f)
    f.close()
    out = subprocess.run([HOST, "post", path, f.name], capture_output=True, text=True, timeout=300)
    os.unlink(f.name)
    return (out.stdout + out.stderr).strip()


def base(**extra):
    return {"model": MODEL, "instructions": LONG, "stream": True, "store": False,
            "input": [{"role": "user", "content": "Reply with the word OK."}], **extra}


print("model", MODEL)
print("\n== 1. baseline")
print(post("/responses", base()))
print("\n== 2. prompt_cache_key, first then second request (look for cached_tokens)")
for _ in range(2):
    print(post("/responses", base(prompt_cache_key="watane-probe-v23")))
print("\n== 3. context_management compaction with a low threshold")
print(post("/responses", base(context_management=[{"type": "compaction", "compact_threshold": 1000}])))
print("\n== 4. a compaction_trigger input item")
b = base()
b["input"].append({"type": "compaction_trigger"})
print(post("/responses", b))
print("\n== 5. POST /responses/compact")
print(post("/responses/compact", {"model": MODEL, "instructions": "You are a probe.",
                                   "input": [{"role": "user", "content": LONG}]}))

print("\n== 6. a compaction item from 3, sent back as input")
os.environ["PROBE_DUMP"] = "1"
out = post("/responses", base(context_management=[{"type": "compaction", "compact_threshold": 1000}]))
items = [json.loads(l.split("item-json ", 1)[1]) for l in out.splitlines() if "item-json " in l]
comp = [i for i in items if i.get("type") == "compaction"]
del os.environ["PROBE_DUMP"]
print("compaction items:", len(comp), "keys:", sorted(comp[0].keys()) if comp else None)
if comp:
    c = {k: v for k, v in comp[-1].items() if k != "id"}
    b = base()
    b["input"] = [c, {"role": "user", "content": "What did my first message ask for? One line."}]
    print(post("/responses", b))

# S37 — Enforcing the shell grant’s local-network setting

**From docs.** V27 asks whether a profile can deny local-network access by default and explicitly allow it on macOS and Linux. For this spike, local means IPv4 and IPv6 loopback, RFC 1918 and IPv6 unique-local ranges, and IPv4 and IPv6 link-local ranges. The metadata address `169.254.169.254` is inside IPv4 link-local. The human asked for the mechanisms and their limitations before deciding V27. [S37 and V27 in the plan](../factory/PLAN.md)

**Recommendation (from docs and probed evidence below).** Keep the domain allowlist in both modes. Use kernel validation to catch obvious local entries when denied, and require a host-owned address check at the final outbound dial before calling the setting enforced. Verify on each supported Claude build that both proxy paths are controlled and direct egress stays isolated. If that integration is unavailable, fail closed for the shell grant that promises local denial. The fallback for the human to consider is explicitly labeled best effort, with the remaining DNS and proxy paths disclosed. Keep the decision in stage C and put the same suite under CI on Linux and macOS when the grant is implemented. [Shell grant and sandbox contract](TECHNICAL.md#42-grants-and-tools) · [SRT network architecture and address checks](https://github.com/anthropic-experimental/sandbox-runtime/blob/0bbebed0aa9a8e628dc4df8bc30293550a1988eb/README.md#network-isolation-architecture)

## Candidate mechanisms

### A. Kernel validation of allowlist entries

- **Probed.** A lexical check can reject `localhost`, subdomains of `.localhost`, case and final-dot variants, and literals within scope including IPv4-mapped IPv6. The fixture accepts `permitted.example` even when its simulated DNS answer is local. Validation alone therefore leaves DNS changes, local aliases, and commands that connect directly to the address to other layers. The fixture assumes syntax validation has already run. [Fixture and output](#linux-container)
- **From docs.** This is a small profile validation change, with no platform-specific runtime. Apply it after canonicalization and retain the existing plain-domain grammar. Reject alternate numeric spellings or require them to canonicalize as addresses before checking. A save-time lookup would still be a snapshot. [Shell grant contract](TECHNICAL.md#42-grants-and-tools) · [WHATWG IPv4 parser](https://url.spec.whatwg.org/#concept-ipv4-parser) · [SRT resolved-address guard](https://github.com/anthropic-experimental/sandbox-runtime/blob/0bbebed0aa9a8e628dc4df8bc30293550a1988eb/src/sandbox/resolved-address-guard.ts)

### B. Claude sandbox settings

- **Probed, prior macOS run.** S1 on Claude Code 2.1.330 and Sonnet reported `curl sent=True` on all 12 cases. The unsandboxed control reached the loopback listener over all three client paths. An empty allowlist and an unrelated domain each gave three `reached=False` results. With `["localhost"]`, direct IP and direct hostname gave `reached=False` and the sandbox proxy gave `reached=True`. This demonstrates the behavior of that release and configuration for loopback. It supplies no measured private, metadata, DNS-change, or mixed-answer outcomes. [Recorded output](ARCHITECTURE-PROPOSAL.md#18-checks-before-building) · [Probe](../proto/claude-probes/probes.py) · [S1 PR](https://github.com/civic-tech-pioneer/agentic-harness/pull/37)
- **From docs.** Standalone Anthropic Sandbox Runtime (SRT) uses Seatbelt on macOS to restrict outbound access to the proxy ports and bubblewrap plus Unix socket relays on Linux to connect an isolated network namespace to host proxies. Keep unsandboxed commands, exclusions, and additional Unix socket access disabled. SRT’s `allowLocalBinding` permits direct host loopback on macOS and does not expose host loopback on Linux, where the child has its own namespace. It is unsuitable as a portable switch for these ranges. [SRT network isolation](https://github.com/anthropic-experimental/sandbox-runtime/blob/0bbebed0aa9a8e628dc4df8bc30293550a1988eb/README.md#network-isolation-architecture) · [SRT network configuration](https://github.com/anthropic-experimental/sandbox-runtime/blob/0bbebed0aa9a8e628dc4df8bc30293550a1988eb/README.md#network-configuration)
- **From docs.** Pinned upstream SRT has a direct-dial resolved-address guard. It resolves once and supplies filtered addresses to the dial. It covers loopback, link-local and several metadata endpoints by default. Private ranges and ULA require extra `deniedResolvedAddresses`. The guard handles mapped IPv4 and several translated forms. It has intentional exceptions for explicitly allowed literals and reserved localhost names. Parent and MITM proxy routes delegate resolution and address policy to the next hop. This is a promising low-integration-cost option if the pinned Claude binary exposes equivalent controls and behavior on both platforms. The upstream source establishes SRT behavior only. [SRT resolved-address guard](https://github.com/anthropic-experimental/sandbox-runtime/blob/0bbebed0aa9a8e628dc4df8bc30293550a1988eb/src/sandbox/resolved-address-guard.ts) · [Claude Code sandboxing](https://code.claude.com/docs/en/sandboxing)

### C. Watane host egress broker

- **Probed.** The fixture applies a domain check, then checks resolved addresses for the configured ranges. Denied mode drops every local candidate in the fixture and selects the public candidate in a mixed answer. The next lookup of the same name returns a mapped loopback address and is denied. The real loopback listener records zero requests for denied mode, one for allowed mode, and one for a direct connection outside the broker. The changing answers are injected and the only actual dial is to the fixture on IPv4 loopback. [Fixture and output](#linux-container)
- **From docs.** Put the check where resolution and the last dial happen. Use one resolution per connection attempt and dial only a checked numeric sockaddr, preserving the original hostname for HTTP authority and TLS verification. Check each fallback candidate, HTTP request target, CONNECT target and SOCKS destination. In allowed mode, remove the profile’s local range denial while retaining hostname checks and runtime isolation. A user-facing mode can choose this policy even when an upstream proxy would otherwise keep its own stricter defaults. [SRT resolved-address guard](https://github.com/anthropic-experimental/sandbox-runtime/blob/0bbebed0aa9a8e628dc4df8bc30293550a1988eb/src/sandbox/resolved-address-guard.ts) · [HTTP CONNECT](https://www.rfc-editor.org/rfc/rfc9110.html#name-connect) · [SOCKS5 request](https://www.rfc-editor.org/rfc/rfc1928.html#section-4)
- **From docs.** Try explicit HTTP and SOCKS proxy ports where supported, or a mandatory upstream hop if both paths demonstrably use it and preserve the intended allowlist. SRT’s external proxy ports substitute for internal listeners, so the broker must own both the name and address policy on those paths. Proxy environment variables alone specify client preferences. The sandbox must restrict raw IPv4 and IPv6, UDP, host Unix sockets and any other reachable service that could relay traffic. Keep policy attached to the session and close its listeners and tunnels at session end. This adds a watane host component, protocol handling, lifecycle and cross-platform integration tests. Address classification also depends on deployment routing, including NAT64 prefixes, public addresses forwarded to local services, and explicitly allowed public relays. [SRT proxy wiring](https://github.com/anthropic-experimental/sandbox-runtime/blob/0bbebed0aa9a8e628dc4df8bc30293550a1988eb/src/sandbox/sandbox-manager.ts#L1776-L1839) · [SRT limitations](https://github.com/anthropic-experimental/sandbox-runtime/blob/0bbebed0aa9a8e628dc4df8bc30293550a1988eb/README.md#security-limitations)

### D. OS egress rules or a filtering resolver

- **From docs.** Linux could apply CIDR rules at the socket or packet boundary using cgroup connect hooks or a per-session namespace and firewall. That can catch direct destinations and DNS changes at connection time. A socket rule needs separate IPv4 and IPv6 hooks and coverage for datagram paths. A packet rule needs coverage for both families and the actual routed or translated destinations. Either design must cover the host proxy’s connections as well as the sandboxed children. On macOS, an Apple Network Extension content filter is a separate system integration with entitlements and deployment requirements. A machine-wide PF rule or a shared user identity lacks an obvious mapping to concurrent profiles. These are higher-cost owner/admin integrations compared with a host broker plus the existing sandbox. [Linux cgroup socket hooks](https://docs.ebpf.io/linux/program-type/BPF_PROG_TYPE_CGROUP_SOCK_ADDR/) · [nftables matching](https://wiki.nftables.org/wiki-nftables/index.php/Matching_packet_metainformation) · [Apple content filter providers](https://developer.apple.com/documentation/networkextension/content-filter-providers)
- **From docs.** A resolver can strip private and link-local answers as defense in depth. This covers only clients and proxies that use that resolver. Literal addresses, hosts-file answers, another resolver and a separate dial-time lookup remain outside a DNS-only rule. An address check can use such a resolver, but must still control the final dial. Resolver deployment is moderate if a trusted DNS service already exists and adds another policy surface otherwise. [dnsmasq rebinding options](https://dnsmasq.org/docs/dnsmasq-man.html) · [SRT dial-time lookup](https://github.com/anthropic-experimental/sandbox-runtime/blob/0bbebed0aa9a8e628dc4df8bc30293550a1988eb/src/sandbox/resolved-address-guard.ts)

## Evidence and remaining checks

### Linux container

**Probed.** Run the following from the repository root. It requires no network inside the container and never contacts a real metadata service. [Fixture](../probes/local-network.py)

```sh
docker run --rm --network none -i python@sha256:e48f59cdba69a3cbd9ed2cf0f348d90d45a8d350d98134cf1070bc106ee9ed90 python -I - < probes/local-network.py
```

```text
platform: Linux x86_64, Python 3.13.18
entry check: 14 obvious entries denied; permitted.example accepted
fixture DNS loopback-v4 127.1.2.3: address-denied
fixture DNS loopback-v6 ::1: address-denied
fixture DNS private-10 10.1.2.3: address-denied
fixture DNS private-172 172.31.1.2: address-denied
fixture DNS private-192 192.168.1.2: address-denied
fixture DNS private-v6 fd00::1: address-denied
fixture DNS link-local-v4 169.254.1.2: address-denied
fixture DNS link-local-v6 fe80::1: address-denied
fixture DNS metadata 169.254.169.254: address-denied
fixture DNS mapped-private ::ffff:10.1.2.3: address-denied
fixture DNS mapped-loopback ::ffff:7f00:1: address-denied
fixture DNS mapped-metadata ::ffff:a9fe:a9fe: address-denied
mixed answer then changed answer: dial 93.184.216.34; address-denied
loopback fixture: denied=address-denied allowed=HTTP 204 direct=HTTP 204 hits=2
```

**Probed, cloud Linux isolation attempt.** `sh probes/local-network-isolation.sh` exited 2 because bubblewrap failed before the enclosed command ran. The same script can exercise a minimal Seatbelt profile on a macOS runner. [Isolation probe](../probes/local-network-isolation.sh)

```text
bubblewrap 0.12.0
control client: connected
bwrap: setting up uid map: Read-only file system
wrapped client did not complete (exit 1)
```

**From docs.** Evidence needed before the grant ships is a pinned Claude Code run on macOS and Linux with a working sandbox, controlled DNS answers, held listeners, both HTTP and SOCKS paths, and a direct client control. Check denied and allowed modes, mixed and changed answers, IPv6, mapped IPv4, and isolated metadata and private-address fixtures. The existing factory macOS gate does not exercise those paths. This spike uses the recorded macOS observation and makes no new macOS measurement. [Factory CI](../.github/workflows/factory-ci.yml) · [Prior Claude probe](../proto/claude-probes/probes.py)

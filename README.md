# Ж / zh + Positronic — Distributed Private Cloud

> **Alpha 0.1 starter, Halloween 2026.** Your hardware. Your network. Your cloud.
>
> This is an *executable bootstrap*, **not** a functioning distributed cloud. It gives us a real local resource map, a Rust `zh` launcher backed by actual Bash, and a minimal Go control-plane scaffold. The node-to-node network, federation, and workload scheduling are not yet built.

## Quick start: see the resource map **without installing Rust**

On Linux or **inside WSL2**, with Python 3.9+:

```bash
unzip parallax-zh-starter.zip
cd parallax-zh-starter
python3 scripts/preview.py
```

Open the URL printed in the terminal (it includes a temporary, random session token). The page reads **real CPU/RAM/root filesystem/network measurements from your current Linux environment** and updates every two seconds; no fake connected nodes or scheduling status. On WSL2, those readings describe the *WSL virtual machine*, **not all resources of the Windows host**. GPU discovery is not implemented.

The preview works without Go, Rust, Node, npm, or a Chromium source build. Press `Ctrl+C` to stop it. To print the URL without opening a browser, add `--no-open`.

## Build the native `zh` executable

**Prerequisites:** Linux x86-64 or WSL2; GNU Bash; Rust stable with `cargo`; optionally Go 1.23+ to test the Positronic placeholder; any Chromium-compatible browser for the graphical map. Rust installation instructions: https://rustup.rs/ . On WSL2, use WSLg with a Linux browser or install `wslu` for `wslview` to open the Windows browser.

```bash
cd parallax-zh-starter
bash scripts/bootstrap.sh
./target/release/zh --help
./target/release/zh --resources
./target/release/zh
```

`bootstrap.sh` builds Rust in release mode, runs Rust tests, and runs Go tests *if Go is installed*. It does not automatically install a Rust compiler, browsers, or dependencies from opaque shell scripts. `zh` uses your existing `bash` program (default lookup from `PATH`), so Bash needs to be installed.

If you want the native command on `PATH`:

```bash
mkdir -p "$HOME/.local/bin"
install -m755 target/release/zh "$HOME/.local/bin/zh"
# Add "$HOME/.local/bin" to PATH if necessary.
```

To point `zh` to your own Parallax/Chromium binary:

```bash
PARALLAX_CHROMIUM=/absolute/path/to/chrome zh
```

By default `zh` tries Chromium, Chrome, and Edge, then `wslview`, then `xdg-open`; if none work, it prints a local URL to open manually. **We don't need to compile the full Parallax Chromium fork just to use the map.**

### Supported commands today

| Command | Behavior |
| --- | --- |
| `zh` | Starts a local resource-map HTTP server, tries to open Chromium in app mode, then enters a **persistent interactive GNU Bash** session in the launching terminal |
| `zh --headless` | Persistent interactive Bash with no GUI |
| `zh --map` | Read-only resource map without opening a shell; Ctrl+C exits |
| `zh --resources` | Prints a real local telemetry snapshot as JSON |
| `zh -c 'echo hello'` | Delegates a command to Bash, preserving its semantics |
| `zh script.sh arg1` | Runs an existing Bash script with arguments |
| `zh --help` / `zh --version` | Help and version |

**Explicitly unavailable:** `.zh`/ZhQL execution, `zzh`, browser-based interactive terminal (PTY), remote machine enrollment, resource pooling, peer discovery, scheduling, recovery, GPU inventory, storage replication, federation, marketplace, and settlement. These commands/features are *planned*, not silently emulated.

### Bash compatibility means GNU Bash is doing the work

The alpha **delegates to the real Bash executable**, which is the straightforward way to preserve pipelines, functions, variables, aliases, `cd`, shell startup files, and job control in a persistent session. It does not assert a new Rust parser is already 100% Bash-compatible.

The browser shows telemetry and a terminal cheat sheet. **You type commands in the terminal that launched `zh`**, not into the browser yet. Later, we can attach a PTY behind a properly authorized local IPC service.

## Run the separate Positronic Go scaffold

From another terminal:

```bash
cd services/positronic
go test ./...
go run .
# In a second terminal:
curl http://127.0.0.1:9460/healthz
```

This intentionally serves only `GET /healthz`, returning `state: "scaffold_only"`. It is *not wired to the Rust agent or the map*, and it does not schedule jobs. Any other route returns `501 Not Implemented`. The listener is restricted to the local loopback interface pending proper identity, authorization, and transport security.

## Repository map

```text
parallax-zh-starter/
├── Cargo.toml                        Rust workspace
├── crates/zh/                        One Rust binary crate (not four invented crates)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                   CLI and browser launch
│       ├── shell.rs                  Bash delegation
│       ├── resources.rs              Linux /proc and /sys resource readings
│       └── http.rs                   Read-only local HTTP API and static app
├── apps/resource-map/                Zero-dependency Chromium-compatible UI
│   ├── index.html
│   ├── styles.css
│   └── app.js
├── services/positronic/              Separate Go control-plane stub
│   ├── go.mod
│   ├── main.go
│   └── main_test.go
├── scripts/
│   ├── bootstrap.sh                  Build and test
│   └── preview.py                    No-Rust local preview
└── tests/
    └── test_preview.py              Auth, telemetry and UI tests
```

**Implementation choices:** We deliberately started with **one** Rust crate. `resources`, `shell`, and `http` are modules of it, not separate packages to maintain. No React/Helm build pipeline yet; the map is dependency-free HTML/CSS/JS so you can see the concept immediately and move it to the existing Parallax Chromium/Helm infrastructure later. The Go module path `example.com/parallax/positronic` is a **placeholder**, to be changed when you choose the real repository path.

## Trust boundary (read before exposing anything)

- The **first wave is a distributed *private* cloud**, built from machines the owner controls or explicitly trusts. There is no public contributed-compute marketplace in this starter.
- This alpha binds its web servers to `127.0.0.1` only. It is **read-only**: the browser cannot run commands or mutate local configuration.
- Telemetry requires a per-startup 256-bit random bearer token, passed in a URL fragment and then in an API authorization header. The server checks the `Host` header against the assigned loopback port. It's sufficient for a *local alpha demo*, **not a complete security design**.
- Do **not** forward the local port to the internet, expose it via reverse proxies, or give arbitrary websites access to the URL/token. Prefer a Unix domain socket with authenticated session bridging in the next iteration. No background daemon or remote execution is installed.
- Browser UI is served with a restrictive Content Security Policy, `no-store`, and no third-party script or stylesheet dependencies.
- The snapshot intentionally contains hostname and machine resource information. Treat local telemetry as sensitive operational metadata.

## Tests and validation

```bash
python3 -m unittest discover -s tests -v
(cd services/positronic && go test ./...)
node --check apps/resource-map/app.js          # optional
bash -n scripts/bootstrap.sh
cargo test -p zh                               # requires Cargo
```

Before packaging, the Python preview's four tests, the Go service tests, JavaScript syntax check and Bash syntax check passed in the packaging environment. **Rust compilation/tests were not run here because `cargo` was not installed.** The Rust source is provided for you/Codex to compile and validate on your machine; do not assume the binary is already built.

## Next milestones — in order

1. **Validate Rust/WSL2 launch.** Install Rust, run `bash scripts/bootstrap.sh`, fix any compiler or WSL/browser-launch issues; require `zh --resources` and `zh` to behave as documented.
2. **Native `zh` session & IPC.** Keep Bash as the compatibility substrate; introduce an authenticated Unix-socket local control service and a PTY bridge for the in-browser terminal. Distinguish Bash execution from the forthcoming ZhQL workflows. No general-purpose command execution over an unauthenticated HTTP endpoint.
3. **Node identity & enrollment.** Durable keys and `fabric_id`/`node_id` identity, explicit pairing, trust revocation, encryption, bounded resource leases. Nodes private by default.
4. **Positronic ↔ Rust agent contract.** Versioned API/schema, resource advertising, stale/offline status, CPU/RAM quotas and scheduler eligibility. Plexus owns relationship/topology data; Positronic owns scheduling/control.
5. **Two nodes, one job.** Start with a bounded CPU batch job and verified result, not GPU inference or arbitrary code from strangers.
6. **Three-node failure test.** Disconnect a worker, retry idempotently, prevent duplicate accepted results, finish the job with correct output.
7. **Private federation.** Connect separately administered trusted fabrics with explicit policies. Public participation and settlement are optional *later* extensions.

ZhQL is **the single declarative workflow language**, with surface syntax to be derived from the existing Archer Cookbook, not an invented second scripting dialect. `zh` remains a native shell/runtime even when the Chromium window is closed. We are not replacing Kubernetes with Slipstream: **Positronic is the control plane**; Slipstream is developer/build/deploy integration; Plexus handles peer topology and trust.

## Scope & ownership

This is a **locally generated proof-of-concept skeleton** for Parallax/Positronic and is not the user's existing production codebase. It includes no credentials, keys, copied private repositories, real network access, payment features, or deployment credentials. No open-source license has been selected or implied; add one only when you've chosen it.

**Happy Halloween.** 🎃 Build the smallest ghost of a cloud, then make it haunt a second machine.

#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

printf '\n  Ж  PARALLAX / POSITRONIC\n'
printf '  Distributed Private Cloud — alpha bootstrap\n\n'

if [[ "$(uname -s)" != "Linux" ]]; then
  printf 'This alpha currently targets Linux and WSL2.\n' >&2
  exit 1
fi

if ! command -v bash >/dev/null 2>&1; then
  printf 'Bash is required for compatibility mode.\n' >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  printf 'Rust/Cargo missing. Install the stable Rust toolchain: https://rustup.rs/\n' >&2
  printf 'You can preview the resource map without Rust using:\n'
  printf '    python3 scripts/preview.py\n\n'
  exit 2
fi

printf '[1/3] Build Rust zh\n'
cargo build --release -p zh
printf '\n[2/3] Rust tests\n'
cargo test -p zh

if command -v go >/dev/null 2>&1; then
  printf '\n[3/3] Positronic Go scaffold tests\n'
  (cd services/positronic && go test ./...)
else
  printf '\n[3/3] Skipping Go scaffold tests (Go not installed)\n'
fi

printf '\nBootstrap complete. Start with:\n'
printf '  %s/target/release/zh --resources\n' "$ROOT"
printf '  %s/target/release/zh\n\n' "$ROOT"
printf 'To place zh on PATH (optional):\n'
printf '  install -Dm755 target/release/zh "$HOME/.local/bin/zh"\n'

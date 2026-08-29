#!/usr/bin/env bash
# CI-stable 9router parity smoke (no live provider keys).
set -euo pipefail
cd "$(dirname "$0")/.."
echo "== parity smoke =="
cargo test -p cipherroute --lib stream_flags -- --nocapture
cargo test -p cipherroute --lib parity_tests -- --nocapture
cargo test -p cipherroute --lib claude_format -- --nocapture
cargo test -p cipherroute --lib combo -- --nocapture
cargo test -p cipherroute --lib chat:: -- --nocapture
cargo test -p cipherroute --lib error_config -- --nocapture
echo "OK"

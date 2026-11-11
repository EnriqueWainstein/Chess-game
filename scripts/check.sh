#!/usr/bin/env bash
set -euo pipefail

cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check

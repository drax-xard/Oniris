#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- gen-test-packs packs

#!/usr/bin/env bash
# Linux build + Rust tests from Windows, before pushing: `npm run build`, then `npm run test:linux`.
# Copies the repo into the Linux filesystem (building on /mnt/d is very slow) and reuses the Windows-built dist/.
# src-tauri/llama/ is left alone: on Windows it holds the Windows engine, in WSL the Linux one.
set -euo pipefail
src="$(cd "$(dirname "$0")/.." && pwd)"
rsync -a --delete --exclude node_modules --exclude src-tauri/target --exclude src-tauri/llama --exclude .git "$src/" ~/cursora/
cd ~/cursora
~/.cargo/bin/cargo test --manifest-path src-tauri/Cargo.toml "$@"

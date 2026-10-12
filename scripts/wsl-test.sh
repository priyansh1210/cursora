#!/usr/bin/env bash
# Linux build + Rust tests from Windows, before pushing: `npm run build`, then `npm run test:linux`.
# Copies the repo into the Linux filesystem (building on /mnt/d is very slow) and reuses the Windows-built dist/.
set -euo pipefail
src="$(cd "$(dirname "$0")/.." && pwd)"
rsync -a --delete --exclude node_modules --exclude src-tauri/target --exclude .git "$src/" ~/cursora/
cd ~/cursora
~/.cargo/bin/cargo test --manifest-path src-tauri/Cargo.toml "$@"

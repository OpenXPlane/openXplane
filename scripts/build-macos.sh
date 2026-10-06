#!/usr/bin/env bash
# Builds the release binary and packages it into dist/macos/ (needs the latest Rust stable).
set -euo pipefail
cd "$(dirname "$0")/.."
version="$(scripts/version.sh)"
cargo build --locked --release
out="dist/macos"
rm -rf "$out" && mkdir -p "$out"
cp target/release/openxplane README.md LICENSE "$out/"
cp -r examples "$out/"
echo "openXplane $version -> $out/"

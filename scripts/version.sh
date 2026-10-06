#!/usr/bin/env bash
# Prints the version MAJOR.MINOR.COMMITS: VERSION holds MAJOR.MINOR, the commit count is the patch.
set -euo pipefail
cd "$(dirname "$0")/.."
echo "$(tr -d '[:space:]' < VERSION).$(git rev-list --count HEAD 2>/dev/null || echo 0)"

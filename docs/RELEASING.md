# Releasing

- The version is `MAJOR.MINOR.COMMITS`: `VERSION` holds `MAJOR.MINOR`, the commit count is the patch number
  (`scripts/version.sh`). Change `VERSION` for a new minor release.
- Every push to `main` that changes code runs the `release` workflow: it builds Linux, macOS (arm64) and
  Windows archives with `cargo build --locked --release` and replaces the rolling **nightly** release.
- The `ci` workflow checks formatting, clippy and the tests on all three systems; it must be green before
  a change is merged.
- Notable changes go to [CHANGELOG.md](../CHANGELOG.md) under the current version.
- The website (`site/`) is published from `main` by the `pages` workflow; it copies the documents and
  research notes next to the page, so a document change appears there with the next publish.

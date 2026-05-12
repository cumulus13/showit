# Changelog

All notable changes to **showit** will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

## [0.1.0] — 2025-12-15

### Added
- Initial release
- Substring / wildcard (`*`, `?`) / full-regex (`-r`) title search
- Interactive REPL: focus by number, close with `[n]c`, re-search, quit with `x`/`q`
- True-colour hex palette for window titles (configurable via `config.toml`)
- In-title match highlighting in configurable colour
- `--list` flag to show all visible windows
- `--init-config` / `--config-path` flags for easy config setup
- Cross-platform stubs for CI testing on Linux/macOS
- GitHub Actions: CI (test + clippy + fmt), cross-compile Windows binary, publish to crates.io on tag
- Dependabot for automatic dependency and Actions updates

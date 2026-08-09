# Changelog

All notable changes to **showit** will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

## [0.1.16] — 2026-08-08

### Added
- `-f, --focus` flag: by default a selected window is only *raised*
  (Z-order/visibility change, `SWP_NOACTIVATE`) and keyboard focus stays
  wherever it was. Pass `-f`/`--focus` to actually *activate* the window
  (`SetForegroundWindow`), stealing input focus the same way Alt-Tab does.
  Applies to the single-match auto-raise path and to picking a number in the
  interactive list.

### Changed
- Clarified `--help` and the in-list hint to distinguish "raise" (default)
  from "focus" (`-f`).

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

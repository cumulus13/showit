# Changelog

All notable changes to **showit** will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Fixed
- **Focus didn't actually switch when showit was launched from `cmd.exe`** (worked fine from Windows Terminal). The old `bring_to_front` deliberately used `SW_SHOWNA`/`SWP_NOACTIVATE` — it never called `SetForegroundWindow` at all, only reordered z-order. Fixed by actually calling `SetForegroundWindow`/`BringWindowToTop`, satisfied via `AttachThreadInput` to the current foreground thread (already the right sequence for this), with a minimize/restore fallback for stubborn cases (e.g. elevated targets).
  - v1 of this fix used a synthetic ALT keypress to satisfy the foreground lock; removed after it caused focus to snap back (the synthetic keyup landed on the newly-foregrounded window). v2 had a build-breaking typo (`SW_MINIMIZE` used but not imported). v3 adds a short settle delay before the process exits (showit returning/exiting instantly after `SetForegroundWindow` can let the previous foreground app reclaim focus before the OS finishes the transition) plus `SHOWIT_DEBUG=1` diagnostics printed to stderr for further troubleshooting.
- `--list` / `-l` flag was missing from the CLI (regression — it was documented in the README/CHANGELOG but never implemented in `Args`), so `showit --list` failed with an argument error and `PATTERN` was always required.
- Colored output printed raw ANSI escape codes instead of colors when run from plain `cmd.exe` (Windows Terminal/PowerShell enable VT processing by default; legacy `cmd.exe` does not). Now enabled explicitly at startup via `colored::control::set_virtual_terminal(true)`.
- Window list would include showit's own console/host window (e.g. when the query matched the console's title, such as "cmd" or the current directory), since `enumerate_windows()` never excluded it. Now filtered out via `GetConsoleWindow()`.

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

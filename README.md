# showit

> Bring any Windows window to the foreground by searching its title — fast, coloured, interactive.

[![CI](https://github.com/cumulus13/showit/actions/workflows/ci.yml/badge.svg)](https://github.com/cumulus13/showit/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/showit.svg)](https://crates.io/crates/showit)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

## Features

- **Substring / wildcard / regex search** — plain text, `*`/`?` globs, or full regex with `-r`
- **Vivid hex-colour output** — each title gets its own colour from a configurable palette; the matched portion is highlighted
- **Interactive REPL** — after results appear, type a number to focus, `[n]c` to close, or any text to search again
- **Config file** — `config.toml` lets you set any colour in the palette, the index colour, match highlight, errors, success messages
- **Single static binary** — no runtime dependencies; ships as a native Windows `.exe`

---

## Installation

### From crates.io

```sh
cargo install showit
```

### From source

```sh
git clone https://github.com/cumulus13/showit
cd showit
cargo build --release
# binary at: target/release/showit.exe
```

---

## Usage

```
showit [OPTIONS] [PATTERN]
```

| Argument / Flag   | Description                                         |
|-------------------|-----------------------------------------------------|
| `PATTERN`         | Title to search (substring by default)              |
| `-r, --regex`     | Treat PATTERN as a regular expression               |
| `-l, --list`      | List **all** visible windows                        |
| `--config-path`   | Print the path to the config file                   |
| `--init-config`   | Write a default config file (won't overwrite)       |
| `-h, --help`      | Print help                                          |
| `-V, --version`   | Print version                                       |

### Examples

```sh
# Substring match (case-insensitive)
showit firefox

# Wildcard — any window whose title starts with "Visual"
showit "Visual*"

# Regex — windows containing "term" or "bash"
showit -r "term|bash"

# List every visible window
showit --list
```

### Interactive commands

After results are displayed:

| Input       | Effect                                        |
|-------------|-----------------------------------------------|
| `3`         | Bring window #3 to the front                  |
| `2c`        | **Close** window #2 (sends WM_CLOSE)          |
| `notepad`   | Start a new search for "notepad"              |
| `x` / `q`  | Quit                                          |

---

## Configuration

Run `showit --init-config` to create a starter config, then open it:

```sh
showit --config-path   # prints the path
```

**Windows:** `%APPDATA%\showit\config.toml`  
**Linux/macOS:** `~/.config/showit/config.toml`

### Example `config.toml`

```toml
# showit configuration file
# Colors must be valid hex strings: "#RRGGBB" or "#RGB"

# Palette cycled through for each window title in the list
colors = [
    "#00FFFF",   # Cyan
    "#FF6B6B",   # Coral red
    "#FFD700",   # Gold
    "#7CFC00",   # Lawn green
    "#FF69B4",   # Hot pink
    "#00CED1",   # Dark turquoise
    "#FFA500",   # Orange
    "#DA70D6",   # Orchid
    "#ADFF2F",   # Green-yellow
    "#1E90FF",   # Dodger blue
    "#FF4500",   # Orange-red
    "#98FB98",   # Pale green
]

# Color for the index numbers (e.g.  1.  2.  3.)
index_color   = "#888888"

# Color for the prompt line
prompt_color  = "#00BFFF"

# Color used to highlight the matched portion inside a title
match_color   = "#FFFF00"

# Color for error messages
error_color   = "#FF4444"

# Color for success messages (focused / closed confirmations)
success_color = "#44FF88"
```

---

## Architecture

```
src/
├── main.rs        — CLI args (clap), interactive REPL, command dispatch
├── config.rs      — Config struct, TOML load/write, hex-color parsing
├── display.rs     — Coloured output helpers (colorize, highlight_match, …)
├── search.rs      — Substring / wildcard / regex filtering
└── windows_api.rs — Win32 EnumWindows, SetForegroundWindow, PostMessage(WM_CLOSE)
                     (non-Windows stub for CI)
```

---

## Development

```sh
cargo build          # debug build (cross-compiles on Linux for testing)
cargo test           # 12 unit tests, all platform-safe
cargo clippy         # lints
cargo build --release --target x86_64-pc-windows-msvc   # final Windows binary
```

---

## License

MIT — see [LICENSE](LICENSE).

---

## 👤 Author
        
[Hadi Cahyadi](mailto:cumulus13@gmail.com)
    

[![Buy Me a Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/cumulus13)

[![Donate via Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/cumulus13)
 
[Support me on Patreon](https://www.patreon.com/cumulus13)
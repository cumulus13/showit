// main.rs — showit
// Author  : Hadi Cahyadi <cumulus13@gmail.com>
// Homepage: https://github.com/cumulus13/showit
// License : MIT
//
// EXACT same logic as original C# showme, converted to Rust, with added features:
//   - hex colored titles (different color per title)
//   - config file for custom colors
//   - [n]c = close window n
//   - [n]f = raise window n in focused mode (steals focus)
//   - typing any text (not a number) = new search with that text
//   - x/q = quit

mod config;
mod display;
mod search;
mod windows_api;

use anyhow::Result;
use clap::Parser;
use clap_version_flag::colorful_version;
use config::Config;
use display::{colorize, print_error, print_success};
use search::{detect_mode, filter_windows};
use std::io::{self, BufRead, Write};
use windows_api::{
    bring_to_front, close_window, enumerate_windows, run_delayed_raise_and_exit, WindowInfo,
};

#[derive(Parser, Debug)]
#[command(
    name = "showit",
    author = "Hadi Cahyadi <cumulus13@gmail.com>",
    version,
    about = "Search open window titles and bring one to the foreground",
    long_about = "Usage: showit <pattern>\n\n\
    Searches visible window titles. If one match: raise immediately.\n\
    If multiple: show list, pick a number.\n\n\
    After list is shown:\n\
      <number>   raise that window (then quit)\n\
      <n>f       raise that window IN FOCUSED MODE, stealing focus\n\
                 regardless of -f/--focus (then quit)\n\
      <n>c       close window n   (then quit)\n\
      <text>     new search\n\
      x / q      quit\n\n\
    By default the window is only raised to the top (its z-order/visibility\n\
    changes but keyboard focus is left wherever it was). Pass -f/--focus to\n\
    make every raise activate it (SetForegroundWindow) instead, stealing\n\
    input focus too — or use the per-pick <n>f form above for a one-off.\n\n\
    Homepage: https://github.com/cumulus13/showit"
)]
struct Args {
    /// Window title pattern (substring / wildcard * / regex with -r)
    pattern: String,

    /// Treat pattern as a regular expression
    #[arg(short, long)]
    regex: bool,

    /// Activate (focus) the window instead of only raising it to the top
    #[arg(short, long)]
    focus: bool,

    /// Print the config file path and exit
    #[arg(long)]
    config_path: bool,

    /// Write a default config file and exit
    #[arg(long)]
    init_config: bool,
}

enum PickAction {
    // Continue,
    NewSearch(String),
    Quit,
}

fn main() -> Result<()> {
    let os_args: Vec<String> = std::env::args().collect();

    // Hidden internal entry point: bring_to_front() re-execs itself as
    // `showit --__raise <hwnd> <focus>`, detached and delayed, so the actual
    // raise happens *after* the parent console (conhost) has already
    // reclaimed its own foreground on exit. See windows_api::bring_to_front
    // for the full explanation. Not a documented/public flag — never returns.
    if os_args.len() == 4 && os_args[1] == "--__raise" {
        if let Ok(hwnd) = os_args[2].parse::<usize>() {
            let focus = os_args[3] == "1";
            run_delayed_raise_and_exit(hwnd, focus);
        }
        std::process::exit(1);
    }

    if os_args.len() == 2 && (os_args[1] == "-V" || os_args[1] == "--version") {
        let version = colorful_version!();
        version.print_and_exit();
    }
    let args = Args::parse();
    let cfg = Config::load()?;

    if args.config_path {
        println!("{}", Config::config_path().display());
        return Ok(());
    }
    if args.init_config {
        let path = Config::config_path();
        if path.exists() {
            println!("Config already exists: {}", path.display());
        } else {
            Config::write_default(&path)?;
            println!("Written: {}", path.display());
        }
        return Ok(());
    }

    run(&args.pattern, args.regex, args.focus, &cfg)
}

fn run(pattern: &str, force_regex: bool, focus: bool, cfg: &Config) -> Result<()> {
    let mut current_query = pattern.to_string();

    loop {
        // 1. Enumerate visible windows (same as C# EnumWindows + IsWindowVisible)
        let all = enumerate_windows()?;

        // 2. Filter by pattern
        let mode = detect_mode(&current_query, force_regex);

        let matched: Vec<&WindowInfo> =
            filter_windows(&all, &current_query, &mode).unwrap_or_default();

        match matched.len() {
            // ── No match ──────────────────────────────────────────────────────
            0 => {
                print_error(
                    &format!("No matching windows found for '{}'.", current_query),
                    cfg,
                );
            }

            // ── Single match: focus and quit ──────────────────────────────────
            1 => {
                let win = matched[0];
                match bring_to_front(win, focus) {
                    Ok(()) => print_success(&format!("{} brought to the front.", win.title), cfg),
                    Err(e) => print_error(&format!("Failed: {}", e), cfg),
                }
                // Action taken → quit
                return Ok(());
            }

            // ── Multiple matches: show list, read selection ────────────────────
            _ => {
                show_list(&matched, &current_query, cfg);

                match pick_loop(&matched, focus, cfg)? {
                    PickAction::NewSearch(new_query) => {
                        current_query = new_query;
                        continue;
                    }
                    PickAction::Quit => return Ok(()),
                }
            }
        }

        // After no-match: prompt for a new search or quit
        print!("{} ", colorize("Search (x/q to quit):", &cfg.prompt_color));
        io::stdout().flush()?;

        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let input = line.trim();

        if input.is_empty() {
            continue;
        }

        if matches!(input.to_lowercase().as_str(), "x" | "q" | "exit" | "quit") {
            break;
        }

        current_query = input.to_string();
    }

    Ok(())
}

/// Format a single window entry for the list.
///
/// For windows whose title is unreliable (e.g. `wt`, `pwsh`, `vim`) the
/// type label is shown prominently on the left; the raw title becomes a
/// secondary "subtitle" in the index colour so the user can still
/// distinguish tabs / sessions:
///
///   `  3.  [Windows Terminal]  my-project`
///
/// For normal windows the existing doc / app / user split is kept:
///
///   `  3.  readme.txt - Notepad`
fn format_entry(win: &WindowInfo, cfg: &Config) -> String {
    if win.title_is_unreliable() {
        let label = win.type_label();
        let type_part = colorize(&format!("[{}]", label), &cfg.app_color);
        let title_part = colorize(&win.title, &cfg.index_color);
        format!("{}  {}", type_part, title_part)
    } else {
        // Original doc / app / user split on the LAST " - "
        match win.title.rfind(" - ") {
            Some(pos) => {
                let doc = colorize(&win.title[..pos], &cfg.doc_color);
                let sep = colorize(" - ", &cfg.index_color);
                let app_full = &win.title[pos + 3..];

                // Detect a trailing "(Something)" user tag
                let app_colored = if app_full.ends_with(')') {
                    if let Some(tag_start) = app_full.rfind(" (") {
                        let app_name = colorize(&app_full[..tag_start], &cfg.app_color);
                        let user_tag = colorize(&app_full[tag_start + 1..], &cfg.user_color);
                        format!("{} {}", app_name, user_tag)
                    } else {
                        colorize(app_full, &cfg.user_color)
                    }
                } else {
                    colorize(app_full, &cfg.app_color)
                };

                format!("{}{}{}", doc, sep, app_colored)
            }
            None => colorize(&win.title, &cfg.doc_color),
        }
    }
}

/// Print the numbered, colored window list.
fn show_list(windows: &[&WindowInfo], query: &str, cfg: &Config) {
    println!(
        "\n{} windows match '{}':\n",
        colorize(&windows.len().to_string(), &cfg.index_color),
        colorize(query, &cfg.match_color)
    );

    for (i, win) in windows.iter().enumerate() {
        let index = colorize(&format!("{:>3}.", i + 1), &cfg.index_color);
        let entry = format_entry(win, cfg);
        println!("{}  {}", index, entry);
    }

    println!();
    println!(
        "{}",
        colorize(
            "  [number] raise (quit)   [n]f focus (quit)   [n]c close (quit)   [text] new search   x quit",
            &cfg.index_color
        )
    );
    println!();
}

/// Read one line from stdin and dispatch:
///   number   → raise (respects launch-time -f/--focus) → **Quit**
///   [n]f     → raise IN FOCUSED MODE (steals focus) → **Quit**
///   [n]c     → close → **Quit**
///   text     → NewSearch
///   x/q      → Quit
fn pick_loop(windows: &[&WindowInfo], focus: bool, cfg: &Config) -> Result<PickAction> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!(
            "{} ",
            colorize(&format!("Select 1-{}:", windows.len()), &cfg.prompt_color)
        );

        stdout.flush()?;

        let mut line = String::new();

        stdin.lock().read_line(&mut line)?;

        let input = line.trim();

        if input.is_empty() {
            continue;
        }

        // quit
        if matches!(input.to_lowercase().as_str(), "x" | "q" | "exit" | "quit") {
            return Ok(PickAction::Quit);
        }

        // [n]c → close → quit
        if let Some(num_str) = input.strip_suffix('c').or_else(|| input.strip_suffix('C')) {
            if let Ok(n) = num_str.trim().parse::<usize>() {
                if n >= 1 && n <= windows.len() {
                    let win = windows[n - 1];

                    match close_window(win) {
                        Ok(()) => print_success(&format!("{} closed.", win.title), cfg),
                        Err(e) => print_error(&format!("Failed to close: {}", e), cfg),
                    }
                } else {
                    print_error(&format!("Invalid: enter 1-{}.", windows.len()), cfg);
                    continue; // bad number: re-prompt
                }
                return Ok(PickAction::Quit); // ← quit after close
            }
            // suffix 'c' but non-numeric prefix → fall through to NewSearch
        }

        // [n]f → raise IN FOCUSED MODE (steals focus regardless of the
        // launch-time -f/--focus setting) → quit
        if let Some(num_str) = input.strip_suffix('f').or_else(|| input.strip_suffix('F')) {
            if let Ok(n) = num_str.trim().parse::<usize>() {
                if n >= 1 && n <= windows.len() {
                    let win = windows[n - 1];
                    match bring_to_front(win, true) {
                        Ok(()) => print_success(
                            &format!("{} brought to the front (focused).", win.title),
                            cfg,
                        ),
                        Err(e) => print_error(&format!("Failed: {}", e), cfg),
                    }
                } else {
                    print_error(&format!("Invalid: enter 1-{}.", windows.len()), cfg);
                    continue; // bad number: re-prompt
                }
                return Ok(PickAction::Quit); // ← quit after focus
            }
            // suffix 'f' but non-numeric prefix → fall through to NewSearch
        }

        // plain number → raise (launch-time -f/--focus setting applies) → quit
        if let Ok(n) = input.parse::<usize>() {
            if n >= 1 && n <= windows.len() {
                let win = windows[n - 1];
                match bring_to_front(win, focus) {
                    Ok(()) => print_success(&format!("{} brought to the front.", win.title), cfg),
                    Err(e) => print_error(&format!("Failed: {}", e), cfg),
                }
                return Ok(PickAction::Quit); // ← quit after focus
            } else {
                print_error(&format!("Invalid: enter 1-{}.", windows.len()), cfg);
                continue; // bad number: re-prompt
            }
        }

        // anything else → new search
        return Ok(PickAction::NewSearch(input.to_string()));
    }
}

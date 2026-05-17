// main.rs — showit
// Author  : Hadi Cahyadi <cumulus13@gmail.com>
// Homepage: https://github.com/cumulus13/showit
// License : MIT
//
// EXACT same logic as original C# showme, converted to Rust, with added features:
//   - hex colored titles (different color per title)
//   - config file for custom colors
//   - [n]c = close window n
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
use windows_api::{bring_to_front, close_window, enumerate_windows, WindowInfo};

#[derive(Parser, Debug)]
#[command(
    name = "showit",
    author = "Hadi Cahyadi <cumulus13@gmail.com>",
    version,
    about = "Search open window titles and bring one to the foreground",
    long_about = "Usage: showit <pattern>\n\n\
    Searches visible window titles. If one match: focus immediately.\n\
    If multiple: show list, pick a number.\n\n\
    After list is shown:\n\
      <number>   focus that window\n\
      <n>c       close window n (e.g. 2c)\n\
      <text>     new search\n\
      x / q      quit\n\n\
    Homepage: https://github.com/cumulus13/showit"
)]
struct Args {
    /// Window title pattern (substring / wildcard * / regex with -r)
    pattern: String,

    /// Treat pattern as a regular expression
    #[arg(short, long)]
    regex: bool,

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

    run(&args.pattern, args.regex, &cfg)
}

fn run(pattern: &str, force_regex: bool, cfg: &Config) -> Result<()> {
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

            // ── Single match: focus and done (exact original behaviour) ───────
            1 => {
                let win = matched[0];

                match bring_to_front(win) {
                    Ok(()) => {
                        print_success(&format!("{} brought to the front.", win.title), cfg);
                    }
                    Err(e) => {
                        print_error(&format!("Failed: {}", e), cfg);
                    }
                }
            }

            // ── Multiple matches: show list, read selection ────────────────────
            _ => {
                show_list(&matched, &current_query, cfg);

                match pick_loop(&matched, cfg)? {
                    PickAction::NewSearch(new_query) => {
                        current_query = new_query;
                        continue;
                    }

                    PickAction::Quit => {
                        break;
                    }
                }
            }
        }

        // prompt after single/no result too
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

/// Print the numbered, colored window list.
/// Each title is split on the LAST " - ":
///   left  (document/file path) → cycling palette color per entry
///   right (app name)           → fixed app_color from config
fn show_list(windows: &[&WindowInfo], query: &str, cfg: &Config) {
    println!(
        "\n{} windows match '{}':\n",
        colorize(&windows.len().to_string(), &cfg.index_color),
        colorize(query, &cfg.match_color)
    );

    for (i, win) in windows.iter().enumerate() {
        let index = colorize(&format!("{:>3}.", i + 1), &cfg.index_color);

        // Split title into up to 3 fixed-color parts:
        //   "C:\path\file (project) - Sublime Text (ADMIN)"
        //    ^^^^^^^^^^^^^^^^^^^^^^    ^^^^^^^^^^^^  ^^^^^^^
        //    doc_color (part1)         app_color     user_color (part3)
        let title_colored = match win.title.rfind(" - ") {
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
        };

        println!("{}  {}", index, title_colored);
    }
    println!();
    println!(
        "{}",
        colorize(
            "  [number] focus   [n]c close   [text] new search   x quit",
            &cfg.index_color
        )
    );
    println!();
}

/// Read one line from stdin and dispatch:
///   number -> focus
///   [n]c   -> close
///   text   -> new search
///   x/q    -> quit
fn pick_loop(windows: &[&WindowInfo], cfg: &Config) -> Result<PickAction> {
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

        // [n]c -> close
        if let Some(num_str) = input.strip_suffix('c').or_else(|| input.strip_suffix('C')) {
            if let Ok(n) = num_str.trim().parse::<usize>() {
                if n >= 1 && n <= windows.len() {
                    let win = windows[n - 1];

                    match close_window(win) {
                        Ok(()) => {
                            print_success(&format!("{} closed.", win.title), cfg);
                        }

                        Err(e) => {
                            print_error(&format!("Failed to close: {}", e), cfg);
                        }
                    }
                } else {
                    print_error(&format!("Invalid: enter 1-{}.", windows.len()), cfg);
                }

                // stay in same list
                continue;
            }
        }

        // plain number -> focus
        if let Ok(n) = input.parse::<usize>() {
            if n >= 1 && n <= windows.len() {
                let win = windows[n - 1];

                match bring_to_front(win) {
                    Ok(()) => {
                        print_success(&format!("{} brought to the front.", win.title), cfg);
                    }

                    Err(e) => {
                        print_error(&format!("Failed: {}", e), cfg);
                    }
                }
            } else {
                print_error(&format!("Invalid: enter 1-{}.", windows.len()), cfg);
            }

            // stay in same list
            continue;
        }

        // anything else -> new search
        return Ok(PickAction::NewSearch(input.to_string()));
    }
}

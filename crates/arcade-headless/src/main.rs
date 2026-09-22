//! Headless entry point for ArcadeEdit.
//!
//! Provides fast, display-independent commands for agents, CI pipelines, and CLI scripting:
//! - `inspect`: document inspection, metric calculation, and syntax detection
//! - `search`: in-memory rope search across files and directories
//! - `replace`: atomic search-and-replace with dry-run and CI check modes

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use arcade_core::{find_all, load_from_file, replace_all, save_to_file};
use arcade_language::LanguageId;
use arcade_protocol::{
    FileReplaceSummary, InspectResult, ReplaceResult, Response, SearchMatch, SearchResult,
    PROTOCOL_VERSION,
};
use arcade_workspace::discover_files;

fn print_help() {
    println!(
        r#"ArcadeEdit Headless CLI (Protocol v{})

USAGE:
    arcade-headless <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    inspect <FILE> [--json]
        Inspect file metrics, line counts, encoding, line endings, and syntax language.

    search <QUERY> [PATHS...] [--json]
        Find all occurrences of a string query across files and directories.

    replace <QUERY> <REPLACEMENT> [PATHS...] [--dry-run] [--check] [--json]
        Perform atomic search-and-replace across files.

FLAGS:
    -h, --help       Print help information
    -V, --version    Print version and protocol information
    --json           Output results as structured, versioned JSON
    --dry-run        Calculate and report replacements without modifying files
    --check          Verify whether matches exist; exits 1 if matches found, 0 if clean

EXAMPLES:
    arcade-headless inspect crates/arcade-core/src/lib.rs
    arcade-headless inspect Cargo.toml --json
    arcade-headless search "TODO" crates/
    arcade-headless replace "old_api" "new_api" src/ --dry-run
    arcade-headless replace "foo" "bar" src/ --check
"#,
        PROTOCOL_VERSION
    );
}

fn print_version() {
    println!(
        "arcade-headless {} (protocol {})",
        env!("CARGO_PKG_VERSION"),
        PROTOCOL_VERSION
    );
}

fn detect_line_ending(text: &str) -> String {
    let has_crlf = text.contains("\r\n");
    let has_lf = text.contains('\n');

    if has_crlf && has_lf {
        // Check if there are bare LFs beside CRLF
        let stripped = text.replace("\r\n", "");
        if stripped.contains('\n') {
            "Mixed (CRLF + LF)".into()
        } else {
            "CRLF".into()
        }
    } else if has_lf {
        "LF".into()
    } else {
        "None".into()
    }
}

fn run_inspect(path_str: &str, json_mode: bool) -> Result<ExitCode, String> {
    let path = PathBuf::from(path_str);
    if !path.exists() {
        return Err(format!("File does not exist: {}", path.display()));
    }
    if !path.is_file() {
        return Err(format!("Path is not a regular file: {}", path.display()));
    }

    let (doc, meta) = load_from_file(&path).map_err(|e| format!("Failed to load file: {e}"))?;
    let raw_text = doc.to_string();
    let byte_size = meta.len_bytes as usize;
    let char_count = doc.rope().len_chars();
    let line_count = doc.len_lines();
    let line_ending = detect_line_ending(&raw_text);

    let language = match LanguageId::from_path(&path) {
        LanguageId::Rust => "Rust",
        LanguageId::Markdown => "Markdown",
        LanguageId::PlainText => "Plain Text",
    };

    let is_read_only = fs::metadata(&path)
        .map(|m| m.permissions().readonly())
        .unwrap_or(false);

    let result = InspectResult {
        path: path.clone(),
        byte_size,
        char_count,
        line_count,
        encoding: "UTF-8".into(),
        line_ending,
        language: language.into(),
        is_read_only,
    };

    if json_mode {
        let envelope = Response::new(result);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope).map_err(|e| e.to_string())?
        );
    } else {
        println!("File:         {}", result.path.display());
        println!(
            "Size:         {} bytes ({:.1} KB)",
            result.byte_size,
            result.byte_size as f64 / 1024.0
        );
        println!("Characters:   {}", result.char_count);
        println!("Lines:        {}", result.line_count);
        println!("Encoding:     {}", result.encoding);
        println!("Line Ending:  {}", result.line_ending);
        println!("Language:     {}", result.language);
        println!("Read Only:    {}", result.is_read_only);
    }

    Ok(ExitCode::SUCCESS)
}

fn run_search(query: &str, paths: &[String], json_mode: bool) -> Result<ExitCode, String> {
    if query.is_empty() {
        return Err("Search query cannot be empty".into());
    }

    let search_roots: Vec<PathBuf> = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths.iter().map(PathBuf::from).collect()
    };

    let files = discover_files(&search_roots);
    let mut matches = Vec::new();
    let files_searched = files.len();

    for file_path in &files {
        if let Ok((doc, _)) = load_from_file(file_path) {
            let found_ranges = find_all(doc.rope(), query);
            let rope = doc.rope();

            for range in found_ranges {
                let line_idx = doc.line_of_byte(range.start);
                let line_start_char = rope.line_to_char(line_idx);
                let match_char = doc.byte_to_char(range.start).unwrap_or(0);
                let col = match_char.saturating_sub(line_start_char) + 1;

                let line_slice = rope.line(line_idx);
                let line_text = line_slice
                    .to_string()
                    .trim_end_matches(['\r', '\n'])
                    .to_string();

                matches.push(SearchMatch {
                    path: file_path.clone(),
                    line: line_idx + 1,
                    column: col,
                    byte_offset: range.start.0,
                    line_text,
                });
            }
        }
    }

    let total_matches = matches.len();
    let result = SearchResult {
        query: query.to_string(),
        total_matches,
        files_searched,
        matches,
    };

    if json_mode {
        let envelope = Response::new(result);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope).map_err(|e| e.to_string())?
        );
    } else {
        for m in &result.matches {
            println!(
                "{}:{}:{}: {}",
                m.path.display(),
                m.line,
                m.column,
                m.line_text
            );
        }
        println!(
            "\nFound {} occurrence{} across {} file{}.",
            result.total_matches,
            if result.total_matches == 1 { "" } else { "s" },
            result.files_searched,
            if result.files_searched == 1 { "" } else { "s" }
        );
    }

    if total_matches > 0 {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}

fn run_replace(
    query: &str,
    replacement: &str,
    paths: &[String],
    dry_run: bool,
    check_mode: bool,
    json_mode: bool,
) -> Result<ExitCode, String> {
    if query.is_empty() {
        return Err("Search query cannot be empty".into());
    }

    let search_roots: Vec<PathBuf> = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths.iter().map(PathBuf::from).collect()
    };

    let files = discover_files(&search_roots);
    let mut file_summaries = Vec::new();
    let mut total_replacements = 0;

    for file_path in &files {
        if let Ok((mut doc, _meta)) = load_from_file(file_path) {
            let matches = find_all(doc.rope(), query);
            let count = matches.len();
            if count > 0 {
                total_replacements += count;
                let modified = !dry_run && !check_mode;

                if modified {
                    replace_all(&mut doc, query, replacement)
                        .map_err(|e| format!("Failed to apply replacement in {}: {e:?}", file_path.display()))?;
                    save_to_file(&doc, file_path)
                        .map_err(|e| format!("Failed to save {}: {e}", file_path.display()))?;
                }

                file_summaries.push(FileReplaceSummary {
                    path: file_path.clone(),
                    replacements: count,
                    modified,
                });
            }
        }
    }

    let result = ReplaceResult {
        query: query.to_string(),
        replacement: replacement.to_string(),
        dry_run: dry_run || check_mode,
        total_replacements,
        files: file_summaries,
    };

    if json_mode {
        let envelope = Response::new(result);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope).map_err(|e| e.to_string())?
        );
    } else {
        let prefix = if check_mode {
            "[CHECK]"
        } else if dry_run {
            "[DRY RUN]"
        } else {
            "[MODIFIED]"
        };

        for f in &result.files {
            println!(
                "{} {}: {} replacement{}",
                prefix,
                f.path.display(),
                f.replacements,
                if f.replacements == 1 { "" } else { "s" }
            );
        }

        let mode_desc = if check_mode {
            "Detected"
        } else if dry_run {
            "Would replace"
        } else {
            "Replaced"
        };

        println!(
            "\n{} {} occurrence{} across {} file{}.",
            mode_desc,
            result.total_replacements,
            if result.total_replacements == 1 { "" } else { "s" },
            result.files.len(),
            if result.files.len() == 1 { "" } else { "s" }
        );
    }

    if check_mode {
        // In check mode: exit 0 if no occurrences found, exit 1 if matches exist
        if total_replacements == 0 {
            Ok(ExitCode::SUCCESS)
        } else {
            Ok(ExitCode::from(1))
        }
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_help();
        return ExitCode::SUCCESS;
    }

    let first_arg = &args[1];

    if first_arg == "-h" || first_arg == "--help" || first_arg == "help" {
        print_help();
        return ExitCode::SUCCESS;
    }

    if first_arg == "-V" || first_arg == "--version" || first_arg == "version" {
        print_version();
        return ExitCode::SUCCESS;
    }

    match first_arg.as_str() {
        "inspect" => {
            let mut json_mode = false;
            let mut file_path = None;

            for arg in &args[2..] {
                if arg == "--json" {
                    json_mode = true;
                } else if !arg.starts_with('-') && file_path.is_none() {
                    file_path = Some(arg.as_str());
                }
            }

            match file_path {
                Some(path) => match run_inspect(path, json_mode) {
                    Ok(code) => code,
                    Err(e) => {
                        eprintln!("Error: {e}");
                        ExitCode::from(1)
                    }
                },
                None => {
                    eprintln!("Error: Missing required file path for 'inspect'.");
                    eprintln!("Usage: arcade-headless inspect <FILE> [--json]");
                    ExitCode::from(1)
                }
            }
        }

        "search" => {
            let mut json_mode = false;
            let mut query = None;
            let mut paths = Vec::new();

            for arg in &args[2..] {
                if arg == "--json" {
                    json_mode = true;
                } else if !arg.starts_with('-') && query.is_none() {
                    query = Some(arg.as_str());
                } else if !arg.starts_with('-') {
                    paths.push(arg.clone());
                }
            }

            match query {
                Some(q) => match run_search(q, &paths, json_mode) {
                    Ok(code) => code,
                    Err(e) => {
                        eprintln!("Error: {e}");
                        ExitCode::from(1)
                    }
                },
                None => {
                    eprintln!("Error: Missing required query string for 'search'.");
                    eprintln!("Usage: arcade-headless search <QUERY> [PATHS...] [--json]");
                    ExitCode::from(1)
                }
            }
        }

        "replace" => {
            let mut json_mode = false;
            let mut dry_run = false;
            let mut check_mode = false;
            let mut query = None;
            let mut replacement = None;
            let mut paths = Vec::new();

            for arg in &args[2..] {
                if arg == "--json" {
                    json_mode = true;
                } else if arg == "--dry-run" {
                    dry_run = true;
                } else if arg == "--check" {
                    check_mode = true;
                } else if !arg.starts_with('-') && query.is_none() {
                    query = Some(arg.as_str());
                } else if !arg.starts_with('-') && replacement.is_none() {
                    replacement = Some(arg.as_str());
                } else if !arg.starts_with('-') {
                    paths.push(arg.clone());
                }
            }

            match (query, replacement) {
                (Some(q), Some(r)) => {
                    match run_replace(q, r, &paths, dry_run, check_mode, json_mode) {
                        Ok(code) => code,
                        Err(e) => {
                            eprintln!("Error: {e}");
                            ExitCode::from(1)
                        }
                    }
                }
                _ => {
                    eprintln!("Error: 'replace' requires both <QUERY> and <REPLACEMENT>.");
                    eprintln!("Usage: arcade-headless replace <QUERY> <REPLACEMENT> [PATHS...] [--dry-run] [--check] [--json]");
                    ExitCode::from(1)
                }
            }
        }

        unknown => {
            eprintln!("Unknown subcommand: '{unknown}'");
            eprintln!("Run 'arcade-headless --help' for available subcommands.");
            ExitCode::from(1)
        }
    }
}

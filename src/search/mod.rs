use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use serde_json::Value;

use crate::settings::{CaseSensitivity, SearchCliSettings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchScope {
    Contents,
    Names,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchOptions {
    pub query: String,
    pub case: CaseSensitivity,
    pub regex: bool,
    pub whole_word: bool,
    pub include_hidden: bool,
    pub respect_gitignore: bool,
}

#[derive(Clone, Debug)]
pub struct ContentMatch {
    pub path: PathBuf,
    pub line_number: u64,
    pub line_text: String,
    pub match_ranges: Vec<std::ops::Range<usize>>,
}

#[derive(Clone, Debug)]
pub struct NameMatch {
    pub path: PathBuf,
    pub match_ranges: Vec<std::ops::Range<usize>>,
}

pub struct SearchOutcome<T> {
    pub items: Vec<T>,
    pub truncated: bool,
}

pub fn run_content_search(
    dir: &Path,
    options: &SearchOptions,
    cli: &SearchCliSettings,
    max_results: usize,
    cancel: &AtomicBool,
) -> Result<SearchOutcome<ContentMatch>, String> {
    if options.query.is_empty() {
        return Ok(SearchOutcome { items: Vec::new(), truncated: false });
    }

    let binary = cli.binary_path.as_deref().unwrap_or("rg");
    let mut args: Vec<String> = vec!["--json".into(), "--line-number".into()];
    args.extend(common_flags(options));
    args.push("--".into());
    args.push(options.query.clone());
    args.push(".".into());
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    let output = crate::process::run_with_timeout(binary, dir, &arg_refs, None, cli.timeout_ms, cancel)
        .ok_or_else(rg_unavailable_message)?;

    match output.status_code {
        Some(0) | Some(1) => Ok(parse_content_matches(dir, &output.stdout, max_results)),
        _ => Err(rg_error_message(&output.stderr)),
    }
}

pub fn run_name_search(
    dir: &Path,
    options: &SearchOptions,
    cli: &SearchCliSettings,
    max_results: usize,
    cancel: &AtomicBool,
) -> Result<SearchOutcome<NameMatch>, String> {
    if options.query.is_empty() {
        return Ok(SearchOutcome { items: Vec::new(), truncated: false });
    }

    let binary = cli.binary_path.as_deref().unwrap_or("rg");

    let mut list_args: Vec<String> = vec!["--files".into()];
    if options.include_hidden {
        list_args.push("--hidden".into());
    }
    if !options.respect_gitignore {
        list_args.push("--no-ignore".into());
    }
    list_args.push(".".into());
    let list_arg_refs: Vec<&str> = list_args.iter().map(String::as_str).collect();

    let listing = crate::process::run_with_timeout(binary, dir, &list_arg_refs, None, cli.timeout_ms, cancel)
        .ok_or_else(rg_unavailable_message)?;
    match listing.status_code {
        Some(0) | Some(1) => {}
        _ => return Err(rg_error_message(&listing.stderr)),
    }

    if options.regex {
        run_name_regex_filter(dir, options, cli, binary, &listing.stdout, max_results, cancel)
    } else {
        Ok(filter_names_literal(dir, options, &listing.stdout, max_results))
    }
}

fn run_name_regex_filter(
    dir: &Path,
    options: &SearchOptions,
    cli: &SearchCliSettings,
    binary: &str,
    candidates: &[u8],
    max_results: usize,
    cancel: &AtomicBool,
) -> Result<SearchOutcome<NameMatch>, String> {
    let mut args: Vec<String> = vec!["--json".into(), case_flag(options.case).to_string()];
    if options.whole_word {
        args.push("--word-regexp".into());
    }
    args.push("--".into());
    args.push(options.query.clone());
    args.push("-".into());
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    let output = crate::process::run_with_timeout(binary, dir, &arg_refs, Some(candidates), cli.timeout_ms, cancel)
        .ok_or_else(rg_unavailable_message)?;
    match output.status_code {
        Some(0) | Some(1) => {}
        _ => return Err(rg_error_message(&output.stderr)),
    }

    let mut items = Vec::new();
    let mut truncated = false;
    for value in iter_matches(&output.stdout) {
        let Some(text) = value
            .pointer("/data/lines/text")
            .and_then(Value::as_str)
        else {
            continue;
        };
        if items.len() >= max_results {
            truncated = true;
            break;
        }
        let text = text.trim_end_matches(['\n', '\r']);
        items.push(NameMatch {
            path: dir.join(text),
            match_ranges: extract_match_ranges(&value),
        });
    }
    Ok(SearchOutcome { items, truncated })
}

fn filter_names_literal(
    dir: &Path,
    options: &SearchOptions,
    candidates: &[u8],
    max_results: usize,
) -> SearchOutcome<NameMatch> {
    let insensitive = is_case_insensitive(options.case, &options.query);
    let needle = if insensitive { options.query.to_lowercase() } else { options.query.clone() };

    let mut items = Vec::new();
    let mut truncated = false;
    for line in candidates.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let text = String::from_utf8_lossy(line);
        let haystack = if insensitive { text.to_lowercase() } else { text.to_string() };
        let Some(start) = haystack.find(&needle) else {
            continue;
        };
        if items.len() >= max_results {
            truncated = true;
            break;
        }
        items.push(NameMatch {
            path: dir.join(text.as_ref()),
            match_ranges: vec![start..start + needle.len()],
        });
    }
    SearchOutcome { items, truncated }
}

fn parse_content_matches(dir: &Path, stdout: &[u8], max_results: usize) -> SearchOutcome<ContentMatch> {
    let mut items = Vec::new();
    let mut truncated = false;

    for value in iter_matches(stdout) {
        let Some(path) = value.pointer("/data/path/text").and_then(Value::as_str) else {
            continue;
        };
        let Some(line_number) = value.pointer("/data/line_number").and_then(Value::as_u64) else {
            continue;
        };
        let Some(line_text) = value.pointer("/data/lines/text").and_then(Value::as_str) else {
            continue;
        };

        if items.len() >= max_results {
            truncated = true;
            break;
        }

        items.push(ContentMatch {
            path: dir.join(path),
            line_number,
            line_text: line_text.trim_end_matches(['\n', '\r']).to_string(),
            match_ranges: extract_match_ranges(&value),
        });
    }

    SearchOutcome { items, truncated }
}

fn extract_match_ranges(value: &Value) -> Vec<std::ops::Range<usize>> {
    value
        .pointer("/data/submatches")
        .and_then(Value::as_array)
        .map(|submatches| {
            submatches
                .iter()
                .filter_map(|submatch| {
                    let start = submatch.get("start")?.as_u64()? as usize;
                    let end = submatch.get("end")?.as_u64()? as usize;
                    Some(start..end)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn iter_matches(stdout: &[u8]) -> impl Iterator<Item = Value> + '_ {
    stdout
        .split(|&b| b == b'\n')
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .filter(|value| value.get("type").and_then(Value::as_str) == Some("match"))
}

fn common_flags(options: &SearchOptions) -> Vec<String> {
    let mut flags = vec![case_flag(options.case).to_string()];
    if !options.regex {
        flags.push("--fixed-strings".into());
    }
    if options.whole_word {
        flags.push("--word-regexp".into());
    }
    if options.include_hidden {
        flags.push("--hidden".into());
    }
    if !options.respect_gitignore {
        flags.push("--no-ignore".into());
    }
    flags
}

fn case_flag(case: CaseSensitivity) -> &'static str {
    match case {
        CaseSensitivity::Sensitive => "-s",
        CaseSensitivity::Insensitive => "-i",
        CaseSensitivity::Smart => "-S",
    }
}

fn is_case_insensitive(case: CaseSensitivity, query: &str) -> bool {
    match case {
        CaseSensitivity::Sensitive => false,
        CaseSensitivity::Insensitive => true,
        CaseSensitivity::Smart => !query.chars().any(|c| c.is_uppercase()),
    }
}

fn rg_unavailable_message() -> String {
    "Couldn't start ripgrep (rg). Make sure it's installed and on your PATH.".to_string()
}

fn rg_error_message(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr).trim().to_string();
    if text.is_empty() {
        "ripgrep exited with an error".to_string()
    } else {
        text
    }
}

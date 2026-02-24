// voikko-cli: shared utilities for CLI tools.

use std::path::PathBuf;
use std::process;

use voikko_fi::handle::{VoikkoError, VoikkoHandle};

/// Default dictionary directory name within VFST dictionary packages.
const DICT_SUBDIR: &str = "5/mor-standard";

/// Morphology transducer file name.
const MOR_VFST: &str = "mor.vfst";

/// Autocorrect transducer file name.
const AUTOCORR_VFST: &str = "autocorr.vfst";

/// Search for dictionary files and create a VoikkoHandle.
///
/// Search order:
/// 1. `dict_path` argument (if provided)
/// 2. `VOIKKO_DICT_PATH` environment variable
/// 3. `~/.voikko/5/mor-standard`
/// 4. Current working directory (looks for `mor.vfst` directly)
pub fn load_handle(dict_path: Option<&str>) -> Result<VoikkoHandle, String> {
    let search_paths = build_search_paths(dict_path);

    for dir in &search_paths {
        let mor_path = dir.join(MOR_VFST);
        if mor_path.is_file() {
            let mor_data = std::fs::read(&mor_path)
                .map_err(|e| format!("failed to read {}: {}", mor_path.display(), e))?;

            let autocorr_path = dir.join(AUTOCORR_VFST);
            let autocorr_data =
                if autocorr_path.is_file() {
                    Some(std::fs::read(&autocorr_path).map_err(|e| {
                        format!("failed to read {}: {}", autocorr_path.display(), e)
                    })?)
                } else {
                    None
                };

            return VoikkoHandle::from_bytes(&mor_data, autocorr_data.as_deref(), "fi")
                .map_err(|e: VoikkoError| format!("failed to create VoikkoHandle: {e}"));
        }
    }

    Err(format!(
        "could not find {} in any of the search paths:\n{}",
        MOR_VFST,
        search_paths
            .iter()
            .map(|p| format!("  - {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}

/// Build the list of directories to search for dictionary files.
fn build_search_paths(dict_path: Option<&str>) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. Explicit path from argument
    if let Some(p) = dict_path {
        paths.push(PathBuf::from(p));
    }

    // 2. VOIKKO_DICT_PATH environment variable
    if let Ok(env_path) = std::env::var("VOIKKO_DICT_PATH") {
        paths.push(PathBuf::from(&env_path));
        // Also check the standard subdirectory within the env path
        paths.push(PathBuf::from(&env_path).join(DICT_SUBDIR));
    }

    // 3. Home directory paths
    if let Some(home) = home_dir() {
        paths.push(home.join(".voikko").join(DICT_SUBDIR));
        // macOS Library/Spelling
        #[cfg(target_os = "macos")]
        paths.push(
            home.join("Library")
                .join("Spelling")
                .join("voikko")
                .join(DICT_SUBDIR),
        );
    }

    // 4. System paths
    paths.push(PathBuf::from("/etc/voikko").join(DICT_SUBDIR));
    paths.push(PathBuf::from("/usr/lib/voikko").join(DICT_SUBDIR));
    paths.push(PathBuf::from("/usr/share/voikko").join(DICT_SUBDIR));

    // 5. Current directory (fallback for local development)
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd);
    }

    paths
}

/// Get the user's home directory.
fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

/// Parse a `--dict-path=PATH` or `-d PATH` argument from command line args.
///
/// Returns `(dict_path, remaining_args)`.
pub fn parse_dict_path(args: &[String]) -> (Option<String>, Vec<String>) {
    let mut dict_path = None;
    let mut remaining = Vec::new();
    let mut skip_next = false;

    for (i, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if let Some(val) = arg.strip_prefix("--dict-path=") {
            dict_path = Some(val.to_string());
        } else if arg == "--dict-path" || arg == "-d" {
            if i + 1 < args.len() {
                dict_path = Some(args[i + 1].clone());
                skip_next = true;
            } else {
                eprintln!("error: {} requires a value", arg);
                process::exit(1);
            }
        } else {
            remaining.push(arg.clone());
        }
    }

    (dict_path, remaining)
}

/// Print an error message and exit with code 1.
pub fn fatal(msg: &str) -> ! {
    eprintln!("error: {msg}");
    process::exit(1);
}

/// Check if `--help` or `-h` is in the args.
pub fn wants_help(args: &[String]) -> bool {
    args.iter().any(|a| a == "--help" || a == "-h")
}

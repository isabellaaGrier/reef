//! State file management for `reef persist state` mode.
//!
//! Persists exported variables across bash invocations by writing them to a
//! file as `export KEY='value'` statements that bash can source on the next run.

use std::fs;
use std::path::Path;

use crate::env_diff;

/// Write exported variables to a state file as bash `export` statements.
///
/// Parses null-separated env output (from `env -0`), filters out bash
/// internals, and writes `export KEY='value'` lines that bash can source.
///
/// # Errors
///
/// Returns [`std::io::Error`] if writing to `path` fails (e.g., permission
/// denied, directory does not exist, disk full).
pub fn save_state(path: &Path, env_data: &str) -> std::io::Result<()> {
    let mut output = String::with_capacity(env_data.len());

    for entry in env_data.split('\0') {
        let entry = entry.trim_start_matches('\n');
        if entry.is_empty() {
            continue;
        }
        if let Some(eq_pos) = entry.find('=') {
            let key = &entry[..eq_pos];
            let value = &entry[eq_pos + 1..];

            if key.is_empty()
                || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            {
                continue;
            }
            if env_diff::should_skip_var(key) {
                continue;
            }

            output.push_str("export ");
            output.push_str(key);
            output.push_str("='");
            for &b in value.as_bytes() {
                if b == b'\'' {
                    output.push_str("'\\''");
                } else {
                    output.push(b as char);
                }
            }
            output.push_str("'\n");
        }
    }

    fs::write(path, output)
}

/// Build a bash prefix that sources the state file if it exists.
#[must_use]
pub fn state_prefix(path: &Path) -> String {
    let p = path.display();
    format!("[ -f '{p}' ] && source '{p}'\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn save_and_read_state() {
        let dir = std::env::temp_dir();
        let path = dir.join("reef-test-state");

        let env_data = "FOO=bar\0MY_VAR=hello world\0";
        save_state(&path, env_data).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("export FOO='bar'"));
        assert!(content.contains("export MY_VAR='hello world'"));

        fs::remove_file(&path).ok();
    }

    #[test]
    fn save_state_escapes_quotes() {
        let dir = std::env::temp_dir();
        let path = dir.join("reef-test-state-quotes");

        let env_data = "QUOTED=it's a test\0";
        save_state(&path, env_data).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("export QUOTED='it'\\''s a test'"));

        fs::remove_file(&path).ok();
    }

    #[test]
    fn save_state_skips_bash_internals() {
        let dir = std::env::temp_dir();
        let path = dir.join("reef-test-state-skip");

        let env_data = "BASH_VERSION=5.2\0REAL_VAR=keep\0SHLVL=1\0";
        save_state(&path, env_data).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(!content.contains("BASH_VERSION"));
        assert!(!content.contains("SHLVL"));
        assert!(content.contains("export REAL_VAR='keep'"));

        fs::remove_file(&path).ok();
    }

    #[test]
    fn state_prefix_format() {
        let path = Path::new("/tmp/reef-state-12345");
        let prefix = state_prefix(path);
        assert_eq!(
            prefix,
            "[ -f '/tmp/reef-state-12345' ] && source '/tmp/reef-state-12345'\n"
        );
    }
}

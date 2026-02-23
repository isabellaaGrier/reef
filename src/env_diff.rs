//! Environment snapshot diffing for bash-to-fish variable sync.
//!
//! Captures environment state before and after a bash command, then generates
//! fish shell commands (`set -gx`, `set -e`, `cd`) to apply the differences.

use std::borrow::Cow;
use std::collections::HashMap;

/// Variables that are internal to bash and should not be synced to fish.
/// Sorted by ASCII byte order for O(log n) binary search.
const SKIP_VARS: &[&str] = &[
    "BASH",
    "BASHOPTS",
    "BASHPID",
    "BASH_ALIASES",
    "BASH_ARGC",
    "BASH_ARGV",
    "BASH_CMDS",
    "BASH_COMMAND",
    "BASH_EXECUTION_STRING",
    "BASH_LINENO",
    "BASH_LOADABLES_PATH",
    "BASH_REMATCH",
    "BASH_SOURCE",
    "BASH_SUBSHELL",
    "BASH_VERSINFO",
    "BASH_VERSION",
    "COLUMNS",
    "COMP_WORDBREAKS",
    "DIRSTACK",
    "EUID",
    "FUNCNAME",
    "GROUPS",
    "HISTCMD",
    "HISTFILE",
    "HOSTNAME",
    "HOSTTYPE",
    "IFS",
    "LINES",
    "MACHTYPE",
    "MAILCHECK",
    "OLDPWD",
    "OPTERR",
    "OPTIND",
    "OSTYPE",
    "PIPESTATUS",
    "PPID",
    "PS1",
    "PS2",
    "PS4",
    "PWD",
    "RANDOM",
    "SECONDS",
    "SHELL",
    "SHELLOPTS",
    "SHLVL",
    "UID",
    "_",
];

/// A snapshot of the shell environment at a point in time.
#[derive(Debug, Clone)]
pub struct EnvSnapshot {
    vars: HashMap<String, String>,
    cwd: String,
}

impl EnvSnapshot {
    /// Create a snapshot from the given variables and working directory.
    #[must_use]
    pub fn new(vars: HashMap<String, String>, cwd: String) -> Self {
        EnvSnapshot { vars, cwd }
    }

    /// Capture the current process environment, skipping bash-internal vars.
    #[must_use]
    pub fn capture_current() -> Self {
        let vars: HashMap<String, String> = std::env::vars()
            .filter(|(k, _)| !should_skip_var(k))
            .collect();
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        EnvSnapshot { vars, cwd }
    }

    /// The environment variables in this snapshot.
    ///
    /// # Examples
    ///
    /// ```
    /// use reef::env_diff::EnvSnapshot;
    /// let snap = EnvSnapshot::capture_current();
    /// assert!(snap.vars().contains_key("HOME"));
    /// ```
    #[must_use]
    #[allow(dead_code)] // public API for downstream consumers
    pub fn vars(&self) -> &HashMap<String, String> {
        &self.vars
    }

    /// The working directory in this snapshot.
    ///
    /// # Examples
    ///
    /// ```
    /// use reef::env_diff::EnvSnapshot;
    /// let snap = EnvSnapshot::capture_current();
    /// assert!(!snap.cwd().is_empty());
    /// ```
    #[must_use]
    #[allow(dead_code)] // public API for downstream consumers
    pub fn cwd(&self) -> &str {
        &self.cwd
    }

    /// Diff two snapshots, writing fish commands into a single buffer.
    ///
    /// Appends newline-separated commands like `set -gx VAR value`,
    /// `set -e VAR`, or `cd /new/path` to `out`. Uses a single allocation
    /// instead of one `String` per command.
    pub fn diff_into(&self, after: &EnvSnapshot, out: &mut String) {
        // New or changed variables
        for (key, new_val) in &after.vars {
            if should_skip_var(key) {
                continue;
            }

            let changed = match self.vars.get(key) {
                Some(old_val) => old_val != new_val,
                None => true,
            };

            if changed {
                out.push_str("set -gx ");
                out.push_str(key);
                out.push(' ');
                // PATH-like variables: split on : for fish list semantics
                if key.ends_with("PATH") && new_val.contains(':') {
                    for (i, part) in new_val.split(':').enumerate() {
                        if i > 0 {
                            out.push(' ');
                        }
                        out.push_str(part);
                    }
                } else {
                    out.push_str(&shell_escape(new_val));
                }
                out.push('\n');
            }
        }

        // Removed variables
        for key in self.vars.keys() {
            if should_skip_var(key) {
                continue;
            }
            if !after.vars.contains_key(key) {
                out.push_str("set -e ");
                out.push_str(key);
                out.push('\n');
            }
        }

        // Changed directory
        if !after.cwd.is_empty() && self.cwd != after.cwd {
            out.push_str("cd ");
            out.push_str(&shell_escape(&after.cwd));
            out.push('\n');
        }
    }

    /// Diff two snapshots, returning fish commands as a newline-separated string.
    ///
    /// Convenience wrapper around [`diff_into`](Self::diff_into) that allocates
    /// and returns a new `String`.
    #[must_use]
    #[allow(dead_code)]
    pub fn diff(&self, after: &EnvSnapshot) -> String {
        let mut out = String::new();
        self.diff_into(after, &mut out);
        out
    }
}

/// Parse null-separated environment output (from `env -0`).
#[must_use]
pub fn parse_null_separated_env(data: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();

    // env -0 outputs VAR=value\0VAR=value\0...
    for entry in data.split('\0') {
        let entry = entry.trim_start_matches('\n');
        if entry.is_empty() {
            continue;
        }
        if let Some(eq_pos) = entry.find('=') {
            let key = &entry[..eq_pos];
            let value = &entry[eq_pos + 1..];
            // Skip entries that don't look like valid variable names
            if !key.is_empty() && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                vars.insert(key.to_string(), value.to_string());
            }
        }
    }

    vars
}

/// Check if a variable should be skipped during env sync.
#[must_use]
pub(crate) fn should_skip_var(name: &str) -> bool {
    SKIP_VARS.binary_search(&name).is_ok()
}

/// Escape a string for safe use in fish shell commands.
/// Returns `Cow::Borrowed` when no escaping is needed (avoids allocation).
fn shell_escape(s: &str) -> Cow<'_, str> {
    // If it's simple (alphanumeric, slashes, dots, hyphens), no quoting needed
    if s.bytes().all(|b| {
        b.is_ascii_alphanumeric()
            || matches!(b, b'/' | b'.' | b'-' | b'_' | b':' | b'~' | b'+' | b',')
    }) {
        return Cow::Borrowed(s);
    }
    // Otherwise, single-quote it (escaping any internal single quotes)
    let mut result = String::with_capacity(s.len() + 2);
    result.push('\'');
    for &b in s.as_bytes() {
        if b == b'\'' {
            result.push_str("'\\''");
        } else {
            result.push(b as char);
        }
    }
    result.push('\'');
    Cow::Owned(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_vars_sorted() {
        for pair in SKIP_VARS.windows(2) {
            assert!(
                pair[0] < pair[1],
                "SKIP_VARS not sorted: {:?} >= {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn parse_null_env() {
        let data = "FOO=bar\0BAZ=qux\0MULTI=hello world\0";
        let vars = parse_null_separated_env(data);
        assert_eq!(vars.get("FOO").unwrap(), "bar");
        assert_eq!(vars.get("BAZ").unwrap(), "qux");
        assert_eq!(vars.get("MULTI").unwrap(), "hello world");
    }

    #[test]
    fn diff_new_var() {
        let before = EnvSnapshot::new(HashMap::new(), "/home".to_string());
        let mut after_vars = HashMap::new();
        after_vars.insert("NEW_VAR".to_string(), "hello".to_string());
        let after = EnvSnapshot::new(after_vars, "/home".to_string());

        let out = before.diff(&after);
        assert!(out.contains("set -gx NEW_VAR"));
    }

    #[test]
    fn diff_removed_var() {
        let mut before_vars = HashMap::new();
        before_vars.insert("OLD_VAR".to_string(), "gone".to_string());
        let before = EnvSnapshot::new(before_vars, "/home".to_string());
        let after = EnvSnapshot::new(HashMap::new(), "/home".to_string());

        let out = before.diff(&after);
        assert!(out.lines().any(|l| l == "set -e OLD_VAR"));
    }

    #[test]
    fn diff_changed_cwd() {
        let before = EnvSnapshot::new(HashMap::new(), "/home".to_string());
        let after = EnvSnapshot::new(HashMap::new(), "/tmp".to_string());

        let out = before.diff(&after);
        assert!(out.contains("cd /tmp"));
    }

    #[test]
    fn diff_path_split() {
        let before = EnvSnapshot::new(HashMap::new(), "/home".to_string());
        let mut after_vars = HashMap::new();
        after_vars.insert("PATH".to_string(), "/usr/bin:/usr/local/bin".to_string());
        let after = EnvSnapshot::new(after_vars, "/home".to_string());

        let out = before.diff(&after);
        let path_line = out.lines().find(|l| l.contains("PATH")).unwrap();
        assert!(path_line.contains("/usr/bin /usr/local/bin"));
    }

    #[test]
    fn skip_bash_internal_vars() {
        let before = EnvSnapshot::new(HashMap::new(), "/home".to_string());
        let mut after_vars = HashMap::new();
        after_vars.insert("BASH_VERSION".to_string(), "5.2.0".to_string());
        after_vars.insert("REAL_VAR".to_string(), "keep".to_string());
        let after = EnvSnapshot::new(after_vars, "/home".to_string());

        let out = before.diff(&after);
        assert!(!out.contains("BASH_VERSION"));
        assert!(out.contains("REAL_VAR"));
    }

    #[test]
    fn shell_escape_simple() {
        assert_eq!(shell_escape("/usr/bin"), "/usr/bin");
        assert_eq!(shell_escape("hello"), "hello");
    }

    #[test]
    fn shell_escape_spaces() {
        assert_eq!(shell_escape("hello world"), "'hello world'");
    }

    #[test]
    fn shell_escape_quotes() {
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn capture_current_env() {
        let snap = EnvSnapshot::capture_current();
        assert!(!snap.vars().is_empty());
        assert!(!snap.cwd().is_empty());
        assert!(snap.vars().contains_key("HOME"));
    }
}

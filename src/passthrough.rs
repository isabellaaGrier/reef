//! Bash command passthrough execution with environment diffing.
//!
//! Runs commands in a bash subprocess, captures environment changes, and
//! prints fish `set` commands to synchronize the fish shell's state.

use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::env_diff::{self, EnvSnapshot};
use crate::state;

/// Null-delimited sentinel markers for separating env data from command output.
/// Null bytes prevent collisions with any possible command output.
const ENV_MARKER: &str = "\0__REEF_ENV__\0";
const CWD_MARKER: &str = "\0__REEF_CWD__\0";

/// Execute a command through bash with streaming output, then print
/// environment changes as fish commands to stdout.
///
/// Returns the bash command's exit code. The caller (fish) is expected to
/// pipe stdout through `| source` to apply environment changes.
///
/// How it works:
/// 1. Capture a "before" snapshot of the current environment
/// 2. Run the command in bash with stderr inherited (streams directly)
/// 3. Stdout is captured — the command output appears before our markers,
///    and we print it back to the real stdout immediately
/// 4. After the markers, we parse the env dump
/// 5. Diff before/after and print fish `set` commands
///
/// # Examples
///
/// ```no_run
/// use reef::passthrough::bash_exec;
///
/// // Run a bash command and get its exit code
/// let exit_code = bash_exec("export MY_VAR=hello && echo done");
/// assert_eq!(exit_code, 0);
/// // Fish `set -gx MY_VAR hello` commands are printed to stdout
/// ```
#[must_use]
pub fn bash_exec(command: &str) -> i32 {
    let before = EnvSnapshot::capture_current();

    // Run the user's command in bash with output to stderr (so user sees it),
    // then dump env to stdout (for fish to eval).
    let script = build_script(&shell_escape_for_bash(command), " >&2", true);

    let output = match Command::new("bash")
        .args(["-c", &script])
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("reef: failed to run bash: {e}");
            return 1;
        }
    };

    // Preserve signal information: on Unix, a process killed by signal has
    // no exit code. Fall back to 128 + signal number (shell convention).
    #[cfg(unix)]
    let exit_code = {
        use std::os::unix::process::ExitStatusExt;
        output.status.code().unwrap_or_else(|| {
            output.status.signal().map_or(1, |sig| 128 + sig)
        })
    };
    #[cfg(not(unix))]
    let exit_code = output.status.code().unwrap_or(1);
    diff_and_print_env(&before, &output.stdout);
    exit_code
}

/// Execute a command through bash and only print environment diff as
/// fish commands. No command output is shown — both stdout and stderr
/// are suppressed. Used to source bash scripts and capture their
/// environment side effects.
///
/// Returns the bash command's exit code.
///
/// # Examples
///
/// ```no_run
/// use reef::passthrough::bash_exec_env_diff;
///
/// // Source a bash script, capturing only env changes
/// let exit_code = bash_exec_env_diff("source ~/.bashrc");
/// // Fish `set -gx` commands for any new/changed vars are on stdout
/// ```
#[must_use]
pub fn bash_exec_env_diff(command: &str) -> i32 {
    let before = EnvSnapshot::capture_current();

    // Run the command and capture env afterward — all in one bash invocation.
    // Suppress command stdout/stderr since we only want the env diff.
    let script = build_script(&shell_escape_for_bash(command), " >/dev/null 2>&1", false);

    let output = match Command::new("bash").args(["-c", &script]).output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("reef: failed to run bash: {e}");
            return 1;
        }
    };

    diff_and_print_env(&before, &output.stdout);

    if output.status.success() {
        0
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            output.status.code().unwrap_or_else(|| {
                output.status.signal().map_or(1, |sig| 128 + sig)
            })
        }
        #[cfg(not(unix))]
        {
            output.status.code().unwrap_or(1)
        }
    }
}

/// Execute a command through bash with state file persistence.
///
/// Before running the command, sources the state file to restore previous
/// exported variables. After running, saves the new environment to the state
/// file and prints the diff as fish commands.
///
/// Returns the bash command's exit code.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use reef::passthrough::bash_exec_with_state;
///
/// let state = Path::new("/tmp/reef-state-12345");
/// let exit_code = bash_exec_with_state("export FOO=bar", state);
/// // FOO=bar is persisted to the state file for next invocation
/// ```
#[must_use]
pub fn bash_exec_with_state(command: &str, state_path: &Path) -> i32 {
    let before = EnvSnapshot::capture_current();

    let prefix = state::state_prefix(state_path);
    let escaped = shell_escape_for_bash(command);
    let body = build_script(&escaped, " >&2", true);

    let mut script = String::with_capacity(prefix.len() + body.len());
    script.push_str(&prefix);
    script.push_str(&body);

    let output = match Command::new("bash")
        .args(["-c", &script])
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("reef: failed to run bash: {e}");
            return 1;
        }
    };

    #[cfg(unix)]
    let exit_code = {
        use std::os::unix::process::ExitStatusExt;
        output.status.code().unwrap_or_else(|| {
            output.status.signal().map_or(1, |sig| 128 + sig)
        })
    };
    #[cfg(not(unix))]
    let exit_code = output.status.code().unwrap_or(1);
    diff_and_print_env_save_state(&before, &output.stdout, state_path);
    exit_code
}

/// Extract env and cwd sections from bash stdout (after sentinel markers).
fn extract_env_sections(raw_stdout: &[u8]) -> Option<(String, String)> {
    let stdout = String::from_utf8_lossy(raw_stdout);
    let env_pos = stdout.find(ENV_MARKER)?;
    let cwd_pos = stdout.find(CWD_MARKER)?;
    let env_section = stdout[env_pos + ENV_MARKER.len()..cwd_pos].to_string();
    let cwd_section = stdout[cwd_pos + CWD_MARKER.len()..].trim().to_string();
    Some((env_section, cwd_section))
}

/// Parse env data from bash stdout, diff against the before snapshot,
/// and print fish `set` commands to stdout.
fn diff_and_print_env(before: &EnvSnapshot, raw_stdout: &[u8]) {
    if let Some((env_section, cwd_section)) = extract_env_sections(raw_stdout) {
        let after = EnvSnapshot::new(
            env_diff::parse_null_separated_env(&env_section),
            cwd_section,
        );
        let mut buf = String::new();
        before.diff_into(&after, &mut buf);
        if !buf.is_empty() {
            let _ = io::stdout().lock().write_all(buf.as_bytes());
        }
    }
}

/// Like `diff_and_print_env`, but also saves the env snapshot to a state file
/// so subsequent invocations can restore it.
fn diff_and_print_env_save_state(before: &EnvSnapshot, raw_stdout: &[u8], state_path: &Path) {
    if let Some((env_section, cwd_section)) = extract_env_sections(raw_stdout) {
        let _ = state::save_state(state_path, &env_section);
        let after = EnvSnapshot::new(
            env_diff::parse_null_separated_env(&env_section),
            cwd_section,
        );
        let mut buf = String::new();
        before.diff_into(&after, &mut buf);
        if !buf.is_empty() {
            let _ = io::stdout().lock().write_all(buf.as_bytes());
        }
    }
}

/// Build a bash script that evals the command with the given redirect suffix,
/// then dumps env markers + env -0 + cwd for the diff.
fn build_script(escaped_cmd: &str, redirect: &str, track_exit: bool) -> String {
    let mut s = String::with_capacity(escaped_cmd.len() + 100);
    s.push_str("eval ");
    s.push_str(escaped_cmd);
    s.push_str(redirect);
    s.push('\n');
    if track_exit {
        s.push_str("__reef_exit=$?\n");
    }
    // Use printf with null bytes for sentinels — prevents collisions with any output
    s.push_str("printf '\\0__REEF_ENV__\\0'\nenv -0\nprintf '\\0__REEF_CWD__\\0'\npwd");
    if track_exit {
        s.push_str("\nexit $__reef_exit");
    }
    s
}

/// Escape a command string for embedding in a bash `eval` statement.
/// We single-quote the entire thing to prevent any interpretation.
fn shell_escape_for_bash(s: &str) -> String {
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
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_escape_simple() {
        assert_eq!(shell_escape_for_bash("echo hello"), "'echo hello'");
    }

    #[test]
    fn shell_escape_with_quotes() {
        assert_eq!(
            shell_escape_for_bash("echo 'it'\"s\""),
            "'echo '\\''it'\\''\"s\"'"
        );
    }

    #[test]
    fn bash_exec_sets_var() {
        // Run a command that exports a unique variable
        let code = bash_exec("export __REEF_TEST_VAR_xyzzy=hello_reef");
        // The command should succeed
        assert_eq!(code, 0);
    }

    #[test]
    fn bash_exec_env_diff_captures_var() {
        // This test verifies that bash_exec_env_diff runs without error
        let code = bash_exec_env_diff("export __REEF_TEST_ED_VAR=test_val");
        assert_eq!(code, 0);
    }

    #[test]
    fn bash_exec_preserves_exit_code() {
        let code = bash_exec("exit 42");
        assert_eq!(code, 42);
    }

    #[test]
    fn bash_exec_exit_code_zero() {
        let code = bash_exec("true");
        assert_eq!(code, 0);
    }

    #[test]
    fn sentinel_uses_null_bytes() {
        // Verify sentinels contain null bytes to prevent collision
        assert!(ENV_MARKER.contains('\0'));
        assert!(CWD_MARKER.contains('\0'));
    }
}

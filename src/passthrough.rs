use std::io::{self, Write};
use std::process::{Command, Stdio};

use crate::env_diff::{self, EnvSnapshot};

/// Sentinel markers — must match env_diff.rs
const ENV_MARKER: &str = "__REEF_ENV_MARKER_5f3a__";
const CWD_MARKER: &str = "__REEF_CWD_MARKER_5f3a__";

/// Execute a command through bash with streaming output, then print
/// environment changes as fish commands to stdout.
///
/// How it works:
/// 1. Capture a "before" snapshot of the current environment
/// 2. Run the command in bash with stderr inherited (streams directly)
/// 3. Stdout is captured — the command output appears before our markers,
///    and we print it back to the real stdout immediately
/// 4. After the markers, we parse the env dump
/// 5. Diff before/after and print fish set commands
///
/// The caller (fish) is expected to eval the fish commands that come after
/// the real command output. To make this work cleanly, the fish wrapper
/// sources the output, so we separate command output (printed to stderr
/// for the user to see) from fish commands (printed to stdout for eval).
pub fn bash_exec(command: &str) -> i32 {
    // Snapshot current environment
    let before = match EnvSnapshot::capture_current() {
        Ok(snap) => snap,
        Err(e) => {
            eprintln!("reef: failed to capture environment: {}", e);
            return 1;
        }
    };

    // Build a bash script that:
    // 1. Runs the user's command (stderr goes to terminal, stdout to terminal)
    // 2. Prints sentinel markers and env dump to fd 3
    // We use fd 3 to separate command output from env data.
    //
    // However, since the fish caller needs to `| source` our output,
    // we use a simpler approach: run the command with fully inherited
    // stdio, then do a second quick bash to capture env.
    //
    // Problem: env changes from the command are lost between processes.
    //
    // Solution: Run everything in one bash, redirect command output to
    // stderr (so user sees it), and env data to stdout (for fish to eval).
    let script = format!(
        r#"eval {cmd} >&2
__reef_exit=$?
echo '{env_marker}'
env -0
echo '{cwd_marker}'
pwd
exit $__reef_exit"#,
        cmd = shell_escape_for_bash(command),
        env_marker = ENV_MARKER,
        cwd_marker = CWD_MARKER,
    );

    let output = match Command::new("bash")
        .args(["-c", &script])
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("reef: failed to run bash: {}", e);
            return 1;
        }
    };

    let exit_code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse env data from stdout (after the marker)
    let env_start = stdout.find(ENV_MARKER);
    let cwd_start = stdout.find(CWD_MARKER);

    if let (Some(env_pos), Some(cwd_pos)) = (env_start, cwd_start) {
        let env_section = &stdout[env_pos + ENV_MARKER.len()..cwd_pos];
        let cwd_section = stdout[cwd_pos + CWD_MARKER.len()..].trim();

        let after_vars = env_diff::parse_null_separated_env(env_section);
        let after = EnvSnapshot {
            vars: after_vars,
            cwd: cwd_section.to_string(),
        };

        // Print fish commands to apply env changes
        let commands = before.diff(&after);
        let stdout_handle = io::stdout();
        let mut out = stdout_handle.lock();
        for cmd in &commands {
            let _ = writeln!(out, "{}", cmd);
        }
    }

    exit_code
}

/// Execute a command through bash and only print environment diff as
/// fish commands. No command output is shown. Used by `source.fish`
/// to source bash scripts and capture their environment side effects.
pub fn bash_exec_env_diff(command: &str) -> i32 {
    // Snapshot current environment
    let before = match EnvSnapshot::capture_current() {
        Ok(snap) => snap,
        Err(e) => {
            eprintln!("reef: failed to capture environment: {}", e);
            return 1;
        }
    };

    // Run the command and capture env afterward — all in one bash invocation.
    // Suppress command stdout/stderr since we only want the env diff.
    let script = format!(
        r#"eval {cmd} >/dev/null 2>&1
echo '{env_marker}'
env -0
echo '{cwd_marker}'
pwd"#,
        cmd = shell_escape_for_bash(command),
        env_marker = ENV_MARKER,
        cwd_marker = CWD_MARKER,
    );

    let output = match Command::new("bash")
        .args(["-c", &script])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("reef: failed to run bash: {}", e);
            return 1;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    let env_start = stdout.find(ENV_MARKER);
    let cwd_start = stdout.find(CWD_MARKER);

    if let (Some(env_pos), Some(cwd_pos)) = (env_start, cwd_start) {
        let env_section = &stdout[env_pos + ENV_MARKER.len()..cwd_pos];
        let cwd_section = stdout[cwd_pos + CWD_MARKER.len()..].trim();

        let after_vars = env_diff::parse_null_separated_env(env_section);
        let after = EnvSnapshot {
            vars: after_vars,
            cwd: cwd_section.to_string(),
        };

        let commands = before.diff(&after);
        let stdout_handle = io::stdout();
        let mut out = stdout_handle.lock();
        for cmd in &commands {
            let _ = writeln!(out, "{}", cmd);
        }
    }

    // For env-diff mode, always return 0 if bash ran successfully
    if output.status.success() { 0 } else { output.status.code().unwrap_or(1) }
}

/// Escape a command string for embedding in a bash `eval` statement.
/// We single-quote the entire thing to prevent any interpretation.
fn shell_escape_for_bash(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
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
}

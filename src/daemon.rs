//! Persistent bash coprocess daemon for `reef persist full`.
//!
//! Architecture:
//!   - `start()` spawns a detached daemon process (`reef daemon _serve`).
//!   - `exec()` connects to the socket, sends a command, and receives
//!     the output + env diff + exit code.
//!   - `stop()` sends a shutdown signal via the socket.
//!   - `status()` checks if the daemon is alive by pinging the socket.
//!
//! The daemon runs single-threaded — one command at a time, matching
//! interactive shell semantics. Zero external dependencies.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::{Command, Stdio};
use std::{fs, process};

use crate::env_diff::{self, EnvSnapshot};

/// Null-delimited sentinel markers used in the bash protocol.
/// Null bytes avoid collisions with any command output.
const ENV_SENTINEL: &str = "\0__REEF_DAEMON_ENV__\0";
const CWD_SENTINEL: &str = "\0__REEF_DAEMON_CWD__\0";
const EXIT_SENTINEL: &str = "\0__REEF_DAEMON_EXIT__\0";
const DONE_SENTINEL: &str = "\0__REEF_DAEMON_DONE__\0";

/// Magic command sent by `stop()` to shut down the daemon.
const SHUTDOWN_CMD: &str = "__REEF_SHUTDOWN__";

/// Magic command sent by `status()` to check if the daemon is alive.
const PING_CMD: &str = "__REEF_PING__";
const PONG_RESPONSE: &[u8] = b"__REEF_PONG__\n";

// -----------------------------------------------------------------------
// Client API (called by `reef daemon exec/stop/status`)
// -----------------------------------------------------------------------

/// Send a command to the daemon and print results.
/// Returns the command's exit code.
#[must_use]
pub fn exec(socket_path: &str, command: &str) -> i32 {
    let mut stream = match UnixStream::connect(socket_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("reef daemon: failed to connect: {e}");
            eprintln!("reef daemon: is the daemon running? try: reef persist full");
            return 1;
        }
    };

    let before = EnvSnapshot::capture_current();

    // Send command length (4 bytes LE) + command bytes
    let cmd_bytes = command.as_bytes();
    // Shell commands are always far below u32::MAX; truncation cannot occur
    // in practice, and the server rejects oversized payloads anyway.
    #[allow(clippy::cast_possible_truncation)]
    let len = cmd_bytes.len() as u32;
    if stream.write_all(&len.to_le_bytes()).is_err()
        || stream.write_all(cmd_bytes).is_err()
        || stream.flush().is_err()
    {
        eprintln!("reef daemon: failed to send command");
        return 1;
    }

    // Read response until we see DONE_SENTINEL
    let mut response = Vec::with_capacity(4096);
    let mut buf = [0u8; 4096];
    loop {
        let n = match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => {
                eprintln!("reef daemon: read error: {e}");
                return 1;
            }
        };
        response.extend_from_slice(&buf[..n]);
        if contains_sentinel(&response, DONE_SENTINEL) {
            break;
        }
    }

    parse_and_print_response(&before, &response)
}

/// Tell the daemon to shut down.
pub fn stop(socket_path: &str) {
    if let Ok(mut stream) = UnixStream::connect(socket_path) {
        let cmd_bytes = SHUTDOWN_CMD.as_bytes();
        // Constant string — always fits in u32.
        #[allow(clippy::cast_possible_truncation)]
        let len = cmd_bytes.len() as u32;
        let _ = stream.write_all(&len.to_le_bytes());
        let _ = stream.write_all(cmd_bytes);
        let _ = stream.flush();
    }
    // Clean up socket file
    let _ = fs::remove_file(socket_path);
}

/// Check if the daemon is running and responsive.
#[must_use]
pub fn status(socket_path: &str) -> bool {
    let Ok(mut stream) = UnixStream::connect(socket_path) else {
        return false;
    };

    let cmd_bytes = PING_CMD.as_bytes();
    // Constant string — always fits in u32.
    #[allow(clippy::cast_possible_truncation)]
    let len = cmd_bytes.len() as u32;
    if stream.write_all(&len.to_le_bytes()).is_err()
        || stream.write_all(cmd_bytes).is_err()
        || stream.flush().is_err()
    {
        return false;
    }

    let mut buf = [0u8; 64];
    match stream.read(&mut buf) {
        Ok(n) => &buf[..n] == PONG_RESPONSE,
        Err(_) => false,
    }
}

/// Parse the daemon response: extract user output, env diff, and exit code.
fn parse_and_print_response(before: &EnvSnapshot, response: &[u8]) -> i32 {
    let data = String::from_utf8_lossy(response);

    // Response format:
    //   <user_output>ENV_SENTINEL<env_data>CWD_SENTINEL<cwd>EXIT_SENTINEL<code>DONE_SENTINEL

    let Some(env_pos) = data.find(ENV_SENTINEL) else {
        // No sentinels — dump everything as output
        let _ = io::stderr().write_all(response);
        return 1;
    };

    let after_env = &data[env_pos + ENV_SENTINEL.len()..];

    let Some(cwd_pos) = after_env.find(CWD_SENTINEL) else {
        return 1;
    };
    let env_section = &after_env[..cwd_pos];

    let after_cwd = &after_env[cwd_pos + CWD_SENTINEL.len()..];
    let Some(exit_pos) = after_cwd.find(EXIT_SENTINEL) else {
        return 1;
    };
    let cwd_section = after_cwd[..exit_pos].trim();

    let after_exit = &after_cwd[exit_pos + EXIT_SENTINEL.len()..];
    let done_pos = after_exit.find(DONE_SENTINEL).unwrap_or(after_exit.len());
    let exit_code: i32 = after_exit[..done_pos].trim().parse().unwrap_or(1);

    // Exit 127 = command not found in bash. Suppress the error message
    // so the fish wrapper can fall back to trying it as a fish command.
    if exit_code == 127 {
        return 127;
    }

    // Print user output to stderr (so user sees it)
    let user_output = &response[..env_pos];
    if !user_output.is_empty() {
        let _ = io::stderr().write_all(user_output);
    }

    // Build env snapshot and diff
    let after = EnvSnapshot::new(
        env_diff::parse_null_separated_env(env_section),
        cwd_section.to_string(),
    );

    let mut buf = String::new();
    before.diff_into(&after, &mut buf);
    if !buf.is_empty() {
        let _ = io::stdout().lock().write_all(buf.as_bytes());
    }

    exit_code
}

// -----------------------------------------------------------------------
// Server (daemon process)
// -----------------------------------------------------------------------

/// Start the daemon: spawn a detached `reef daemon _serve` process.
pub fn start(socket_path: &str) {
    // Remove stale socket if it exists
    let _ = fs::remove_file(socket_path);

    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("reef daemon: failed to find executable: {e}");
            process::exit(1);
        }
    };

    match Command::new(exe)
        .args(["daemon", "_serve", "--socket", socket_path])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(_) => {}
        Err(e) => {
            eprintln!("reef daemon: failed to spawn: {e}");
            process::exit(1);
        }
    }

    // Wait for socket to appear (up to 500ms)
    for _ in 0..50 {
        if std::path::Path::new(socket_path).exists() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    eprintln!("reef daemon: timed out waiting for socket");
}

/// Main daemon loop: spawn bash, accept connections, proxy commands.
/// Called by `reef daemon _serve` (internal, not user-facing).
///
/// # Panics
///
/// Panics if `bash.stdin` or `bash.stdout` cannot be taken after spawning
/// with `Stdio::piped()`. This is infallible in practice — `take()` only
/// returns `None` if called twice, and we call it exactly once.
pub fn serve(socket_path: &str) {
    let listener = match UnixListener::bind(socket_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("reef daemon: failed to bind socket: {e}");
            return;
        }
    };

    // Spawn persistent bash process
    let mut bash = match Command::new("bash")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("reef daemon: failed to spawn bash: {e}");
            let _ = fs::remove_file(socket_path);
            return;
        }
    };

    let bash_stdin = bash.stdin.take().expect("stdin was set to piped");
    let bash_stdout = bash.stdout.take().expect("stdout was set to piped");

    let mut writer = io::BufWriter::new(bash_stdin);
    let mut reader = BufReader::new(bash_stdout);

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            continue;
        };

        // Read command: 4-byte LE length + command bytes
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).is_err() {
            continue;
        }
        let cmd_len = u32::from_le_bytes(len_buf) as usize;

        // Guard against absurd lengths — reject anything over 16 MiB to
        // prevent a malicious or buggy client from exhausting memory.
        const MAX_CMD_LEN: usize = 16 * 1024 * 1024;
        if cmd_len > MAX_CMD_LEN {
            continue;
        }

        let mut cmd_buf = vec![0u8; cmd_len];
        if stream.read_exact(&mut cmd_buf).is_err() {
            continue;
        }
        let command = String::from_utf8_lossy(&cmd_buf);

        // Handle special commands
        if *command == *SHUTDOWN_CMD {
            let _ = bash.kill();
            let _ = bash.wait();
            let _ = fs::remove_file(socket_path);
            return;
        }

        if *command == *PING_CMD {
            let _ = stream.write_all(PONG_RESPONSE);
            continue;
        }

        // Build the bash script to execute
        let script = build_daemon_script(&command);

        // Send to bash
        if writeln!(writer, "{script}").is_err() || writer.flush().is_err() {
            // Bash process died
            let _ = stream.write_all(b"reef daemon: bash process died\n");
            let _ = bash.kill();
            let _ = fs::remove_file(socket_path);
            return;
        }

        // Read bash output until DONE_SENTINEL
        let mut response = Vec::with_capacity(4096);
        loop {
            let mut line = Vec::new();
            match reader.read_until(b'\n', &mut line) {
                Ok(0) | Err(_) => break, // EOF or error — bash died
                Ok(_) => {
                    response.extend_from_slice(&line);
                    if contains_sentinel(&response, DONE_SENTINEL) {
                        break;
                    }
                }
            }
        }

        // Send response back to client
        let _ = stream.write_all(&response);

        // Check if bash is still alive
        if let Some(_status) = bash.try_wait().ok().flatten() {
            let _ = fs::remove_file(socket_path);
            return;
        }
    }
}

/// Build a bash script block for the daemon to eval.
///
/// The script:
/// 1. Evals the user's command with output to stdout (inherited as stderr)
/// 2. Captures exit code
/// 3. Prints env dump with null-delimited sentinels
fn build_daemon_script(command: &str) -> String {
    // Escape command for eval (single-quote it)
    let mut escaped = String::with_capacity(command.len() + 2);
    escaped.push('\'');
    for &b in command.as_bytes() {
        if b == b'\'' {
            escaped.push_str("'\\''");
        } else {
            escaped.push(b as char);
        }
    }
    escaped.push('\'');

    let mut s = String::with_capacity(escaped.len() + 256);
    s.push_str("eval ");
    s.push_str(&escaped);
    s.push_str(" >&2\n");
    s.push_str("__reef_exit=$?\n");
    s.push_str("printf '\\0__REEF_DAEMON_ENV__\\0'\n");
    s.push_str("env -0\n");
    s.push_str("printf '\\0__REEF_DAEMON_CWD__\\0'\n");
    s.push_str("pwd\n");
    s.push_str("printf '\\0__REEF_DAEMON_EXIT__\\0%d\\0__REEF_DAEMON_DONE__\\0\\n' $__reef_exit\n");
    s
}

/// Check if a byte slice contains a sentinel string.
fn contains_sentinel(data: &[u8], sentinel: &str) -> bool {
    let sentinel_bytes = sentinel.as_bytes();
    data.windows(sentinel_bytes.len())
        .any(|w| w == sentinel_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_sentinel_finds_match() {
        let data = b"hello\0__REEF_DAEMON_DONE__\0\n";
        assert!(contains_sentinel(data, DONE_SENTINEL));
    }

    #[test]
    fn contains_sentinel_no_match() {
        let data = b"hello world\n";
        assert!(!contains_sentinel(data, DONE_SENTINEL));
    }

    #[test]
    fn build_daemon_script_format() {
        let script = build_daemon_script("echo hello");
        assert!(script.contains("eval 'echo hello'"));
        assert!(script.contains("__reef_exit=$?"));
        assert!(script.contains("env -0"));
        assert!(script.contains("pwd"));
    }

    #[test]
    fn build_daemon_script_escapes_quotes() {
        let script = build_daemon_script("echo 'it'\"s\"");
        assert!(script.contains("'\\''"));
    }

    #[test]
    fn parse_response_extracts_exit_code() {
        let before = EnvSnapshot::new(
            std::collections::HashMap::new(),
            "/home".to_string(),
        );

        let mut response = Vec::new();
        response.extend_from_slice(b"output text");
        response.extend_from_slice(ENV_SENTINEL.as_bytes());
        response.extend_from_slice(b"MY_VAR=hello\0");
        response.extend_from_slice(CWD_SENTINEL.as_bytes());
        response.extend_from_slice(b"/tmp\n");
        response.extend_from_slice(EXIT_SENTINEL.as_bytes());
        response.extend_from_slice(b"42");
        response.extend_from_slice(DONE_SENTINEL.as_bytes());

        let exit_code = parse_and_print_response(&before, &response);
        assert_eq!(exit_code, 42);
    }
}

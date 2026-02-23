//! Reef CLI — binary entry point.

use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("usage: reef <detect|translate|bash-exec|daemon> [flags] -- <command>");
        process::exit(2);
    }

    match args[1].as_str() {
        "--version" | "-V" => {
            println!("reef {}", env!("CARGO_PKG_VERSION"));
        }
        "detect" => {
            // --quick and full detection use the same heuristic for now;
            // full AST-based detection may be added later.
            let cmd = collect_after_dashdash(&args[2..]);
            if reef::detect::looks_like_bash(&cmd) {
                process::exit(0);
            } else {
                process::exit(1);
            }
        }
        "translate" => {
            let cmd = collect_after_dashdash(&args[2..]);
            match reef::translate::translate_bash_to_fish(&cmd) {
                Ok(fish_code) => print!("{fish_code}"),
                Err(e) => {
                    eprintln!("reef: translation failed: {e}");
                    process::exit(1);
                }
            }
        }
        "bash-exec" => {
            let env_diff = args[2..].iter().any(|a| a == "--env-diff");
            let state_file = find_flag_value(&args[2..], "--state-file");
            let cmd = collect_after_dashdash(&args[2..]);
            let exit_code = if env_diff {
                reef::passthrough::bash_exec_env_diff(&cmd)
            } else if let Some(path) = state_file {
                reef::passthrough::bash_exec_with_state(&cmd, Path::new(&path))
            } else {
                reef::passthrough::bash_exec(&cmd)
            };
            process::exit(exit_code);
        }
        "daemon" => {
            if args.len() < 3 {
                eprintln!("usage: reef daemon <start|stop|status|exec> --socket <path>");
                process::exit(2);
            }
            let socket = find_flag_value(&args[3..], "--socket").unwrap_or_else(|| {
                eprintln!("reef daemon: --socket <path> is required");
                process::exit(2);
            });
            match args[2].as_str() {
                "start" => reef::daemon::start(&socket),
                "stop" => reef::daemon::stop(&socket),
                "status" => {
                    if reef::daemon::status(&socket) {
                        println!("reef daemon: running");
                    } else {
                        println!("reef daemon: not running");
                        process::exit(1);
                    }
                }
                "exec" => {
                    let cmd = collect_after_dashdash(&args[3..]);
                    let exit_code = reef::daemon::exec(&socket, &cmd);
                    process::exit(exit_code);
                }
                // Internal: called by `start` to run the daemon loop
                "_serve" => reef::daemon::serve(&socket),
                other => {
                    eprintln!("reef daemon: unknown subcommand '{other}'");
                    process::exit(2);
                }
            }
        }
        other => {
            eprintln!("reef: unknown command '{other}'");
            process::exit(2);
        }
    }
}

/// Collect all arguments after `--` into a single string.
fn collect_after_dashdash(args: &[String]) -> String {
    if let Some(pos) = args.iter().position(|a| a == "--") {
        args[pos + 1..].join(" ")
    } else {
        args.join(" ")
    }
}

/// Find the value of a `--flag value` pair in an argument list.
fn find_flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find_map(|pair| {
        if pair[0] == flag {
            Some(pair[1].clone())
        } else {
            None
        }
    })
}

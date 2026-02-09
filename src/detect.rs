/// Quick check: does this string contain bash-specific syntax?
/// This must be FAST — it runs on every Enter keypress.
/// No regex, no parsing — just string contains checks.
pub fn looks_like_bash(input: &str) -> bool {
    // Bash keyword checks
    let bash_indicators = [
        "export ", "unset ", "declare ", "local ",
        " then", ";then", "\tthen",
        " fi", ";fi",
        " done", ";done",
        " do ", ";do ", "do\n", "do;",
        " esac", ";esac",
        "${", "$((", "<<<",
        "function ", "shopt ",
        "read -p", "read -r",
    ];

    for indicator in &bash_indicators {
        if input.contains(indicator) {
            return true;
        }
    }

    // $() command substitution — bash uses $(cmd), fish uses (cmd)
    if input.contains("$(") {
        return true;
    }

    // [[ ]] double bracket tests
    if input.contains("[[") && input.contains("]]") {
        return true;
    }

    // Brace range expansion: {1..5}, {a..z}, {1..10..2}
    // Fish doesn't support {N..N} ranges
    if has_brace_range(input) {
        return true;
    }

    // C-style for loop: for ((i=0; ...
    if input.contains("for ((") {
        return true;
    }

    // Process substitution: <(cmd) or >(cmd)
    if input.contains("<(") || input.contains(">(") {
        return true;
    }

    false
}

/// Check for bash brace range expansion like {1..5} or {a..z}
fn has_brace_range(input: &str) -> bool {
    let mut i = 0;
    let bytes = input.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'{' {
            // Look for ..} pattern inside braces
            if let Some(close) = input[i..].find('}') {
                let inner = &input[i + 1..i + close];
                if inner.contains("..") {
                    let parts: Vec<&str> = inner.split("..").collect();
                    if parts.len() >= 2
                        && !parts[0].is_empty()
                        && !parts[1].is_empty()
                    {
                        return true;
                    }
                }
            }
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_export() {
        assert!(looks_like_bash("export PATH=/usr/bin:$PATH"));
        assert!(looks_like_bash("export EDITOR=vim"));
    }

    #[test]
    fn detects_for_loop() {
        assert!(looks_like_bash("for i in $(seq 5); do echo $i; done"));
    }

    #[test]
    fn detects_if_then() {
        assert!(looks_like_bash("if [ -f foo ]; then echo yes; fi"));
    }

    #[test]
    fn detects_command_substitution() {
        assert!(looks_like_bash("echo $(whoami)"));
    }

    #[test]
    fn detects_double_brackets() {
        assert!(looks_like_bash("[[ -n \"$HOME\" ]] && echo yes"));
    }

    #[test]
    fn detects_parameter_expansion() {
        assert!(looks_like_bash("echo ${HOME:-/tmp}"));
    }

    #[test]
    fn ignores_plain_fish() {
        assert!(!looks_like_bash("echo hello"));
        assert!(!looks_like_bash("set -gx PATH /usr/bin $PATH"));
        assert!(!looks_like_bash("for i in (seq 5); echo $i; end"));
    }
}

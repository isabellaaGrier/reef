# fishgate — Bash Compatibility Layer for Fish Shell

## The Problem

Fish is the best interactive shell — fastest out of the box, best autosuggestions,
best syntax highlighting, zero configuration needed. But it has one fatal flaw:
paste a bash command from Stack Overflow and it breaks. `export`, `$()`, `for/do/done`,
`[[ ]]` — none of it works. This is the #1 reason people don't switch to fish.

## The Solution

fishgate is a Rust-powered translation daemon that makes bash syntax work seamlessly
inside fish. No prefix commands, no mode switching. You just type. If it's fish, fish
runs it. If it's bash, fishgate catches it, translates it, and executes it — all in
under 1ms for common patterns.

Combined with fish-rust-wrappers (grep→rg, find→fd, sed→sd, du→dust, ps→procs),
the entire terminal experience becomes: "everything you already know works, but faster."

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    User types command                    │
│                   in kitty terminal                      │
└──────────────────────┬───────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────┐
│                     fish shell                           │
│                                                          │
│  1. Fish parser attempts to parse the input              │
│  2. If it succeeds → execute natively (fast path)        │
│  3. If it fails → triggers one of two hooks:             │
│     a. fish_command_not_found (unknown command like      │
│        'export') → fishgate handles it                   │
│     b. Syntax error (like for/do/done) → requires        │
│        a different interception strategy                 │
└──────────────────────┬───────────────────────────────────┘
                       │ failed
                       ▼
┌──────────────────────────────────────────────────────────┐
│              fishgate (Rust binary)                      │
│                                                          │
│  TIER 1: Keyword Functions (zero-cost, native fish)      │
│  ─────────────────────────────────────────────────       │
│  Fish function wrappers installed for known bash-isms    │
│                                                          │
│    export VAR=val     → set -gx VAR val                  │
│    unset VAR          → set -e VAR                       │
│    source file.sh     → bass source file.sh              │
│    [[ condition ]]    → test condition                   │
│    VAR=x command      → env VAR=x command                │
│                                                          │
│  These are fish functions that call the Rust binary      │
│  for translation. Sub-millisecond. Feels native.         │
│                                                          │
│  TIER 2: AST Translation (fast, ~1ms)                    │
│  ─────────────────────────────────────                   │
│  For multi-statement bash (for/do/done, if/then/fi):     │
│                                                          │
│  1. conch-parser parses the bash into an AST             │
│  2. Walk the AST, emit fish equivalents:                 │
│       for i in $(seq 5); do       →  for i in (seq 5)    │
│         echo $i                   →    echo $i           │
│       done                        →  end                 │
│       ${var:-default}             →  if set -q var;      │
│                                        echo $var;        │
│                                      else;               │
│                                        echo default;     │
│                                      end                 │
│  3. Execute the translated fish code                     │
│                                                          │
│  TIER 3: Bash Passthrough (slower, ~5-10ms)              │
│  ──────────────────────────────────────────              │
│  For anything too complex to translate:                  │
│                                                          │
│  1. Snapshot current env (PATH, vars, cwd, aliases)      │
│  2. Run: bash -c "$original_command"                     │
│  3. Stream stdout/stderr to terminal in real time        │
│  4. Snapshot env after execution                         │
│  5. Diff the two snapshots                               │
│  6. Apply changes back to fish session:                  │
│     - New/changed env vars → set -gx                     │
│     - Changed PATH → set PATH                            │
│     - Changed cwd → cd                                   │
│  7. User sees output as if fish ran it                   │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

---

## Interception Strategy

The tricky part: fish_command_not_found only fires for unknown *commands*,
not syntax errors. We need multiple interception layers:

### Layer 1: Fish Functions for Bash Keywords

Install fish functions that shadow common bash-only commands.
These ARE the command — fish calls them natively.

```
~/.config/fish/functions/
├── export.fish        # export VAR=val → set -gx VAR val
├── unset.fish         # unset VAR → set -e VAR
├── declare.fish       # declare -x VAR=val → set -gx VAR val
├── local.fish         # local VAR=val → set -l VAR val
├── readonly.fish      # readonly VAR=val → set -g VAR val (read-only not native)
├── shopt.fish         # no-op or translate known options
├── alias_bash.fish    # alias x='y' → alias x 'y' (fish-native)
└── source.fish        # source file.sh → detect bash, use bass-style passthrough
```

Example — `export.fish`:
```fish
function export --description "bash export → fish set -gx"
    for arg in $argv
        if string match -qr '^([^=]+)=(.*)$' -- $arg
            set -l varname (string replace -r '=.*' '' -- $arg)
            set -l varval (string replace -r '^[^=]+=' '' -- $arg)
            set -gx $varname $varval
        else
            # export VAR (no value) — just mark as exported
            set -gx $arg $$arg
        end
    end
end
```

This is zero-cost. Fish finds the function, calls it. No daemon needed.
Covers: export, unset, source, declare, local.

### Layer 2: fish_command_not_found → fishgate binary

For commands fish doesn't recognize at all.

```fish
# ~/.config/fish/functions/fish_command_not_found.fish
function fish_command_not_found
    # Pass the entire failed command line to fishgate
    set -l result (fishgate translate -- $argv 2>/dev/null)
    if test $status -eq 0
        eval $result
    else
        # fishgate couldn't translate — try bash passthrough
        fishgate bash-exec -- $argv
        if test $status -eq 127
            # Not valid bash either — show real error
            __fish_default_command_not_found_handler $argv[1]
        end
    end
end
```

### Layer 3: Keybinding Hook for Syntax Errors

For multi-line bash like `for i in $(seq 5); do echo $i; done`,
fish won't even try to execute — it's a parse error.

Solution: bind a custom handler to Enter that pre-screens the input.

```fish
# In config.fish or a conf.d file
function __fishgate_preexec --on-event fish_preexec
    # fish_preexec fires BEFORE execution with the command string
    # We can't intercept parse failures this way though...
end

# Better: replace the Enter key binding
function __fishgate_execute
    set -l cmd (commandline)
    # Quick check: does this look like bash syntax?
    if fishgate detect --quick -- $cmd
        # It's bash — translate and replace the commandline
        set -l translated (fishgate translate -- $cmd)
        if test $status -eq 0
            commandline -r $translated
        else
            # Fall back to bash passthrough
            commandline -r "fishgate bash-exec -- "(string escape -- $cmd)
        end
    end
    commandline -f execute
end
bind \r __fishgate_execute
```

This is the key innovation. By hooking Enter, we intercept bash syntax
BEFORE fish tries to parse it. The detection is sub-millisecond (just
pattern matching on keywords like `do`, `done`, `fi`, `then`, `$()`,
`$(())`). If detected, we translate inline and fish sees valid fish code.

---

## Rust Binary: `fishgate`

### Crate Structure

```
fishgate/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point
│   ├── detect.rs            # Fast bash-ism detection (regex patterns)
│   ├── translate.rs         # AST-based bash→fish translation
│   ├── passthrough.rs       # bash -c execution with env capture
│   ├── env_diff.rs          # Environment snapshot & diff
│   └── patterns.rs          # Known bash→fish pattern mappings
```

### Dependencies

```toml
[package]
name = "fishgate"
version = "0.1.0"
edition = "2021"

[dependencies]
conch-parser = "0.1"       # Bash AST parsing
shell-quote = "0.3"        # Safe quoting for fish/bash
clap = { version = "4", features = ["derive"] }  # CLI args

[profile.release]
opt-level = 3
lto = true                 # Link-time optimization for speed
strip = true               # Small binary
```

### CLI Interface

```
fishgate detect --quick -- "export PATH=/usr/bin:$PATH"
  → exit 0 (bash detected) or exit 1 (not bash)

fishgate translate -- "export PATH=/usr/bin:\$PATH"
  → prints: set -gx PATH /usr/bin $PATH

fishgate translate -- "for i in \$(seq 5); do echo \$i; done"
  → prints: for i in (seq 5)\n  echo $i\nend

fishgate bash-exec -- "complex_bash_command --with 'args'"
  → executes via bash, streams output, applies env changes

fishgate bash-exec --env-diff -- "source ~/.nvm/nvm.sh"
  → executes, captures env changes, prints fish set commands
```

### Detection (patterns.rs) — sub-microsecond

```rust
/// Quick check: does this string contain bash-specific syntax?
/// This must be FAST — it runs on every Enter keypress.
pub fn looks_like_bash(input: &str) -> bool {
    // Keyword checks (exact word boundary matches)
    let bash_keywords = [
        "export ", "unset ", "declare ", "local ",
        "then", "elif", " fi", ";fi", " done", ";done",
        "do\n", "do;", " do ", "esac",
        "${", "$((", "<<<",
        "function ", "shopt ",
        "read -p", "read -r",
    ];

    // Fast scan — no regex, just contains checks
    for keyword in &bash_keywords {
        if input.contains(keyword) {
            return true;
        }
    }

    // Check for $() command substitution (but not fish's ())
    // Bash: $(command)  Fish: (command)
    if input.contains("$(") {
        return true;
    }

    // Check for [[ ]] double bracket tests
    if input.contains("[[") && input.contains("]]") {
        return true;
    }

    // Check for VAR=value at start of line (assignment without set)
    // But NOT VAR=value command (which is env prefix, handled differently)
    // This is tricky — skip for now, let AST handle it

    false
}
```

### Translation (translate.rs) — ~1ms

```rust
use conch_parser::lexer::Lexer;
use conch_parser::parse::DefaultParser;

pub fn translate_bash_to_fish(input: &str) -> Result<String, TranslateError> {
    let lex = Lexer::new(input.chars());
    let parser = DefaultParser::new(lex);

    let mut fish_output = String::new();

    for cmd in parser {
        match cmd? {
            // export VAR=val → set -gx VAR val
            Command::Simple { ref words, .. } if is_export(words) => {
                fish_output.push_str(&translate_export(words));
            }

            // for VAR in WORDS; do BODY; done → for VAR in WORDS\n BODY\nend
            Command::For { ref var, ref words, ref body, .. } => {
                fish_output.push_str(&format!("for {} in {}\n", var,
                    translate_words(words)));
                fish_output.push_str(&translate_body(body));
                fish_output.push_str("end\n");
            }

            // if COND; then BODY; fi → if COND\n BODY\nend
            Command::If { ref conditionals, ref else_branch, .. } => {
                fish_output.push_str(&translate_if(conditionals, else_branch));
            }

            // while COND; do BODY; done → while COND\n BODY\nend
            Command::While { ref guard, ref body, .. } => {
                fish_output.push_str(&translate_while(guard, body));
            }

            // Anything else — try simple word-level translation
            Command::Simple { ref words, ref redirects, .. } => {
                fish_output.push_str(&translate_simple(words, redirects));
            }

            // Can't translate — return error, caller falls back to bash -c
            _ => return Err(TranslateError::Unsupported),
        }
    }

    Ok(fish_output)
}

/// Translate bash word constructs to fish equivalents
fn translate_word(word: &Word) -> String {
    match word {
        // $(command) → (command)
        Word::CommandSubst(inner) => {
            format!("({})", translate_bash_to_fish(inner).unwrap_or(inner.clone()))
        }

        // ${var:-default} → (set -q var; and echo $var; or echo default)
        Word::ParamDefault(var, default) => {
            format!("(set -q {var}; and echo ${var}; or echo {default})")
        }

        // ${var} → $var
        Word::ParamBrace(var) => format!("${}", var),

        // $((expr)) → (math expr)
        Word::ArithSubst(expr) => format!("(math {})", expr),

        // Regular word — pass through
        Word::Literal(s) => s.clone(),
    }
}
```

### Passthrough (passthrough.rs) — ~5-10ms

```rust
use std::process::{Command, Stdio};
use std::collections::HashMap;

pub struct EnvSnapshot {
    vars: HashMap<String, String>,
    cwd: String,
}

impl EnvSnapshot {
    /// Capture current environment via bash
    pub fn capture() -> Self {
        let output = Command::new("bash")
            .args(["-c", "env && echo __FISHGATE_CWD__ && pwd"])
            .output()
            .expect("bash available");

        // Parse env vars and cwd from output
        // ...
        Self { vars, cwd }
    }

    /// Diff two snapshots, return fish commands to apply changes
    pub fn diff(&self, after: &EnvSnapshot) -> Vec<String> {
        let mut commands = Vec::new();

        // New or changed vars
        for (key, val) in &after.vars {
            match self.vars.get(key) {
                Some(old_val) if old_val != val => {
                    commands.push(format!("set -gx {} {}", key,
                        shell_quote::Fish::quote_string(val)));
                }
                None => {
                    commands.push(format!("set -gx {} {}", key,
                        shell_quote::Fish::quote_string(val)));
                }
                _ => {}
            }
        }

        // Removed vars
        for key in self.vars.keys() {
            if !after.vars.contains_key(key) {
                commands.push(format!("set -e {}", key));
            }
        }

        // Changed directory
        if self.cwd != after.cwd {
            commands.push(format!("cd {}", shell_quote::Fish::quote_string(&after.cwd)));
        }

        commands
    }
}

/// Execute a command through bash, stream output, capture env changes
pub fn bash_exec(command: &str) -> i32 {
    let before = EnvSnapshot::capture();

    // Run the actual command with stdout/stderr inherited (streams to terminal)
    let status = Command::new("bash")
        .args(["-c", command])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("bash available");

    // Capture env after
    // We need to run the command AND capture env in one bash invocation:
    // bash -c "$command; echo __FISHGATE_ENV__; env; echo __FISHGATE_CWD__; pwd"
    // But we already streamed output above...
    //
    // Better approach: use a wrapper script that:
    // 1. Sources the command
    // 2. Writes env to a temp file
    // 3. We read the temp file after
    let after = EnvSnapshot::capture_after(command);

    // Print env diff as fish commands to stdout
    for cmd in before.diff(&after) {
        println!("{}", cmd);
    }

    status.code().unwrap_or(1)
}
```

---

## Installation

### What Gets Installed

```
~/.cargo/bin/fishgate                          # Rust binary
~/.config/fish/functions/export.fish           # Tier 1: keyword wrappers
~/.config/fish/functions/unset.fish
~/.config/fish/functions/declare.fish
~/.config/fish/functions/source.fish           # Smart: detects .sh → bass-style
~/.config/fish/functions/fish_command_not_found.fish  # Tier 2: catch unknown commands
~/.config/fish/conf.d/fishgate.fish            # Tier 3: Enter key hook + bashrc sourcing + config
~/.config/fish/functions/{grep,find,sed,du,ps}.fish  # Bonus: rust tool wrappers
```

### Install Script

```fish
# Install the Rust binary
cargo install --path .

# Install fish components
fish install.fish

# Done. No configuration needed.
exec fish
```

---

## What Works After Installation

```bash
# All of these "just work" in fish:

export PATH="/opt/bin:$PATH"             # ✓ keyword wrapper
export EDITOR=vim                        # ✓ keyword wrapper
unset OLDVAR                             # ✓ keyword wrapper
source ~/.bashrc                         # ✓ smart source (bass-style)
$(which python3) --version               # ✓ AST translation → (which python3)
for i in $(seq 10); do echo $i; done    # ✓ AST translation
if [ -f ~/.config ]; then echo y; fi    # ✓ AST translation
VAR=hello && echo $VAR                   # ✓ AST translation
[[ -n "$HOME" ]] && echo yes             # ✓ AST translation
echo $((2 + 2))                          # ✓ AST translation → (math 2 + 2)
${HOME:-/tmp}                            # ✓ AST translation
curl https://install.sh | bash           # ✓ passthrough (piped to bash anyway)
source <(kubectl completion bash)        # ✓ passthrough with env capture
. ~/.nvm/nvm.sh && nvm use 18           # ✓ passthrough with env capture

# Plus fish-native commands still work perfectly:
set -gx EDITOR vim                       # ✓ native fish
for i in (seq 10); echo $i; end         # ✓ native fish
grep -r "pattern" --include='*.txt' .    # ✓ rust wrapper → rg
find . -name '*.py' -type f             # ✓ rust wrapper → fd
```

---

## Automatic .bashrc Sourcing

Many tools (nvm, conda, pyenv, sdkman, cargo, etc.) write their initialization
lines to `~/.bashrc` during installation. Without this, those tools silently
break in fish. fishgate solves this at startup:

```fish
# conf.d/fishgate.fish (runs on every shell startup)

# Auto-source .bashrc so tools that install there just work
if test -f ~/.bashrc
    fishgate bash-exec --env-diff -- "source ~/.bashrc" | source
end
```

This runs once on shell startup, sources `.bashrc` through bash, captures
every env change, PATH addition, and variable that any installer ever wrote
in there, and applies it all to the fish session. Takes ~10-15ms on startup —
imperceptible given fish's already-fast launch time.

This means:
- `nvm` installs and writes to `.bashrc` → works in fish automatically
- `conda init` adds bash hooks to `.bashrc` → works in fish automatically
- Any `export PATH=...` an installer appended → applied to fish automatically
- User never needs to manually port `.bashrc` lines to `config.fish`

Combined with the three translation tiers, this closes the last gap:
there is no bash workflow that doesn't work in fish after fishgate is installed.

---

## Performance Targets

| Path                    | Latency      | When                              |
|-------------------------|-------------|-----------------------------------|
| Native fish             | 0ms         | Valid fish syntax                 |
| Tier 1 keyword wrapper  | <0.1ms      | export, unset, declare, source    |
| Tier 2 detect (Enter)   | <0.5ms      | Every Enter keypress              |
| Tier 2 AST translate    | <2ms        | Detected bash, simple patterns    |
| Tier 3 bash passthrough | 5-15ms      | Complex/unknown bash              |

The Enter-key detection MUST be sub-millisecond. It runs on every command.
It's just string contains checks — no regex, no parsing. If it doesn't
detect bash, the overhead is effectively zero.

---

## Project Name Options

- **fishgate** — gateway between bash and fish
- **fishbash** — obvious but a bit awkward
- **reef** — where fish and shells meet
- **angler** — catches bash, converts to fish
- **tideshell** — fish that handles all tides

---

## Repo Structure

```
fishgate/
├── README.md
├── LICENSE (MIT)
├── Cargo.toml
├── src/                    # Rust binary
│   ├── main.rs
│   ├── detect.rs
│   ├── translate.rs
│   ├── passthrough.rs
│   ├── env_diff.rs
│   └── patterns.rs
├── fish/                   # Fish components
│   ├── install.fish
│   ├── functions/
│   │   ├── export.fish
│   │   ├── unset.fish
│   │   ├── declare.fish
│   │   ├── source.fish
│   │   ├── fish_command_not_found.fish
│   │   ├── grep.fish      # rust tool wrappers
│   │   ├── find.fish
│   │   ├── sed.fish
│   │   ├── du.fish
│   │   └── ps.fish
│   └── conf.d/
│       └── fishgate.fish   # Enter key hook, config
└── tests/
    ├── test_detect.rs
    ├── test_translate.rs
    ├── test_passthrough.rs
    └── integration/
        └── test_commands.fish
```

---

## Development Phases

### Phase 1: Keyword Wrappers (week 1)
- export.fish, unset.fish, declare.fish, source.fish
- fish_command_not_found.fish (basic)
- Zero Rust needed — pure fish functions
- TEST: paste 20 common bash one-liners, verify they work

### Phase 2: fishgate binary — detect + translate (weeks 2-3)
- Rust binary with `detect` and `translate` subcommands
- conch-parser integration for AST walking
- Enter key hook (conf.d/fishgate.fish)
- Cover: for/do/done, if/then/fi, $(), ${}, $(())
- TEST: bash test suite of 50+ patterns

### Phase 3: Bash passthrough with env capture (week 4)
- bash-exec subcommand
- Environment snapshot & diff
- Source script support (source .bashrc, nvm.sh, etc.)
- TEST: source nvm, pyenv, conda — verify env changes persist

### Phase 4: Polish + Package (week 5)
- Installer script
- AUR package for Arch/CachyOS
- GitHub README with demos
- Integrate fish-rust-wrappers as optional component
- TEST: fresh CachyOS install → one command → everything works

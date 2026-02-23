//! Abstract syntax tree for bash commands.
//!
//! All AST nodes borrow from the input string (`&'a str`) — zero-copy.
//! The parser produces a `Vec<Cmd<'a>>` representing the top-level command list.

use std::borrow::Cow;

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// A complete command — foreground or background.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Cmd<'a> {
    /// A foreground command list.
    List(AndOrList<'a>),
    /// A background job (`cmd &`).
    Job(AndOrList<'a>),
}

/// A chain of commands connected by `&&` and `||`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndOrList<'a> {
    /// The first pipeline in the chain.
    pub first: Pipeline<'a>,
    /// Subsequent `&&` / `||` pipelines.
    pub rest: Vec<AndOr<'a>>,
}

/// A single `&&` or `||` link in an and-or chain.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AndOr<'a> {
    /// `&&` — run if the previous succeeded.
    And(Pipeline<'a>),
    /// `||` — run if the previous failed.
    Or(Pipeline<'a>),
}

/// A pipeline: one or more commands connected by `|`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Pipeline<'a> {
    /// A single command (no pipe).
    Single(Executable<'a>),
    /// `[!] cmd1 | cmd2 | ...` — bool is true if negated.
    Pipe(bool, Vec<Executable<'a>>),
}

/// An executable unit: simple command, compound command, or function definition.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Executable<'a> {
    /// A simple command (possibly with assignments and redirections).
    Simple(SimpleCmd<'a>),
    /// A compound command (`if`, `for`, `while`, etc.).
    Compound(CompoundCmd<'a>),
    /// A function definition: `name() { body; }`.
    FuncDef(&'a str, CompoundCmd<'a>),
}

// ---------------------------------------------------------------------------
// Simple command
// ---------------------------------------------------------------------------

/// A simple command with optional prefix assignments and suffix words/redirects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleCmd<'a> {
    /// Assignments and redirections before the command name.
    pub prefix: Vec<CmdPrefix<'a>>,
    /// Arguments and redirections after the command name.
    pub suffix: Vec<CmdSuffix<'a>>,
}

/// A prefix element: variable assignment or redirection.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CmdPrefix<'a> {
    /// `NAME=value` — scalar assignment.
    Assign(&'a str, Option<Word<'a>>),
    /// `arr=(word ...)` — array assignment.
    ArrayAssign(&'a str, Vec<Word<'a>>),
    /// `arr+=(word ...)` — array append.
    ArrayAppend(&'a str, Vec<Word<'a>>),
    /// An I/O redirection.
    Redirect(Redir<'a>),
}

/// A suffix element: argument word or redirection.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CmdSuffix<'a> {
    /// A regular argument word.
    Word(Word<'a>),
    /// An I/O redirection.
    Redirect(Redir<'a>),
}

// ---------------------------------------------------------------------------
// Compound commands
// ---------------------------------------------------------------------------

/// A compound command with optional trailing redirections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompoundCmd<'a> {
    /// The compound command body.
    pub kind: CompoundKind<'a>,
    /// Redirections applied to the entire compound command.
    pub redirects: Vec<Redir<'a>>,
}

/// The body of a compound command.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompoundKind<'a> {
    /// `for var [in words]; do body; done`
    For {
        /// Loop variable name.
        var: &'a str,
        /// Word list (None = `"$@"`).
        words: Option<Vec<Word<'a>>>,
        /// Loop body commands.
        body: Vec<Cmd<'a>>,
    },
    /// `while guard; do body; done`
    While(GuardBody<'a>),
    /// `until guard; do body; done`
    Until(GuardBody<'a>),
    /// `if cond; then body; [elif ...;] [else ...;] fi`
    If {
        /// Condition–body pairs (first is `if`, rest are `elif`).
        conditionals: Vec<GuardBody<'a>>,
        /// Optional `else` branch.
        else_branch: Option<Vec<Cmd<'a>>>,
    },
    /// `case word in pattern) body;; ... esac`
    Case {
        /// The word being matched.
        word: Word<'a>,
        /// Pattern–body arms.
        arms: Vec<CaseArm<'a>>,
    },
    /// C-style for loop: `for (( init; cond; step )); do body; done`
    CFor {
        /// Initialization expression.
        init: Option<Arith<'a>>,
        /// Condition expression.
        cond: Option<Arith<'a>>,
        /// Step expression.
        step: Option<Arith<'a>>,
        /// Loop body commands.
        body: Vec<Cmd<'a>>,
    },
    /// `{ body; }` — brace group.
    Brace(Vec<Cmd<'a>>),
    /// `( body )` — subshell.
    Subshell(Vec<Cmd<'a>>),
    /// `[[ expression ]]` — extended test command.
    DoubleBracket(Vec<Cmd<'a>>),
    /// `(( expression ))` — arithmetic command.
    Arithmetic(Arith<'a>),
}

/// A guard (condition) and body pair, used by `while`, `until`, and `if`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardBody<'a> {
    /// The condition commands.
    pub guard: Vec<Cmd<'a>>,
    /// The body commands.
    pub body: Vec<Cmd<'a>>,
}

/// A single arm in a `case` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseArm<'a> {
    /// Patterns to match against (separated by `|`).
    pub patterns: Vec<Word<'a>>,
    /// Commands to execute if a pattern matches.
    pub body: Vec<Cmd<'a>>,
}

// ---------------------------------------------------------------------------
// Words
// ---------------------------------------------------------------------------

/// A shell word: either a single part or a concatenation of parts.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Word<'a> {
    /// A word consisting of a single part.
    Simple(WordPart<'a>),
    /// A word formed by concatenating multiple parts (e.g., `"hello"$var`).
    Concat(Vec<WordPart<'a>>),
}

/// A fragment of a word: bare text, quoted text, or a substitution.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WordPart<'a> {
    /// Unquoted content.
    Bare(Atom<'a>),
    /// Double-quoted content (may contain expansions).
    DQuoted(Vec<Atom<'a>>),
    /// Single-quoted content (literal text, no expansions).
    SQuoted(&'a str),
}

/// An atomic element within a word: literal text, expansion, or glob.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Atom<'a> {
    /// Literal text.
    Lit(&'a str),
    /// Backslash-escaped character.
    Escaped(Cow<'a, str>),
    /// Parameter reference (`$var`, `$1`, `$@`, etc.).
    Param(Param<'a>),
    /// Substitution (`$(cmd)`, `${var...}`, `$((expr))`).
    Subst(Box<Subst<'a>>),
    /// `*` glob wildcard.
    Star,
    /// `?` glob wildcard.
    Question,
    /// `[` glob bracket open.
    SquareOpen,
    /// `]` glob bracket close.
    SquareClose,
    /// `~` tilde expansion.
    Tilde,
    /// `<(cmd)` — process substitution (input).
    ProcSubIn(Vec<Cmd<'a>>),
    /// ANSI-C `$'...'` — raw content between the quotes (escape sequences unresolved).
    AnsiCQuoted(&'a str),
    /// Brace range expansion: `{start..end[..step]}`.
    BraceRange {
        /// Range start value.
        start: &'a str,
        /// Range end value.
        end: &'a str,
        /// Optional step value.
        step: Option<&'a str>,
    },
}

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

/// A shell parameter reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Param<'a> {
    /// Named variable (`$var`).
    Var(&'a str),
    /// Positional parameter (`$1`, `$2`, ...).
    Positional(u32),
    /// `$@` — all positional parameters (separate words).
    At,
    /// `$*` — all positional parameters (single word).
    Star,
    /// `$#` — number of positional parameters.
    Pound,
    /// `$?` — exit status of last command.
    Status,
    /// `$$` — process ID.
    Pid,
    /// `$!` — PID of last background process.
    Bang,
    /// `$-` — current shell option flags.
    Dash,
}

// ---------------------------------------------------------------------------
// Substitutions
// ---------------------------------------------------------------------------

/// A substitution or parameter expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Subst<'a> {
    /// Command substitution: `$(cmd)` or `` `cmd` ``.
    Cmd(Vec<Cmd<'a>>),
    /// Arithmetic expansion: `$((expr))`.
    Arith(Option<Arith<'a>>),
    /// String length: `${#var}`.
    Len(Param<'a>),
    /// `${!var}` — indirect variable expansion.
    Indirect(&'a str),
    /// `${!prefix*}` / `${!prefix@}` — list variables matching prefix.
    PrefixList(&'a str),
    /// `${var@Q}` — parameter transformation (quoting).
    Transform(&'a str, u8),
    /// `${var:-word}` / `${var-word}` — default value.
    Default(Param<'a>, Option<Word<'a>>),
    /// `${var:=word}` / `${var=word}` — assign default.
    Assign(Param<'a>, Option<Word<'a>>),
    /// `${var:?word}` / `${var?word}` — error if unset.
    Error(Param<'a>, Option<Word<'a>>),
    /// `${var:+word}` / `${var+word}` — alternate value.
    Alt(Param<'a>, Option<Word<'a>>),
    /// `${var%pattern}` — remove shortest suffix match.
    TrimSuffixSmall(Param<'a>, Option<Word<'a>>),
    /// `${var%%pattern}` — remove longest suffix match.
    TrimSuffixLarge(Param<'a>, Option<Word<'a>>),
    /// `${var#pattern}` — remove shortest prefix match.
    TrimPrefixSmall(Param<'a>, Option<Word<'a>>),
    /// `${var##pattern}` — remove longest prefix match.
    TrimPrefixLarge(Param<'a>, Option<Word<'a>>),
    /// `${var/pattern/replacement}` — replace first match.
    Replace(Param<'a>, Option<Word<'a>>, Option<Word<'a>>),
    /// `${var//pattern/replacement}` — replace all matches.
    ReplaceAll(Param<'a>, Option<Word<'a>>, Option<Word<'a>>),
    /// `${var/#pattern/replacement}` — replace prefix match.
    ReplacePrefix(Param<'a>, Option<Word<'a>>, Option<Word<'a>>),
    /// `${var/%pattern/replacement}` — replace suffix match.
    ReplaceSuffix(Param<'a>, Option<Word<'a>>, Option<Word<'a>>),
    /// `${var:offset:length}` — substring extraction.
    Substring(Param<'a>, &'a str, Option<&'a str>),
    /// `${var^}` / `${var^^}` — uppercase (bool: all if true).
    Upper(bool, Param<'a>),
    /// `${var,}` / `${var,,}` — lowercase (bool: all if true).
    Lower(bool, Param<'a>),
    /// `${arr[index]}` — array element access (index is a Word for $((expr)) support).
    ArrayElement(&'a str, Word<'a>),
    /// `${arr[@]}` or `${arr[*]}` — all array elements.
    ArrayAll(&'a str),
    /// `${#arr[@]}` — array length.
    ArrayLen(&'a str),
    /// `${arr[@]:offset:length}` — array slice.
    ArraySlice(&'a str, &'a str, Option<&'a str>),
}

// ---------------------------------------------------------------------------
// Arithmetic
// ---------------------------------------------------------------------------

/// An arithmetic expression node (used in `$(( ))`, `(( ))`, and C-style for).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Arith<'a> {
    /// Variable reference.
    Var(&'a str),
    /// Integer literal.
    Lit(i64),

    /// Addition.
    Add(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Subtraction.
    Sub(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Multiplication.
    Mul(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Division.
    Div(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Modulo.
    Rem(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Exponentiation.
    Pow(Box<Arith<'a>>, Box<Arith<'a>>),

    /// Less than.
    Lt(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Less than or equal.
    Le(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Greater than.
    Gt(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Greater than or equal.
    Ge(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Equal.
    Eq(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Not equal.
    Ne(Box<Arith<'a>>, Box<Arith<'a>>),

    /// Bitwise AND.
    BitAnd(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Bitwise OR.
    BitOr(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Bitwise XOR.
    BitXor(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Logical AND.
    LogAnd(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Logical OR.
    LogOr(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Left shift.
    Shl(Box<Arith<'a>>, Box<Arith<'a>>),
    /// Right shift.
    Shr(Box<Arith<'a>>, Box<Arith<'a>>),

    /// Unary plus.
    Pos(Box<Arith<'a>>),
    /// Unary minus.
    Neg(Box<Arith<'a>>),
    /// Logical NOT.
    LogNot(Box<Arith<'a>>),
    /// Bitwise NOT.
    BitNot(Box<Arith<'a>>),

    /// Pre-increment (`++var`).
    PreInc(&'a str),
    /// Post-increment (`var++`).
    PostInc(&'a str),
    /// Pre-decrement (`--var`).
    PreDec(&'a str),
    /// Post-decrement (`var--`).
    PostDec(&'a str),

    /// Ternary operator (`cond ? then : else`).
    Ternary(Box<Arith<'a>>, Box<Arith<'a>>, Box<Arith<'a>>),
    /// Assignment (`var = expr`).
    Assign(&'a str, Box<Arith<'a>>),
}

// ---------------------------------------------------------------------------
// Heredoc body
// ---------------------------------------------------------------------------

/// The body of a heredoc (here-document).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HeredocBody<'a> {
    /// Quoted delimiter — no expansion (literal text).
    Literal(&'a str),
    /// Unquoted delimiter — variable and command expansion.
    Interpolated(Vec<Atom<'a>>),
}

// ---------------------------------------------------------------------------
// Redirects
// ---------------------------------------------------------------------------

/// An I/O redirection.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Redir<'a> {
    /// `[n]< word` — read from file.
    Read(Option<u16>, Word<'a>),
    /// `[n]> word` — write to file.
    Write(Option<u16>, Word<'a>),
    /// `[n]>> word` — append to file.
    Append(Option<u16>, Word<'a>),
    /// `[n]<> word` — open for reading and writing.
    ReadWrite(Option<u16>, Word<'a>),
    /// `[n]>| word` — write, overriding noclobber.
    Clobber(Option<u16>, Word<'a>),
    /// `[n]<& word` — duplicate input fd.
    DupRead(Option<u16>, Word<'a>),
    /// `[n]>& word` — duplicate output fd.
    DupWrite(Option<u16>, Word<'a>),
    /// `<<< word` — here-string.
    HereString(Word<'a>),
    /// `<< [-]DELIM ... DELIM` — here-document.
    Heredoc(HeredocBody<'a>),
    /// `&> word` — redirect both stdout and stderr.
    WriteAll(Word<'a>),
    /// `&>> word` — append both stdout and stderr.
    AppendAll(Word<'a>),
}

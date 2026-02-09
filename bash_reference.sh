# ============================================================================
# REEF BASH REFERENCE
# ============================================================================
# Organized from most common → most obscure bash constructs.
# Each test includes the bash command and expected output.
# Tests marked [TRANSLATE] should be caught by Tier 2 AST translation.
# Tests marked [PASSTHROUGH] will need Tier 3 bash-exec.
# Tests marked [KEYWORD] should be caught by Tier 1 fish functions.
#
# Run each command in fish with reef installed. Verify output matches expected.
# ============================================================================


# ============================================================================
# CATEGORY 1: Variable Assignment & Export
# These are the bread and butter — every bash user types these daily.
# ============================================================================

# 1.1 [KEYWORD] Simple export
export FOO=bar
# Expected: $FOO is "bar"

# 1.2 [KEYWORD] Export with spaces (quoted)
export GREETING="hello world"
# Expected: $GREETING is "hello world"

# 1.3 [KEYWORD] Export PATH append
export PATH="/opt/mybin:$PATH"
# Expected: /opt/mybin prepended to PATH

# 1.4 [KEYWORD] Multiple exports on one line
export A=1 B=2 C=3
# Expected: $A=1, $B=2, $C=3

# 1.5 [KEYWORD] Export without value (mark existing var)
FOO=hello; export FOO
# Expected: $FOO is "hello" and exported

# 1.6 [KEYWORD] Unset
export FOO=bar; unset FOO
# Expected: $FOO is empty/undefined

# 1.7 [KEYWORD] declare -x (same as export)
declare -x MYVAR="test"
# Expected: $MYVAR is "test"

# 1.8 [KEYWORD] local (inside function context)
myfunc() { local x=5; echo $x; }; myfunc
# Expected: 5

# 1.9 [TRANSLATE] Inline assignment before command
FOO=bar echo "test"
# Expected: "test" (FOO set only for that command)

# 1.10 [TRANSLATE] Multiple inline assignments
CC=gcc CFLAGS="-O2" make
# Expected: make runs with CC=gcc and CFLAGS=-O2 in environment

# 1.11 [TRANSLATE] Assignment with command substitution
MYDIR=$(pwd)
# Expected: $MYDIR is current directory

# 1.12 [TRANSLATE] Assignment with backtick substitution
MYDIR=`pwd`
# Expected: $MYDIR is current directory


# ============================================================================
# CATEGORY 2: Command Substitution
# Second most common bash-ism that breaks in fish.
# ============================================================================

# 2.1 [TRANSLATE] Basic $() substitution
echo "User: $(whoami)"
# Expected: "User: <username>"

# 2.2 [TRANSLATE] Nested $() substitution
echo "Dir: $(basename $(pwd))"
# Expected: "Dir: <current_dir_name>"

# 2.3 [TRANSLATE] $() in variable assignment
FILES=$(ls *.txt 2>/dev/null | wc -l)
# Expected: count of .txt files

# 2.4 [TRANSLATE] Backtick substitution
echo "User: `whoami`"
# Expected: "User: <username>"

# 2.5 [TRANSLATE] $() with pipes inside
RESULT=$(echo "hello world" | tr ' ' '\n' | wc -l)
# Expected: RESULT is "2"

# 2.6 [TRANSLATE] $() with quotes inside
MSG=$(echo "it's a \"test\"")
# Expected: MSG is: it's a "test"

# 2.7 [PASSTHROUGH] Deeply nested substitution (3 levels)
echo "$(echo "$(echo "$(whoami)")")"
# Expected: <username>


# ============================================================================
# CATEGORY 3: Conditionals
# if/then/fi, [[ ]], test
# ============================================================================

# 3.1 [TRANSLATE] Simple if/then/fi
if [ -f /etc/hostname ]; then echo "exists"; fi
# Expected: "exists" (on most systems)

# 3.2 [TRANSLATE] if/then/else/fi
if [ -f /nonexistent ]; then echo "yes"; else echo "no"; fi
# Expected: "no"

# 3.3 [TRANSLATE] if/elif/else/fi
x=5; if [ $x -lt 3 ]; then echo "low"; elif [ $x -lt 7 ]; then echo "mid"; else echo "high"; fi
# Expected: "mid"

# 3.4 [TRANSLATE] [[ ]] double bracket test
if [[ -n "$HOME" ]]; then echo "HOME is set"; fi
# Expected: "HOME is set"

# 3.5 [TRANSLATE] [[ ]] with == pattern matching
if [[ "hello" == h* ]]; then echo "match"; fi
# Expected: "match"

# 3.6 [TRANSLATE] [[ ]] with && and ||
if [[ -d /tmp && -w /tmp ]]; then echo "writable tmp"; fi
# Expected: "writable tmp"

# 3.7 [TRANSLATE] [[ ]] regex match
if [[ "hello123" =~ ^[a-z]+[0-9]+$ ]]; then echo "match"; fi
# Expected: "match"

# 3.8 [TRANSLATE] test command (no brackets)
if test -d /tmp; then echo "yes"; fi
# Expected: "yes"

# 3.9 [TRANSLATE] Ternary-style with && ||
[ -d /tmp ] && echo "exists" || echo "missing"
# Expected: "exists"

# 3.10 [TRANSLATE] Negated test
if ! [ -f /nonexistent ]; then echo "missing"; fi
# Expected: "missing"


# ============================================================================
# CATEGORY 4: Loops
# for/do/done, while/do/done, until
# ============================================================================

# 4.1 [TRANSLATE] for with $() word list
for i in $(seq 1 5); do echo $i; done
# Expected: 1 2 3 4 5 (one per line)

# 4.2 [TRANSLATE] for with static word list
for color in red green blue; do echo $color; done
# Expected: red green blue (one per line)

# 4.3 [TRANSLATE] for with glob
for f in /tmp/*; do echo "$f"; done
# Expected: list of files in /tmp

# 4.4 [PASSTHROUGH] C-style for loop
for ((i=0; i<5; i++)); do echo $i; done
# Expected: 0 1 2 3 4

# 4.5 [TRANSLATE] while read loop
echo -e "one\ntwo\nthree" | while read line; do echo "Got: $line"; done
# Expected: "Got: one", "Got: two", "Got: three"

# 4.6 [TRANSLATE] while with condition
i=0; while [ $i -lt 5 ]; do echo $i; i=$((i+1)); done
# Expected: 0 1 2 3 4

# 4.7 [TRANSLATE] until loop
i=0; until [ $i -ge 3 ]; do echo $i; i=$((i+1)); done
# Expected: 0 1 2

# 4.8 [TRANSLATE] for with brace expansion
for i in {1..5}; do echo $i; done
# Expected: 1 2 3 4 5

# 4.9 [TRANSLATE] Nested loops
for i in 1 2; do for j in a b; do echo "$i$j"; done; done
# Expected: 1a 1b 2a 2b

# 4.10 [PASSTHROUGH] while read from file descriptor
while IFS=: read -r user _ _ _ _ home _; do echo "$user -> $home"; done < /etc/passwd | head -3
# Expected: first 3 users and their home dirs


# ============================================================================
# CATEGORY 5: Arithmetic
# $(( )), let, (( ))
# ============================================================================

# 5.1 [TRANSLATE] Basic $(( ))
echo $((2 + 3))
# Expected: 5

# 5.2 [TRANSLATE] $(( )) with variables
x=10; y=3; echo $((x * y))
# Expected: 30

# 5.3 [TRANSLATE] $(( )) modulo
echo $((17 % 5))
# Expected: 2

# 5.4 [TRANSLATE] $(( )) comparison (returns 0 or 1)
echo $((5 > 3))
# Expected: 1

# 5.5 [PASSTHROUGH] (( )) as test command
x=5; if ((x > 3)); then echo "big"; fi
# Expected: "big"

# 5.6 [PASSTHROUGH] (( )) increment
x=5; ((x++)); echo $x
# Expected: 6

# 5.7 [PASSTHROUGH] let command
let "x = 5 + 3"; echo $x
# Expected: 8

# 5.8 [PASSTHROUGH] Ternary in arithmetic
x=10; echo $((x > 5 ? 1 : 0))
# Expected: 1

# 5.9 [PASSTHROUGH] Arithmetic with bitwise ops
echo $((0xFF & 0x0F))
# Expected: 15

# 5.10 [PASSTHROUGH] Arithmetic assignment operators
x=10; ((x += 5)); ((x *= 2)); echo $x
# Expected: 30


# ============================================================================
# CATEGORY 6: Parameter Expansion
# ${var:-default}, ${var%pattern}, ${#var}, etc.
# ============================================================================

# 6.1 [TRANSLATE] Default value ${var:-default}
unset MYVAR; echo ${MYVAR:-"fallback"}
# Expected: "fallback"

# 6.2 [TRANSLATE] Assign default ${var:=default}
unset MYVAR; echo ${MYVAR:="assigned"}; echo $MYVAR
# Expected: "assigned" then "assigned"

# 6.3 [TRANSLATE] Error if unset ${var:?message}
# MYVAR must be set or this errors
MYVAR="ok"; echo ${MYVAR:?"not set"}
# Expected: "ok"

# 6.4 [TRANSLATE] Alternate value ${var:+alternate}
MYVAR="hello"; echo ${MYVAR:+"it exists"}
# Expected: "it exists"

# 6.5 [TRANSLATE] String length ${#var}
MYVAR="hello"; echo ${#MYVAR}
# Expected: 5

# 6.6 [TRANSLATE] Remove shortest suffix ${var%pattern}
FILE="document.tar.gz"; echo ${FILE%.*}
# Expected: "document.tar"

# 6.7 [TRANSLATE] Remove longest suffix ${var%%pattern}
FILE="document.tar.gz"; echo ${FILE%%.*}
# Expected: "document"

# 6.8 [TRANSLATE] Remove shortest prefix ${var#pattern}
PATH_STR="/home/user/docs"; echo ${PATH_STR#*/}
# Expected: "home/user/docs"

# 6.9 [TRANSLATE] Remove longest prefix ${var##pattern}
PATH_STR="/home/user/docs"; echo ${PATH_STR##*/}
# Expected: "docs"

# 6.10 [TRANSLATE] Substitution ${var/pattern/replacement}
STR="hello world"; echo ${STR/world/earth}
# Expected: "hello earth"

# 6.11 [TRANSLATE] Global substitution ${var//pattern/replacement}
STR="aabaa"; echo ${STR//a/x}
# Expected: "xxbxx"

# 6.12 [PASSTHROUGH] Substring extraction ${var:offset:length}
STR="hello world"; echo ${STR:0:5}
# Expected: "hello"

# 6.13 [PASSTHROUGH] Uppercase ${var^^}
STR="hello"; echo ${STR^^}
# Expected: "HELLO"

# 6.14 [PASSTHROUGH] Lowercase ${var,,}
STR="HELLO"; echo ${STR,,}
# Expected: "hello"

# 6.15 [PASSTHROUGH] Capitalize first ${var^}
STR="hello"; echo ${STR^}
# Expected: "Hello"

# 6.16 [PASSTHROUGH] Indirect variable reference ${!var}
MYVAR="hello"; ref=MYVAR; echo ${!ref}
# Expected: "hello"


# ============================================================================
# CATEGORY 7: Heredocs & Herestrings
# <<, <<-, <<<
# ============================================================================

# 7.1 [PASSTHROUGH] Basic heredoc
cat <<EOF
Hello World
This is a heredoc
EOF
# Expected: two lines printed

# 7.2 [PASSTHROUGH] Heredoc with variable expansion
NAME="Xavier"; cat <<EOF
Hello $NAME
EOF
# Expected: "Hello Xavier"

# 7.3 [PASSTHROUGH] Heredoc with NO expansion (quoted delimiter)
cat <<'EOF'
Hello $NAME
This is literal: $(whoami)
EOF
# Expected: literal "$NAME" and "$(whoami)" printed

# 7.4 [PASSTHROUGH] Heredoc with indentation stripping (<<-)
	cat <<-EOF
		Hello
		World
	EOF
# Expected: "Hello" and "World" without leading tabs

# 7.5 [PASSTHROUGH] Herestring <<<
grep "hello" <<< "hello world"
# Expected: "hello world"

# 7.6 [PASSTHROUGH] Herestring with variable
MSG="hello world"; grep "hello" <<< "$MSG"
# Expected: "hello world"

# 7.7 [PASSTHROUGH] Heredoc piped to command
cat <<EOF | wc -l
line1
line2
line3
EOF
# Expected: 3

# 7.8 [PASSTHROUGH] Heredoc to file
cat <<EOF > /tmp/reef_test_heredoc.txt
test content
second line
EOF
cat /tmp/reef_test_heredoc.txt; rm /tmp/reef_test_heredoc.txt
# Expected: "test content" and "second line"


# ============================================================================
# CATEGORY 8: Process Substitution
# <(), >()
# ============================================================================

# 8.1 [PASSTHROUGH] Basic process substitution as input
diff <(echo "hello") <(echo "world")
# Expected: diff output showing hello vs world

# 8.2 [PASSTHROUGH] Process substitution with sort
diff <(sort file1.txt) <(sort file2.txt)
# Expected: diff of sorted files (if files exist)

# 8.3 [PASSTHROUGH] Process substitution for paste
paste <(seq 1 3) <(seq 4 6)
# Expected: "1\t4", "2\t5", "3\t6"

# 8.4 [PASSTHROUGH] Writing to process substitution
echo "hello" > >(cat -n)
# Expected: "     1	hello"

# 8.5 [PASSTHROUGH] Process substitution in while read
while read line; do echo "got: $line"; done < <(echo -e "a\nb\nc")
# Expected: "got: a", "got: b", "got: c"


# ============================================================================
# CATEGORY 9: Arrays (Indexed)
# ============================================================================

# 9.1 [PASSTHROUGH] Array declaration and access
arr=(one two three); echo ${arr[1]}
# Expected: "two"

# 9.2 [PASSTHROUGH] Array all elements
arr=(one two three); echo ${arr[@]}
# Expected: "one two three"

# 9.3 [PASSTHROUGH] Array length
arr=(one two three); echo ${#arr[@]}
# Expected: 3

# 9.4 [PASSTHROUGH] Array append
arr=(one two); arr+=(three); echo ${arr[@]}
# Expected: "one two three"

# 9.5 [PASSTHROUGH] Array slice
arr=(a b c d e); echo ${arr[@]:1:3}
# Expected: "b c d"

# 9.6 [PASSTHROUGH] Array in for loop
arr=(red green blue); for c in "${arr[@]}"; do echo $c; done
# Expected: red green blue

# 9.7 [PASSTHROUGH] Array element deletion
arr=(one two three); unset arr[1]; echo ${arr[@]}
# Expected: "one three"

# 9.8 [PASSTHROUGH] Array with spaces in elements
arr=("hello world" "foo bar"); echo "${arr[0]}"
# Expected: "hello world"


# ============================================================================
# CATEGORY 10: Associative Arrays
# ============================================================================

# 10.1 [PASSTHROUGH] Declare and use
declare -A colors; colors[red]="#FF0000"; colors[blue]="#0000FF"; echo ${colors[red]}
# Expected: "#FF0000"

# 10.2 [PASSTHROUGH] Compound assignment
declare -A user=([name]="Xavier" [city]="Montana"); echo ${user[name]}
# Expected: "Xavier"

# 10.3 [PASSTHROUGH] All keys
declare -A m=([a]=1 [b]=2 [c]=3); echo ${!m[@]}
# Expected: "a b c" (order may vary)

# 10.4 [PASSTHROUGH] All values
declare -A m=([a]=1 [b]=2 [c]=3); echo ${m[@]}
# Expected: "1 2 3" (order may vary)

# 10.5 [PASSTHROUGH] Iterate associative array
declare -A m=([x]=10 [y]=20); for k in "${!m[@]}"; do echo "$k=${m[$k]}"; done
# Expected: "x=10" "y=20" (order may vary)


# ============================================================================
# CATEGORY 11: Case Statements
# ============================================================================

# 11.1 [TRANSLATE] Simple case
x="hello"; case $x in hello) echo "hi";; bye) echo "cya";; esac
# Expected: "hi"

# 11.2 [TRANSLATE] Case with wildcard
x="foobar"; case $x in foo*) echo "starts with foo";; *) echo "other";; esac
# Expected: "starts with foo"

# 11.3 [TRANSLATE] Case with multiple patterns
x="yes"; case $x in y|yes|Y|YES) echo "affirmative";; *) echo "negative";; esac
# Expected: "affirmative"

# 11.4 [PASSTHROUGH] Case with ;& fall-through (bash 4+)
x="a"; case $x in a) echo "a";& b) echo "b";; esac
# Expected: "a" and "b"


# ============================================================================
# CATEGORY 12: Functions
# ============================================================================

# 12.1 [TRANSLATE] Function with keyword
function greet { echo "Hello $1"; }; greet "World"
# Expected: "Hello World"

# 12.2 [TRANSLATE] Function without keyword
greet() { echo "Hello $1"; }; greet "World"
# Expected: "Hello World"

# 12.3 [PASSTHROUGH] Function with local variables
myfunc() { local x=5; local y=10; echo $((x + y)); }; myfunc
# Expected: 15

# 12.4 [PASSTHROUGH] Function with return value
is_even() { return $(( $1 % 2 )); }; is_even 4 && echo "even" || echo "odd"
# Expected: "even"

# 12.5 [PASSTHROUGH] Function using nameref (bash 4.3+)
set_var() { declare -n ref=$1; ref="hello"; }; set_var myvar; echo $myvar
# Expected: "hello"

# 12.6 [PASSTHROUGH] Recursive function
factorial() { if [ $1 -le 1 ]; then echo 1; else echo $(( $1 * $(factorial $(($1 - 1))) )); fi; }; factorial 5
# Expected: 120


# ============================================================================
# CATEGORY 13: Redirections & File Descriptors
# ============================================================================

# 13.1 [TRANSLATE] Stderr to stdout
ls /nonexistent 2>&1 | head -1
# Expected: error message

# 13.2 [TRANSLATE] Discard stderr
ls /nonexistent 2>/dev/null
# Expected: no output

# 13.3 [TRANSLATE] Redirect both stdout and stderr
ls /tmp /nonexistent &>/dev/null
# Expected: no output

# 13.4 [TRANSLATE] Append to file
echo "line1" > /tmp/reef_test.txt; echo "line2" >> /tmp/reef_test.txt; cat /tmp/reef_test.txt; rm /tmp/reef_test.txt
# Expected: "line1" and "line2"

# 13.5 [PASSTHROUGH] File descriptor manipulation
exec 3>/tmp/reef_fd_test.txt; echo "hello" >&3; exec 3>&-; cat /tmp/reef_fd_test.txt; rm /tmp/reef_fd_test.txt
# Expected: "hello"

# 13.6 [PASSTHROUGH] Read from fd
exec 3<<< "from fd 3"; read -r line <&3; echo $line; exec 3<&-
# Expected: "from fd 3"

# 13.7 [PASSTHROUGH] Swap stdout and stderr
{ echo "stdout"; echo "stderr" >&2; } 3>&1 1>&2 2>&3
# Expected: "stderr" on stdout, "stdout" on stderr


# ============================================================================
# CATEGORY 14: Signal Handling & Traps
# ============================================================================

# 14.1 [PASSTHROUGH] Trap on EXIT
bash -c 'trap "echo cleanup" EXIT; echo "running"'
# Expected: "running" then "cleanup"

# 14.2 [PASSTHROUGH] Trap on ERR
bash -c 'trap "echo error caught" ERR; false'
# Expected: "error caught"

# 14.3 [PASSTHROUGH] Trap cleanup with temp files
bash -c 'TMP=$(mktemp); trap "rm -f $TMP" EXIT; echo "test" > $TMP; cat $TMP'
# Expected: "test"

# 14.4 [PASSTHROUGH] Trap with multiple signals
bash -c 'trap "echo caught" SIGINT SIGTERM; echo "waiting"; sleep 0.1'
# Expected: "waiting"


# ============================================================================
# CATEGORY 15: Subshells & Brace Groups
# ============================================================================

# 15.1 [TRANSLATE] Subshell (parentheses)
(cd /tmp && echo "In: $(pwd)"); echo "Back: $(pwd)"
# Expected: "In: /tmp" then "Back: <original_dir>"

# 15.2 [TRANSLATE] Brace group
{ echo "one"; echo "two"; echo "three"; } | wc -l
# Expected: 3

# 15.3 [PASSTHROUGH] Subshell variable isolation
x=outer; (x=inner; echo $x); echo $x
# Expected: "inner" then "outer"

# 15.4 [PASSTHROUGH] Background subshell
(sleep 0.1; echo "background done") & wait
# Expected: "background done"


# ============================================================================
# CATEGORY 16: Special Variables
# ============================================================================

# 16.1 [TRANSLATE] $? exit status
true; echo $?
# Expected: 0

# 16.2 [TRANSLATE] $? after failure
false; echo $?
# Expected: 1

# 16.3 [TRANSLATE] $$ current PID
echo $$
# Expected: a number

# 16.4 [TRANSLATE] $! background PID
sleep 0.1 & echo $!
# Expected: a number

# 16.5 [PASSTHROUGH] $RANDOM
echo $RANDOM
# Expected: a random number 0-32767

# 16.6 [PASSTHROUGH] $LINENO
bash -c 'echo $LINENO'
# Expected: 1

# 16.7 [PASSTHROUGH] $SECONDS
bash -c 'SECONDS=0; sleep 0.1; echo $SECONDS'
# Expected: 0 (too fast to increment to 1)

# 16.8 [PASSTHROUGH] $BASH_VERSION
bash -c 'echo $BASH_VERSION'
# Expected: version string


# ============================================================================
# CATEGORY 17: Coprocesses (Bash 4.0+)
# ============================================================================

# 17.1 [PASSTHROUGH] Basic coproc
coproc { echo "hello from coproc"; }; read -r output <&"${COPROC[0]}"; echo "$output"
# Expected: "hello from coproc"

# 17.2 [PASSTHROUGH] Named coproc with interaction
coproc myproc { read line; echo "got: $line"; }; echo "test" >&"${myproc[1]}"; read -r out <&"${myproc[0]}"; echo "$out"
# Expected: "got: test"

# 17.3 [PASSTHROUGH] Coproc with awk
coproc awk '{print "processed: " $0; fflush()}'; echo "hello" >&"${COPROC[1]}"; read -r out <&"${COPROC[0]}"; echo "$out"
# Expected: "processed: hello"


# ============================================================================
# CATEGORY 18: Namerefs (Bash 4.3+)
# ============================================================================

# 18.1 [PASSTHROUGH] Basic nameref
bash -c 'x=hello; declare -n ref=x; echo $ref'
# Expected: "hello"

# 18.2 [PASSTHROUGH] Nameref assignment
bash -c 'declare -n ref=target; ref="world"; echo $target'
# Expected: "world"

# 18.3 [PASSTHROUGH] Nameref in function
bash -c 'set_result() { declare -n out=$1; out="done"; }; set_result myvar; echo $myvar'
# Expected: "done"


# ============================================================================
# CATEGORY 19: mapfile / readarray
# ============================================================================

# 19.1 [PASSTHROUGH] mapfile from command
bash -c 'mapfile -t lines <<< "$(echo -e "a\nb\nc")"; echo "${lines[1]}"'
# Expected: "b"

# 19.2 [PASSTHROUGH] readarray (alias for mapfile)
bash -c 'readarray -t nums < <(seq 1 5); echo "${nums[2]}"'
# Expected: "3"

# 19.3 [PASSTHROUGH] mapfile with callback
bash -c 'mapfile -t -C "echo processing:" -c 1 lines <<< "$(echo -e "x\ny")"'
# Expected: "processing: 0 x" and "processing: 1 y"


# ============================================================================
# CATEGORY 20: Obscure & Evil Edge Cases
# ============================================================================

# 20.1 [PASSTHROUGH] eval
bash -c 'cmd="echo hello"; eval $cmd'
# Expected: "hello"

# 20.2 [PASSTHROUGH] eval with variable indirection
bash -c 'name=FOO; FOO=bar; eval echo \$$name'
# Expected: "bar"

# 20.3 [PASSTHROUGH] Nested quotes
echo "She said \"it's a 'test'\""
# Expected: She said "it's a 'test'"

# 20.4 [PASSTHROUGH] ANSI-C quoting $'...'
echo $'Hello\tWorld\n'
# Expected: "Hello	World" with tab and trailing newline

# 20.5 [PASSTHROUGH] Extglob patterns
bash -O extglob -c 'for f in !(*.txt); do echo "$f"; done'
# Expected: files not ending in .txt

# 20.6 [PASSTHROUGH] set -e / set -o pipefail
bash -c 'set -eo pipefail; echo "start"; true; echo "end"'
# Expected: "start" "end"

# 20.7 [PASSTHROUGH] Brace expansion with sequences
echo {a..z}
# Expected: a b c d e f g h i j k l m n o p q r s t u v w x y z

# 20.8 [PASSTHROUGH] Brace expansion with step
echo {0..20..5}
# Expected: 0 5 10 15 20

# 20.9 [PASSTHROUGH] Tilde expansion in different contexts
echo ~root
# Expected: /root (or root's home dir)

# 20.10 [PASSTHROUGH] Command grouping with semicolons
{ echo a; echo b; echo c; } | tac
# Expected: c b a

# 20.11 [PASSTHROUGH] Here doc with command substitution inside
bash -c 'cat <<EOF
Today is $(date +%A)
User is $(whoami)
EOF'
# Expected: day of week and username

# 20.12 [PASSTHROUGH] Multiple redirections
bash -c '{ echo out; echo err >&2; } > /tmp/reef_out.txt 2> /tmp/reef_err.txt; cat /tmp/reef_out.txt; cat /tmp/reef_err.txt; rm /tmp/reef_out.txt /tmp/reef_err.txt'
# Expected: "out" then "err"

# 20.13 [PASSTHROUGH] Arithmetic in array index
bash -c 'arr=(a b c d e); i=2; echo ${arr[$((i+1))]}'
# Expected: "d"

# 20.14 [PASSTHROUGH] String repetition via printf
printf '%0.s-' {1..40}; echo
# Expected: 40 dashes

# 20.15 [PASSTHROUGH] select menu (interactive — skip in automated tests)
# select opt in "Option 1" "Option 2" "Quit"; do echo $opt; break; done

# 20.16 [PASSTHROUGH] IFS manipulation
bash -c 'IFS=:; read -ra parts <<< "a:b:c:d"; echo "${parts[2]}"'
# Expected: "c"

# 20.17 [PASSTHROUGH] Bash regex capture groups
bash -c '[[ "2024-01-15" =~ ^([0-9]{4})-([0-9]{2})-([0-9]{2})$ ]] && echo "${BASH_REMATCH[1]} ${BASH_REMATCH[2]} ${BASH_REMATCH[3]}"'
# Expected: "2024 01 15"

# 20.18 [PASSTHROUGH] Wait for multiple background jobs
bash -c 'sleep 0.1 & p1=$!; sleep 0.1 & p2=$!; wait $p1 $p2; echo "both done"'
# Expected: "both done"

# 20.19 [PASSTHROUGH] Null command : (colon)
: && echo "null succeeded"
# Expected: "null succeeded"

# 20.20 [PASSTHROUGH] Nested brace and arithmetic expansion
bash -c 'for i in {1..3}; do for j in {1..3}; do echo -n "$((i*j)) "; done; echo; done'
# Expected: multiplication table 1-3


# ============================================================================
# CATEGORY 21: Real-World One-Liners People Actually Paste
# ============================================================================

# 21.1 [KEYWORD] Node version manager setup
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
# Expected: nvm loaded (if installed)

# 21.2 [KEYWORD] Conda init
export PATH="/opt/conda/bin:$PATH"
# Expected: conda added to PATH

# 21.3 [TRANSLATE] Find and delete old files
find /tmp -name "*.tmp" -mtime +7 -exec rm {} \;
# Expected: (runs via find wrapper → fd)

# 21.4 [TRANSLATE] Count lines of code
find . -name '*.py' | xargs wc -l | tail -1
# Expected: total line count

# 21.5 [PASSTHROUGH] SSH with remote command
# ssh user@host 'cd /app && git pull && systemctl restart app'
# (skip in tests — needs remote host)

# 21.6 [TRANSLATE] Docker compose style
export COMPOSE_PROJECT_NAME=myapp && docker compose up -d
# Expected: sets var and runs docker

# 21.7 [PASSTHROUGH] Tar with process substitution
# tar czf - /some/dir | ssh user@host 'cat > backup.tar.gz'
# (skip — needs remote)

# 21.8 [TRANSLATE] Typical installer pattern
curl -fsSL https://example.com/install.sh | bash
# Expected: downloads and runs (piped to bash directly — works as-is)

# 21.9 [PASSTHROUGH] Git hook one-liner
bash -c 'changed_files=$(git diff --cached --name-only); for f in $changed_files; do echo "checking $f"; done'
# Expected: lists staged files

# 21.10 [TRANSLATE] Quick HTTP server
# python3 -m http.server 8080 &
# Expected: server starts in background

# 21.11 [PASSTHROUGH] Process all CSV files
bash -c 'for f in *.csv; do echo "Processing $f"; wc -l "$f"; done'
# Expected: line counts for each CSV

# 21.12 [PASSTHROUGH] System info one-liner
bash -c 'echo "OS: $(uname -s) $(uname -r)"; echo "CPU: $(nproc) cores"; echo "Mem: $(free -h | awk "/^Mem:/{print \$2}")"'
# Expected: OS, CPU, and memory info

# 21.13 [KEYWORD] Pyenv init
export PYENV_ROOT="$HOME/.pyenv"
export PATH="$PYENV_ROOT/bin:$PATH"
eval "$(pyenv init --path)"
# Expected: pyenv loaded (if installed)

# 21.14 [KEYWORD] Rust/cargo setup
export PATH="$HOME/.cargo/bin:$PATH"
# Expected: cargo added to PATH

# 21.15 [PASSTHROUGH] AWS CLI pattern
bash -c 'INSTANCE_ID=$(aws ec2 describe-instances --query "Reservations[0].Instances[0].InstanceId" --output text 2>/dev/null || echo "none"); echo "Instance: $INSTANCE_ID"'
# Expected: "Instance: none" (if no AWS configured)


# ============================================================================
# SUMMARY
# ============================================================================
# Total tests: ~130
#
# Tier 1 (Keyword wrappers): ~15 tests
# Tier 2 (AST translation):  ~55 tests
# Tier 3 (Bash passthrough): ~60 tests
#
# The distribution reflects reality: simple stuff translates,
# complex stuff passes through, and everything works either way.
# ============================================================================
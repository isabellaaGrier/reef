#!/usr/bin/env fish
# tests/edge_cases.fish — Comprehensive reef edge case test suite
#
# Usage: fish tests/edge_cases.fish
#
# This runs each bash command through reef (detect → translate → execute)
# the same way your Enter key hook does, then logs results to a file.
# Show the results file to Claude for analysis.

set -g RESULTS_FILE (pwd)/tests/edge_results.txt
set -g PASS 0
set -g FAIL 0
set -g T2 0
set -g TOTAL 0

# Clean slate
echo "" > $RESULTS_FILE

function log
    echo $argv >> $RESULTS_FILE
end

function run_test
    set -l num $argv[1]
    set -l desc $argv[2]
    set -l bash_cmd $argv[3]
    set -g TOTAL (math $TOTAL + 1)

    # Expected output from bash
    set -l bash_out (bash -c $bash_cmd 2>/dev/null)
    set -l bash_rc $status

    # Detect
    set -l detected "no"
    if reef detect --quick -- $bash_cmd 2>/dev/null
        set detected "yes"
    end

    # Translate
    set -l translated (reef translate -- $bash_cmd 2>/dev/null)
    set -l t_rc $status

    if test $t_rc -eq 0 -a -n "$translated"
        # Tier 1: run translated fish
        set -l oneliner (string join "; " -- $translated)
        set -l fish_out (fish -c "$oneliner" 2>/dev/null)
        set -l fish_rc $status

        set -l bash_str (string join "\n" -- $bash_out)
        set -l fish_str (string join "\n" -- $fish_out)

        if test "$bash_str" = "$fish_str"
            set -g PASS (math $PASS + 1)
            log "PASS #$num [$desc] → T1:translate"
            log "  cmd:  $bash_cmd"
            log "  xlat: $oneliner"
            if test -n "$bash_str"
                log "  out:  $bash_str"
            end
            log ""
        else
            set -g FAIL (math $FAIL + 1)
            log "FAIL #$num [$desc] → T1:translate"
            log "  cmd:  $bash_cmd"
            log "  xlat: $oneliner"
            log "  bash: $bash_str"
            log "  fish: $fish_str"
            log ""
        end
    else
        # Tier 2: bash-exec fallback
        set -l be_out (reef bash-exec -- $bash_cmd 2>&1 1>/dev/null)
        set -l be_rc $status

        set -l bash_str (string join "\n" -- $bash_out)
        set -l be_str (string join "\n" -- $be_out)

        if test "$bash_str" = "$be_str"
            set -g T2 (math $T2 + 1)
            log "T2   #$num [$desc] → bash-exec (detect:$detected)"
            log "  cmd:  $bash_cmd"
            if test -n "$bash_str"
                log "  out:  $bash_str"
            end
            log ""
        else
            set -g T2 (math $T2 + 1)
            log "T2   #$num [$desc] → bash-exec (detect:$detected, output differs)"
            log "  cmd:  $bash_cmd"
            log "  bash: $bash_str"
            log "  bexe: $be_str"
            log ""
        end
    end
end

# ═══════════════════════════════════════════════════════════
#  TESTS
# ═══════════════════════════════════════════════════════════

log "reef Edge Case Test Suite"
log "========================="
log "Date: (date)"
log ""

# --- Quoting & Variables ---
log "--- Quoting & Variables ---"
run_test 1 "single quotes in double quotes" 'echo "it'\''s a '\''test'\''"'
run_test 2 "double quotes in single quotes" "echo 'he said \"hello\"'"
run_test 3 "escaped double quotes" 'echo "she said \"wow\""'
run_test 4 "dollar in single quotes (literal)" "echo '\$HOME is not expanded'"
run_test 5 "empty variable expansion" 'unset XYZZY_REEF; echo "value: ${XYZZY_REEF}"'
run_test 6 "variable with braces" 'echo "home is ${HOME}"'
run_test 7 "multiple exports" 'export REEF_A=1 REEF_B=2 REEF_C=3; echo "$REEF_A $REEF_B $REEF_C"'
run_test 8 "nested command substitution" 'echo "user: $(whoami)"'
run_test 9 "cmd subst in assignment" 'export REEF_USER=$(whoami); echo "$REEF_USER"'
run_test 10 "double nested cmd subst" 'echo "$(echo $(echo deep))"'

# --- Control Flow ---
log "--- Control Flow ---"
run_test 11 "if/elif/else" 'if [ -d /tmp ]; then echo "tmp"; elif [ -d /var ]; then echo "var"; else echo "neither"; fi'
run_test 12 "nested if" 'if [ -d /tmp ]; then if [ -d /var ]; then echo "both"; fi; fi'
run_test 13 "for loop with seq subst" 'for i in $(seq 3); do echo "n:$i"; done'
run_test 14 "for loop with word list" 'for word in alpha beta gamma; do echo "$word"; done'
run_test 15 "for loop with glob" 'for f in /tmp/.X*; do echo "f:$f"; done'
run_test 16 "nested for loops" 'for i in 1 2; do for j in a b; do echo "$i$j"; done; done'
run_test 17 "while loop" 'x=0; while [ $x -lt 3 ]; do echo $x; x=$((x+1)); done'
run_test 18 "case statement" 'case "hello" in h*) echo "starts with h";; *) echo "other";; esac'
run_test 19 "case multi-pattern" 'case "banana" in apple|orange) echo "citrus";; banana|grape) echo "fruit";; esac'
run_test 20 "case bracket pattern" 'case "yes" in [yY]) echo "y";; [nN]) echo "n";; *) echo "other";; esac'

# --- Arithmetic ---
log "--- Arithmetic ---"
run_test 21 "basic arithmetic" 'echo $((2 + 3))'
run_test 22 "arithmetic multiply" 'echo $((2 + 3 * 4))'
run_test 23 "grouped arithmetic" 'echo $(( (5 + 3) * 2 ))'
run_test 24 "modulo" 'echo $((17 % 5))'
run_test 25 "arith with variables" 'x=10; echo $((x + 5))'
run_test 26 "arith comparison" 'echo $(( 5 > 3 ))'
run_test 27 "ternary" 'x=5; echo $(( x > 3 ? 100 : 200 ))'
run_test 28 "nested ternary" 'echo $(( 1 > 2 ? 10 : 3 > 2 ? 20 : 30 ))'
run_test 29 "bitwise AND" 'echo $((255 & 15))'
run_test 30 "bitwise OR" 'echo $((170 | 85))'

# --- Tests & Conditions ---
log "--- Tests & Conditions ---"
run_test 31 "string equality [[ ]]" '[[ "abc" == "abc" ]] && echo "match"'
run_test 32 "string inequality" '[[ "abc" != "xyz" ]] && echo "different"'
run_test 33 "pattern match [[" '[[ "hello.txt" == *.txt ]] && echo "text file"'
run_test 34 "combined && test" '[[ -d /tmp && -d /var ]] && echo "both exist"'
run_test 35 "combined || test" '[[ -d /tmp || -d /nonexistent ]] && echo "one exists"'
run_test 36 "negated test" '[[ ! -f /nonexistent ]] && echo "correct"'
run_test 37 "numeric -gt" '[[ 42 -gt 10 ]] && echo "bigger"'
run_test 38 "string -z empty" '[[ -z "" ]] && echo "empty"'
run_test 39 "string -n nonempty" '[[ -n "hello" ]] && echo "nonempty"'
run_test 40 "regex match" '[[ "error: line 42" =~ [0-9]+ ]] && echo "has number"'

# --- Pipelines & Redirections ---
log "--- Pipelines & Redirections ---"
run_test 41 "simple pipeline" 'echo "hello world" | tr " " "\n" | sort'
run_test 42 "stderr to devnull" 'ls /nonexistent_reef_path 2>/dev/null; echo "done"'
run_test 43 "stdout append" 'echo "test" >> /dev/null; echo "ok"'
run_test 44 "pipe to while read" 'echo -e "a\nb\nc" | while read -r line; do echo "got:$line"; done'
run_test 45 "subshell" '(echo "from subshell")'
run_test 46 "subshell isolation" 'x=original; (x=changed; echo "inner:$x"); echo "outer:$x"'
run_test 47 "command group" '{ echo "a"; echo "b"; echo "c"; }'
run_test 48 "pipeline 3 stages" 'echo "3 1 2" | tr " " "\n" | sort'
run_test 49 "exit code from subshell" '(exit 0); echo "rc:$?"'
run_test 50 "and chain" 'true && echo "yes"'

# --- Parameter Expansion ---
log "--- Parameter Expansion ---"
run_test 51 "default value :-" 'echo ${UNDEFINED_REEF_VAR:-fallback}'
run_test 52 "variable with braces" 'REEF_X=hello; echo ${REEF_X}'
run_test 53 "string length" 'REEF_S=hello; echo ${#REEF_S}'
run_test 54 "prefix removal ##" 'REEF_P="/home/user/file.txt"; echo ${REEF_P##*/}'
run_test 55 "suffix removal %%" 'REEF_F="doc.tar.gz"; echo ${REEF_F%%.*}'
run_test 56 "single prefix #" 'REEF_P="/home/user/file.txt"; echo ${REEF_P#*/}'
run_test 57 "single suffix %" 'REEF_F="doc.tar.gz"; echo ${REEF_F%.*}'
run_test 58 "search replace /" 'REEF_S="hello world"; echo ${REEF_S/world/earth}'
run_test 59 "global replace //" 'REEF_S="aabbaabb"; echo ${REEF_S//aa/XX}'
run_test 60 "uppercase ^^" 'REEF_S="hello"; echo ${REEF_S^^}'

# --- Brace Expansion ---
log "--- Brace Expansion ---"
run_test 61 "brace list" 'echo {apple,banana,cherry}'
run_test 62 "brace with prefix" 'echo file_{a,b,c}.txt'
run_test 63 "numeric range" 'for i in {1..5}; do echo "n:$i"; done'
run_test 64 "range with step" 'for i in {0..20..5}; do echo $i; done'
run_test 65 "alpha range" 'echo {a..e}'
run_test 66 "reverse range" 'echo {5..1}'
run_test 67 "brace in echo" 'echo {1..3}'
run_test 68 "nested braces" 'echo {a,b}{1,2}'

# --- Here Strings ---
log "--- Here Strings ---"
run_test 69 "basic here string" 'cat <<< "hello world"'
run_test 70 "here string with var" 'REEF_V=test; cat <<< "value: $REEF_V"'
run_test 71 "here string to read" 'read -r a b <<< "hello world"; echo "$a $b"'
run_test 72 "here string special chars" "cat <<< 'single quoted here'"

# --- Process Substitution (should T2) ---
log "--- Process Substitution ---"
run_test 73 "diff process subst" 'diff <(echo "hello") <(echo "hello"); echo "rc:$?"'
run_test 74 "paste process subst" 'paste <(seq 3) <(seq 4 6)'
run_test 75 "read from proc subst" 'while read -r line; do echo ">>$line"; done < <(echo -e "x\ny")'

# --- C-style For (should T2) ---
log "--- C-style For Loops ---"
run_test 76 "basic c-for" 'for ((i=0; i<5; i++)); do echo $i; done'
run_test 77 "c-for with step" 'for ((i=0; i<=20; i+=5)); do echo $i; done'
run_test 78 "c-for countdown" 'for ((i=5; i>0; i--)); do echo $i; done'

# --- Arrays (should T2) ---
log "--- Bash Arrays ---"
run_test 79 "indexed array" 'arr=(one two three); echo ${arr[1]}'
run_test 80 "array length" 'arr=(a b c d); echo ${#arr[@]}'
run_test 81 "array iteration" 'arr=(hello world foo); for item in "${arr[@]}"; do echo "$item"; done'
run_test 82 "array slice" 'arr=(a b c d e); echo ${arr[@]:1:3}'
run_test 83 "associative array" 'declare -A m; m[name]="reef"; m[lang]="rust"; echo ${m[name]}'

# --- Substring (should T2) ---
log "--- Substring Extraction ---"
run_test 84 "substring offset" 'REEF_S="hello world"; echo ${REEF_S:6}'
run_test 85 "substring offset+len" 'REEF_S="hello world"; echo ${REEF_S:0:5}'
run_test 86 "lowercase ,," 'REEF_S="HELLO"; echo ${REEF_S,,}'

# --- Mixed / Complex ---
log "--- Mixed & Complex ---"
run_test 87 "semicolon chain" 'x=1; y=2; z=$((x+y)); echo "sum:$z"'
run_test 88 "AND chain" 'true && echo "a" && echo "b"'
run_test 89 "OR chain" 'false || echo "fallback"'
run_test 90 "mixed AND/OR" 'true && echo "yes" || echo "no"'
run_test 91 "false AND/OR" 'false && echo "yes" || echo "no"'
run_test 92 "inline env var" 'HOME=/tmp echo "test"'
run_test 93 "command -v check" 'if command -v fish >/dev/null 2>&1; then echo "found"; else echo "nope"; fi'
run_test 94 "colon noop" ': ; echo "after noop"'
run_test 95 "test -f" 'if test -f /etc/hostname; then echo "exists"; fi'

# --- Real-World Snippets ---
log "--- Real-World Snippets ---"
run_test 96 "basename in subst" 'echo $(basename /home/user/file.txt)'
run_test 97 "dirname in subst" 'echo $(dirname /home/user/file.txt)'
run_test 98 "wc pipeline" 'echo "one two three" | wc -w'
run_test 99 "tr pipeline" 'echo "HELLO" | tr A-Z a-z'
run_test 100 "cut field" 'echo "a:b:c" | cut -d: -f2'

# --- Exotic / Stress ---
log "--- Exotic & Stress ---"
run_test 101 "deeply nested subst" 'echo $(echo $(echo $(echo deep)))'
run_test 102 "empty if body" 'if true; then :; fi; echo "ok"'
run_test 103 "multi-export with path" 'export REEF_T1=one REEF_T2=two; echo "$REEF_T1 $REEF_T2"'
run_test 104 "for with cmd subst words" 'for f in $(echo a b c); do echo "item:$f"; done'
run_test 105 "declare -x" 'declare -x REEF_DX=declared; echo $REEF_DX'
run_test 106 "unset variable" 'export REEF_DEL=exists; unset REEF_DEL; echo "gone:${REEF_DEL:-yes}"'
run_test 107 "local in function context" 'f() { local x=42; echo $x; }; f'
run_test 108 "function keyword" 'function reef_fn { echo "from fn"; }; reef_fn'
run_test 109 "eval simple" 'eval echo hello'
run_test 110 "trap EXIT" 'trap "echo cleanup" EXIT; echo "running"'
run_test 111 "read from pipe" 'echo "hello world" | { read -r a b; echo "$a|$b"; }'
run_test 112 "while read multi" 'printf "1 a\n2 b\n3 c\n" | while read -r num letter; do echo "$num=$letter"; done'
run_test 113 "nested case in if" 'x=foo; if true; then case "$x" in foo) echo "matched";; esac; fi'
run_test 114 "arithmetic in condition" 'if [ $((2+2)) -eq 4 ]; then echo "math"; fi'
run_test 115 "redirect stderr only" 'echo "visible" 2>/dev/null'

# ═══════════════════════════════════════════════════════════
#  Summary
# ═══════════════════════════════════════════════════════════
set -l total_pass (math $PASS + $T2)
log "═══════════════════════════════════════════"
log "RESULTS: $PASS T1-pass, $T2 T2-fallback, $FAIL failed / $TOTAL total"
log "═══════════════════════════════════════════"

echo ""
echo "  reef Edge Case Test Suite"
echo "  ═══════════════════════════════════════════"
echo ""
echo "  T1 pass:    $PASS"
echo "  T2 fallback: $T2"
echo "  FAIL:        $FAIL"
echo "  Total:       $TOTAL"
echo ""
echo "  Results written to: $RESULTS_FILE"
echo "  Paste that file to Claude for analysis."
echo ""

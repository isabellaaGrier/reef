function source --description "reef: smart source — detects bash scripts and uses passthrough"
    # No arguments = piped stdin (e.g. `cmd | source`). Pass through to builtin.
    if test (count $argv) -eq 0
        builtin source
        return $status
    end

    set -l file $argv[1]
    set -l remaining_args $argv[2..]

    # If the file doesn't exist, try the original builtin
    if not test -f "$file"
        builtin source $argv
        return $status
    end

    # .fish files are always native fish — skip all detection
    if string match -qr '\.fish$' -- $file
        builtin source $argv
        return $status
    end

    # Detect if this is a bash script
    set -l is_bash false

    # Check file extension
    if string match -qr '\.(sh|bash|bashrc|bash_profile|profile)$' -- $file
        set is_bash true
    end

    # Check shebang
    if test "$is_bash" = false
        set -l first_line (head -1 "$file" 2>/dev/null)
        if string match -qr '^#!.*(bash|/sh)' -- "$first_line"
            set is_bash true
        end
    end

    # Check for bash-only syntax (only for extensionless/ambiguous files)
    if test "$is_bash" = false
        set -l sample (head -50 "$file" 2>/dev/null)
        for line in $sample
            # Only match syntax that is unambiguously bash, not valid fish:
            #   then/fi/done/esac are bash control flow keywords
            #   ${...} parameter expansion (fish uses $var or {$var})
            #   export VAR=val with = (fish export.fish handles this but native fish doesn't)
            if string match -qr '(; then$|; then;|; do$|; do;|\bfi$|\bfi;|\bdone$|\bdone;|\besac$|\$\{|^export [A-Za-z_]+=)' -- $line
                set is_bash true
                break
            end
        end
    end

    if test "$is_bash" = true
        # Source through bash and capture env changes
        set -l bash_cmd "source '$file'"
        if test (count $remaining_args) -gt 0
            set bash_cmd "$bash_cmd $remaining_args"
        end

        # Use reef bash-exec if available, otherwise inline bash passthrough
        if command -q reef
            reef bash-exec --env-diff -- $bash_cmd | builtin source
        else
            # Inline fallback: source in bash, dump env, parse in fish
            set -l sentinel "__REEF_ENV_MARKER__"
            set -l output (bash -c "$bash_cmd 2>/dev/null; echo $sentinel; env; echo $sentinel; pwd" 2>/dev/null)

            set -l in_env false
            set -l in_cwd false
            for line in $output
                if test "$line" = "$sentinel"
                    if test "$in_env" = false
                        set in_env true
                    else
                        set in_env false
                        set in_cwd true
                    end
                    continue
                end

                if test "$in_cwd" = true
                    cd "$line"
                    break
                end

                if test "$in_env" = true
                    if string match -qr '^([^=]+)=(.*)$' -- $line
                        set -l varname (string replace -r '=.*' '' -- $line)
                        set -l varval (string replace -r '^[^=]+=' '' -- $line)
                        switch $varname
                            case BASH BASHOPTS BASH_VERSINFO BASH_VERSION SHELLOPTS SHLVL _ PWD OLDPWD
                                continue
                        end
                        if string match -qr 'PATH$' -- $varname
                            set -gx $varname (string split : -- $varval)
                        else
                            set -gx $varname $varval
                        end
                    end
                end
            end
        end
        return 0
    else
        # Not bash — use the builtin source
        builtin source $argv
        return $status
    end
end

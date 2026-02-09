function export --description "reef: bash export → fish set -gx"
    if test (count $argv) -eq 0
        # Bare 'export' with no args — print all exported vars (bash compat)
        set -gx
        return 0
    end

    for arg in $argv
        # Handle flags: export -n VAR (remove export attribute — fish can't do this, skip)
        if test "$arg" = -n
            continue
        end

        if string match -qr '^([^=]+)=(.*)$' -- $arg
            # export VAR=value
            set -l varname (string replace -r '=.*' '' -- $arg)
            set -l varval (string replace -r '^[^=]+=' '' -- $arg)

            # Handle PATH-like colon-separated values — split on : for fish
            if string match -qr 'PATH$' -- $varname
                set -gx $varname (string split : -- $varval)
            else
                set -gx $varname $varval
            end
        else
            # export VAR (no value) — mark existing var as exported
            if set -q $arg
                set -gx $arg $$arg
            else
                set -gx $arg ""
            end
        end
    end
end

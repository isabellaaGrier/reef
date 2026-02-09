function local --description "reef: bash local → fish set -l"
    for arg in $argv
        # Skip flags
        if string match -qr '^-' -- $arg
            continue
        end

        if string match -qr '^([^=]+)=(.*)$' -- $arg
            set -l varname (string replace -r '=.*' '' -- $arg)
            set -l varval (string replace -r '^[^=]+=' '' -- $arg)
            set -l $varname $varval
        else
            # local VAR (no value)
            set -l $arg ""
        end
    end
end

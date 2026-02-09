function readonly --description "reef: bash readonly → fish set -g (no true readonly in fish)"
    for arg in $argv
        # Skip flags
        if string match -qr '^-' -- $arg
            continue
        end

        if string match -qr '^([^=]+)=(.*)$' -- $arg
            set -l varname (string replace -r '=.*' '' -- $arg)
            set -l varval (string replace -r '^[^=]+=' '' -- $arg)
            set -g $varname $varval
        else
            # readonly VAR (no value) — just ensure it's global
            if set -q $arg
                set -g $arg $$arg
            end
        end
    end
end

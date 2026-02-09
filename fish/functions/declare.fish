function declare --description "reef: bash declare → fish set"
    set -l scope -g
    set -l is_export false
    set -l remaining_args

    # Parse flags
    for arg in $argv
        switch $arg
            case -x
                # declare -x → export (global + exported)
                set is_export true
            case -g
                # declare -g → global
                set scope -g
            case -l
                # declare -l → not fish local, bash lowercase. Ignore.
            case -i -r -a -A
                # -i (integer), -r (readonly), -a (array), -A (associative array)
                # Fish doesn't have these — best effort, ignore the flag
            case '-*'
                # Unknown flag — ignore
            case '*'
                set -a remaining_args $arg
        end
    end

    if test "$is_export" = true
        set scope -gx
    end

    for arg in $remaining_args
        if string match -qr '^([^=]+)=(.*)$' -- $arg
            set -l varname (string replace -r '=.*' '' -- $arg)
            set -l varval (string replace -r '^[^=]+=' '' -- $arg)
            set $scope $varname $varval
        else
            # declare VAR (no value)
            if not set -q $arg
                set $scope $arg ""
            end
        end
    end
end

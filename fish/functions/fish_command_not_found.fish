function fish_command_not_found --on-event fish_command_not_found
    # Recursion guard — prevent infinite loop when reef itself triggers command_not_found
    if set -q __reef_cnf_active
        __fish_default_command_not_found_handler $argv[1]
        return
    end

    set -l cmd (string join " " -- $argv)

    # If reef binary is available, try translation first
    if command -q reef
        set -g __reef_cnf_active true

        # Try to translate the bash command to fish
        set -l translated (reef translate -- $cmd 2>/dev/null)
        if test $status -eq 0; and test -n "$translated"
            set -l oneliner (string join "; " -- $translated)
            eval $oneliner
            set -l ret $status
            set -e __reef_cnf_active
            return $ret
        end

        # Translation failed — try bash passthrough
        reef bash-exec -- $cmd | builtin source
        set -l exit_code $pipestatus[1]

        set -e __reef_cnf_active

        if test $exit_code -ne 127
            return $exit_code
        end
    end

    # Neither reef translation nor bash could handle it — show the real error
    __fish_default_command_not_found_handler $argv[1]
end

function unset --description "reef: bash unset → fish set -e"
    for arg in $argv
        # Skip flags like -v (variable) and -f (function) — fish doesn't distinguish
        if string match -qr '^-' -- $arg
            continue
        end
        set -e $arg
    end
end

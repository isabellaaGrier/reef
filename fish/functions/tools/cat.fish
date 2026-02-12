function cat --wraps=bat --description "cat → bat"
    if not command -q bat
        command cat $argv
        return $status
    end

    # Pass through --version/--help to the real tool
    for a in $argv
        if test "$a" = "--version"; or test "$a" = "--help"
            command cat $argv
            return $status
        end
    end

    command bat $argv 2>/dev/null; or command cat $argv
end

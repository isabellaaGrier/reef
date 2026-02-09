function cat --wraps=bat --description "cat → bat"
    if not command -q bat
        command cat $argv
        return $status
    end

    command bat $argv
end

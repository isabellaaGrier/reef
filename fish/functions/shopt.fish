function shopt --description "reef: bash shopt → no-op (fish has no equivalent)"
    # shopt controls bash-specific shell options that have no fish equivalent.
    # Silently succeed for common options, warn on unknown ones.
    #
    # Common shopt options seen in the wild:
    #   -s extglob       (extended globbing — fish has wildcards built-in)
    #   -s dotglob       (glob matches dotfiles — fish doesn't support this natively)
    #   -s nullglob      (glob with no matches expands to nothing — fish default)
    #   -s globstar      (** recursive glob — fish supports this natively)
    #   -s nocaseglob    (case-insensitive glob — not in fish)
    #
    # All are no-ops since fish either already has the behavior or can't replicate it.
    return 0
end

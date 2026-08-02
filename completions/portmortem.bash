_portmortem() {
    local cur prev
    _init_completion || return

    case "$prev" in
        --completion)
            COMPREPLY=($(compgen -W "bash zsh fish nu" -- "$cur"))
            return
            ;;
    esac

    if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "-a --all-ports -j --json -k --kill --completion -h --help -V --version" -- "$cur"))
        return
    fi

    # Belegte Ports vorschlagen
    local ports
    ports=$(ss -tlunp 2>/dev/null | awk 'NR>1 {split($5,a,":"); print a[length(a)]}' | sort -un)
    COMPREPLY=($(compgen -W "$ports" -- "$cur"))
}

complete -F _portmortem portmortem
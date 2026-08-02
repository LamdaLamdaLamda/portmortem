complete -c portmortem -s a -l all-ports  -d "Show all ports held by the same process"
complete -c portmortem -s j -l json       -d "Output as JSON"
complete -c portmortem -s k -l kill       -d "Kill the binding process"
complete -c portmortem -s h -l help       -d "Show help"
complete -c portmortem -s V -l version    -d "Show version"

complete -c portmortem -l completion -d "Install shell completion" -x \
    -a "bash\t'Bash' zsh\t'Zsh' fish\t'Fish' nu\t'Nushell'"

# Belegte Ports vorschlagen
complete -c portmortem -x \
    -a "(ss -tlunp 2>/dev/null | awk 'NR>1 {split(\$5,a,\":\"); print a[length(a)]}' | sort -un)" \
    -d "Port"
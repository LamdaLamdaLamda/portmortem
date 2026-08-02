export extern portmortem [
    ...ports: int               # Port number(s) to investigate
    --all-ports(-a)             # Show all ports held by the same process(es)
    --json(-j)                  # Output as JSON (for scripting)
    --kill(-k)                  # Kill the binding process
    --completion: string@"nu-complete-shells"  # Install shell completion
    --help(-h)                  # Show help
    --version(-V)               # Show version
]

def "nu-complete-shells" [] {
    ["bash", "zsh", "fish", "nu"]
}
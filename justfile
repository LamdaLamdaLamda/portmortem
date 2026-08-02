bash_dir := "/etc/bash_completion.d"
zsh_dir  := "/usr/local/share/zsh/site-functions"
fish_dir := env_var_or_default("HOME", "") / ".config/fish/completions"
nu_dir   := env_var_or_default("HOME", "") / ".config/nushell/completions"

check:
    cargo fmt
    cargo clippy
    cargo audit
    cargo deny check
    cargo test

setup:
    cargo install cargo-audit
    cargo install cargo-deny

format:
    cargo fmt

deploy shell: check
    cargo build --release
    cargo install --path .
    just completions-{{shell}}
    @echo "✓ portmortem deployed"

completions-bash:
    sudo cp completions/portmortem.bash {{bash_dir}}/portmortem
    @echo "✓ bash → {{bash_dir}}/portmortem"

completions-zsh:
    sudo cp completions/_portmortem {{zsh_dir}}/_portmortem
    @echo "✓ zsh  → {{zsh_dir}}/_portmortem"

completions-fish:
    mkdir -p {{fish_dir}}
    cp completions/portmortem.fish {{fish_dir}}/portmortem.fish
    @echo "✓ fish → {{fish_dir}}/portmortem.fish"

completions-nu:
    mkdir -p {{nu_dir}}
    cp completions/portmortem.nu {{nu_dir}}/portmortem.nu
    @echo "✓ nu   → {{nu_dir}}/portmortem.nu"
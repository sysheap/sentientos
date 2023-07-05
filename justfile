build: _build-no-clippy
    cargo clippy

clean:
    cargo clean

_build-no-clippy:
    cargo build

runCommand := "cargo run"
debugCommand := "cargo run -- -s -S"

run: _build-no-clippy
    {{runCommand}}

vscode: _build-no-clippy
    {{debugCommand}}

debug: _build-no-clippy
    tmux new-session -d '{{debugCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/yaros -ex "target remote :1234" -ex "c"' \; attach

debugf FUNC: _build-no-clippy
    tmux new-session -d '{{debugCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/yaros -ex "target remote :1234" -ex "break {{FUNC}}" -ex "c"' \; attach
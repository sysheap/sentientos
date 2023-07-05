build:
    cargo clippy
    cargo build

clean:
    cargo clean

debugCommand := "cargo run -- -s -S"

run: build
    cargo run

test: build
    cargo test

vscode: build
    {{debugCommand}}

debug: build
    tmux new-session -d '{{debugCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/yaros -ex "target remote :1234" -ex "c"' \; attach

debugf FUNC: build
    tmux new-session -d '{{debugCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/yaros -ex "target remote :1234" -ex "break {{FUNC}}" -ex "c"' \; attach
build:
    cd userspace && cargo build    
    cargo build

clippy:
    cd userspace && cargo clippy
    cargo clippy

clean:
    cargo clean

debugCommand := "cargo run -- -s -S"

run: build
    cargo run

test: build clippy
    cargo test

run-vscode: build
    cargo run -- -s -S && echo "DONE"

test-vscode: build
    cargo test -- -s -S && echo "DONE"

debug: build
    tmux new-session -d '{{debugCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/yaros -ex "target remote :1234" -ex "c"' \; attach

debugf FUNC: build
    tmux new-session -d '{{debugCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/yaros -ex "target remote :1234" -ex "break {{FUNC}}" -ex "c"' \; attach
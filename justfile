build-debug:
    cd userspace && cargo build    
    cargo build

build-release:
    cd userspace && cargo build --release
    cargo build --release

clippy:
    cd userspace && cargo build && cargo clippy
    cargo clippy

clean:
    cargo clean

debugCommand := "cargo run -- -s -S"

run: build-release
    cargo run --release

run-debug: build-debug
    cargo run

test: build-release
    cargo test --release

run-vscode: build-debug
    cargo run -- -s -S && echo "DONE"

test-vscode: build-debug
    cargo test -- -s -S && echo "DONE"

debug: build-debug
    tmux new-session -d '{{debugCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/kernel -ex "target remote :1234" -ex "c"' \; attach

debugf FUNC: build-debug
    tmux new-session -d '{{debugCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/kernel -ex "target remote :1234" -ex "break {{FUNC}}" -ex "c"' \; attach

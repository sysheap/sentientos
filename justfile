build-debug:
    cd userspace && cargo build    
    cargo build

build-release:
    cd userspace && cargo build --release
    cargo build --release

clippy:
    cd userspace && cargo build && cargo clippy -- -D warnings
    cargo clippy -- -D warnings

clean:
    cargo clean

debugCommand := "cargo run -- -s -S"
debugReleaseCommand := "cargo run --release -- -s -S"

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
    tmux new-session -d '{{debugCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/kernel -ex "target remote :1234" -ex "hbreak {{FUNC}}" -ex "c"' \; attach

debug-release: build-release
    tmux new-session -d '{{debugReleaseCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/kernel -ex "target remote :1234" -ex "c"' \; attach

debug-releasef FUNC: build-release
    tmux new-session -d '{{debugReleaseCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/kernel -ex "target remote :1234" -ex "hbreak {{FUNC}}" -ex "c"' \; attach

disassm-release: build-release
    riscv64-unknown-elf-objdump -d target/riscv64gc-unknown-none-elf/release/kernel | less
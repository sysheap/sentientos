build-release:
    cargo build --release

build-debug:
    cargo build

clippy:
    cd src/userspace && cargo clippy -- -D warnings
    cargo clippy -- -D warnings

clean:
    cargo clean

debugCommand := "cargo run -- -s -S"
debugReleaseCommand := "cargo run --release -- -s -S"

run:
    cargo run --release

run-debug:
    cargo run

test:
    cargo test --release --features commonlib-test

run-vscode:
    cargo run -- -s -S && echo "DONE"

test-vscode:
    cargo test -- -s -S && echo "DONE"

debug:
    tmux new-session -d '{{debugCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/kernel -ex "target remote :1234"' \; attach

debugf FUNC:
    tmux new-session -d '{{debugCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/kernel -ex "target remote :1234" -ex "hbreak {{FUNC}}"' \; attach

debug-release:
    tmux new-session -d '{{debugReleaseCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/release/kernel -ex "target remote :1234"' \; attach

debug-releasef FUNC:
    tmux new-session -d '{{debugReleaseCommand}}' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/release/kernel -ex "target remote :1234" -ex "hbreak {{FUNC}}"' \; attach

disassm-release:
    riscv64-unknown-elf-objdump -d target/riscv64gc-unknown-none-elf/release/kernel | less
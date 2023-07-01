build: _build-no-clippy
    cargo clippy

clean:
    cargo clean

_build-no-clippy:
    cargo build

run: _build-no-clippy
    qemu-system-riscv64 \
        -machine virt -cpu rv64 \
        -smp 1 \
        -m 128M \
        -nographic -serial mon:stdio \
        -bios none \
        -kernel ./target/riscv64gc-unknown-none-elf/debug/yaros

vscode: _build-no-clippy
    qemu-system-riscv64 \
        -machine virt -cpu rv64 \
        -smp 1 \
        -m 128M \
        -nographic -serial mon:stdio \
        -bios none \
        -kernel ./target/riscv64gc-unknown-none-elf/debug/yaros \
        -s -S

debug: _build-no-clippy
    tmux new-session -d 'qemu-system-riscv64 \
        -machine virt -cpu rv64 \
        -smp 1 \
        -m 128M \
        -nographic -serial mon:stdio \
        -bios none \
        -kernel ./target/riscv64gc-unknown-none-elf/debug/yaros \
        -s -S' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/yaros -ex "target remote :1234" -ex "c"' \; attach

debugf FUNC: _build-no-clippy
    tmux new-session -d 'qemu-system-riscv64 \
        -machine virt -cpu rv64 \
        -smp 1 \
        -m 128M \
        -nographic -serial mon:stdio \
        -bios none \
        -kernel ./target/riscv64gc-unknown-none-elf/debug/yaros \
        -s -S' \; split-window -h 'gdb-multiarch $(pwd)/target/riscv64gc-unknown-none-elf/debug/yaros -ex "target remote :1234" -ex "break {{FUNC}}" -ex "c"' \; attach
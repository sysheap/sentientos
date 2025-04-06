hello_posix: hello_posix.c ../../../../target/riscv64gc-unknown-none-elf/release/libuserspace.a
	riscv64-none-elf-gcc $^ -o $@ -ffreestanding -lgcc -nostdlib -T ../../../userspace.ld -I../include -Wall -Wpedantic -O2


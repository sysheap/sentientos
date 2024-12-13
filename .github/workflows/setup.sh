#!/bin/bash
set -xe

cd "$(dirname "$0")"

TOOLCHAIN=$(cat ../../rust-toolchain | grep channel | awk '{print $3}' | tr -d '"')

sudo apt-get install -y qemu-system-riscv64 binutils-riscv64-linux-gnu curl
sudo rm -rf /var/lib/apt/lists/*

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y --default-toolchain="$TOOLCHAIN" --profile minimal --component clippy --component rustfmt --component miri --component rust-src --target riscv64gc-unknown-none-elf

source "$HOME/.cargo/env"

cargo install just cargo-nextest --locked 

# Download dependencies into cache
just fetch-deps

# Prepare the sysroot for miri such that it is cached as well
cd /tmp
cargo +"$TOOLCHAIN" miri test --target riscv64gc-unknown-linux-gnu || true

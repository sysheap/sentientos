#!/usr/bin/env bash
# Setup SentientOS development environment without Nix.
# Installs pre-compiled binaries only - nothing is compiled from source.
#
# Tested on: Ubuntu 24.04 (x86_64)
# Intended for: Claude Code web sessions and other non-Nix environments.
#
# Prerequisites: curl, sudo, rustup (with nightly toolchain from rust-toolchain file)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

info() { echo "==> $*"; }
ok()   { echo "  OK: $*"; }
skip() { echo "  SKIP: $*  (already installed)"; }

# --- 1. Rust toolchain ---
info "Checking Rust toolchain"
if rustup show active-toolchain 2>/dev/null | grep -q "nightly-2026"; then
    skip "Rust nightly toolchain"
else
    info "Installing Rust toolchain from rust-toolchain file"
    rustup show
fi

if ! rustup component list --installed 2>/dev/null | grep -q clippy; then
    rustup component add clippy
    ok "clippy"
else
    skip "clippy"
fi

# --- 2. System packages (apt) ---
info "Installing system packages via apt"

APT_PACKAGES=(
    qemu-system-misc          # QEMU with riscv64 support
    ipxe-qemu                 # VirtIO ROM files (efi-virtio.rom)
    gcc-riscv64-linux-gnu     # RISC-V cross-compiler (used as linker driver)
    binutils-riscv64-linux-gnu # RISC-V binutils (nm, objcopy, objdump, addr2line)
    linux-libc-dev-riscv64-cross # RISC-V Linux kernel headers (for bindgen)
)

MISSING=()
for pkg in "${APT_PACKAGES[@]}"; do
    if ! dpkg -s "$pkg" &>/dev/null; then
        MISSING+=("$pkg")
    fi
done

if [ ${#MISSING[@]} -gt 0 ]; then
    sudo apt-get update -qq
    sudo apt-get install -y --no-install-recommends "${MISSING[@]}"
    ok "apt packages: ${MISSING[*]}"
else
    skip "all apt packages"
fi

# Ensure efi-virtio.rom is findable by QEMU
if [ ! -f /usr/share/qemu/efi-virtio.rom ]; then
    ROM=$(find /usr -name "efi-virtio.rom" 2>/dev/null | head -1)
    if [ -n "$ROM" ]; then
        sudo ln -sf "$ROM" /usr/share/qemu/efi-virtio.rom
        ok "symlinked efi-virtio.rom"
    fi
fi

# --- 3. Cross-tool symlinks ---
# Project expects riscv64-unknown-linux-musl-* naming (Nix convention).
# Ubuntu provides riscv64-linux-gnu-* which work identically for our use cases:
# - binutils (nm, objcopy, objdump, addr2line): architecture-agnostic ELF tools
# - gcc: used only as linker driver; Rust provides musl CRT objects
info "Creating cross-tool symlinks"
for tool in nm objcopy objdump addr2line ar as gcc; do
    src="/usr/bin/riscv64-linux-gnu-$tool"
    dst="/usr/local/bin/riscv64-unknown-linux-musl-$tool"
    if [ -f "$src" ] && [ ! -e "$dst" ]; then
        sudo ln -sf "$src" "$dst"
    fi
done
ok "riscv64-unknown-linux-musl-* symlinks"

# --- 4. just (command runner) ---
info "Installing just"
if command -v just &>/dev/null; then
    skip "just $(just --version)"
else
    JUST_VERSION="1.46.0"
    curl -sL "https://github.com/casey/just/releases/download/${JUST_VERSION}/just-${JUST_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
        | sudo tar -xz -C /usr/local/bin just
    ok "just $(just --version)"
fi

# --- 5. cargo-nextest ---
info "Installing cargo-nextest"
if command -v cargo-nextest &>/dev/null; then
    skip "cargo-nextest $(cargo-nextest --version 2>/dev/null | head -1)"
else
    NEXTEST_VERSION="0.9.127"
    curl -sL "https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-${NEXTEST_VERSION}/cargo-nextest-${NEXTEST_VERSION}-x86_64-unknown-linux-gnu.tar.gz" \
        | sudo tar -xz -C /usr/local/bin cargo-nextest
    ok "cargo-nextest $(cargo-nextest --version 2>/dev/null | head -1)"
fi

# --- 6. Linux headers symlink ---
info "Setting up Linux headers for bindgen"
HEADERS_LINK="$SCRIPT_DIR/headers/linux_headers"
if [ -L "$HEADERS_LINK" ] || [ -d "$HEADERS_LINK" ]; then
    skip "headers/linux_headers"
else
    ln -sf /usr/riscv64-linux-gnu "$HEADERS_LINK"
    ok "headers/linux_headers -> /usr/riscv64-linux-gnu"
fi

# --- 7. Userspace directories ---
info "Creating userspace directories"
mkdir -p "$SCRIPT_DIR/kernel/compiled_userspace"
mkdir -p "$SCRIPT_DIR/kernel/compiled_userspace_nix"
ok "kernel/compiled_userspace{,_nix}"

# --- 8. LIBCLANG_PATH via .env ---
LIBCLANG_DIR="/usr/lib/llvm-18/lib"
if [ ! -d "$LIBCLANG_DIR" ]; then
    for v in 20 19 17 16 15 14; do
        if [ -d "/usr/lib/llvm-$v/lib" ]; then
            LIBCLANG_DIR="/usr/lib/llvm-$v/lib"
            break
        fi
    done
fi

ENV_FILE="$SCRIPT_DIR/.env"
if [ ! -f "$ENV_FILE" ]; then
    info "Creating .env with LIBCLANG_PATH"
    echo "LIBCLANG_PATH=$LIBCLANG_DIR" > "$ENV_FILE"
    ok ".env"
else
    skip ".env"
fi

# --- Summary ---
echo ""
echo "========================================="
echo "  Development environment ready!"
echo "========================================="
echo ""
echo "Commands: just build, just run, just test"
echo ""
echo "Note: coreutils tests (true, false, echo) will fail"
echo "without Nix. All other tests pass."
echo ""

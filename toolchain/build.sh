#!/usr/bin/env bash

set -xeuo pipefail

cd "$(dirname "$0")"

BINUTILS_VERSION="2.44"
GCC_VERSION="15.1.0"
TARGET="riscv64-unknown-sentientos"
SYSROOT="$(pwd)/sysroot"

clone_repositories () {
  if [ ! -d "binutils" ]; then
    git clone --depth 1 --branch binutils-${BINUTILS_VERSION//./_} git://sourceware.org/git/binutils-gdb.git binutils
    git -C binutils am ../0001-binutils-Add-support-for-sentientos.patch
  fi

  if [ ! -d "gcc" ]; then
    git clone --depth 1 --branch releases/gcc-${GCC_VERSION} git://gcc.gnu.org/git/gcc.git
    git -C gcc am ../0001-gcc-Add-support-for-sentientos.patch
  fi  
}

build_binutils () {
  echo "Build binutils"

  mkdir -p binutils-build
  cd binutils-build

  ../binutils/configure \
    --prefix="$(pwd)/../binutils-bin" \
    --target="$TARGET" \
    --with-sysroot="$SYSROOT" \
    --disable-gdb \
    --disable-nls \
    --disable-werror

  make -j$(nproc)
  make install

  cd ../
}

build_gcc () {
  echo "Build gcc"

  # The $PREFIX/bin dir _must_ be in the PATH. We did that above.
  which -- $TARGET-as || (echo $TARGET-as is not in the PATH && false)

  mkdir -p gcc-build
  cd gcc-build

  ../gcc/configure \
    --prefix="$(pwd)/../gcc-bin" \
    --target="$TARGET" \
    --with-sysroot="$SYSROOT" \
    --disable-nls \
    --enable-languages=c \
    --with-as="$(pwd)/../binutils-bin/bin/${TARGET}-as" \
    --with-ld="$(pwd)/../binutils-bin/bin/${TARGET}-ld"

  make -j$(nproc) all-gcc all-target-libgcc
  make install-gcc install-target-libgcc

  cd ../
}

clone_repositories
build_binutils
build_gcc

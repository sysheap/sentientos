#!/usr/bin/env bash
set -xeuo pipefail

cd "$(dirname "$0")"

BINUTILS_VERSION="2.44"
GCC_VERSION="15.1.0"
NEWLIB_VERSION="4.5.0"
TARGET="riscv64-none-sentientos"
SYSROOT="$(pwd)/sysroot"

# We don't need debug information in the toolchain
export CFLAGS="-g0 -O2 -mtune=native"
export CXXFLAGS="-g0 -O2 -mtune=native"

clone_repositories () {
  if [ ! -d "binutils" ]; then
    git clone --depth 1 --branch binutils-${BINUTILS_VERSION//./_} git://sourceware.org/git/binutils-gdb.git binutils
    git -C binutils am ../0001-binutils-Add-support-for-sentientos.patch
  fi

  if [ ! -d "gcc" ]; then
    git clone --depth 1 --branch releases/gcc-${GCC_VERSION} git://gcc.gnu.org/git/gcc.git
    git -C gcc am ../0001-gcc-Add-support-for-sentientos.patch
  fi  

  if [ ! -d "newlib-cygwin" ]; then
    git clone --depth 1 --branch newlib-${NEWLIB_VERSION} git://sourceware.org/git/newlib-cygwin.git
    git -C newlib-cygwin am ../0001-newlib-Add-sentientos-as-target.patch
  fi  
}

apply_autoconf() {
    cd gcc/libstdc++-v3
    autoconf
    cd ../..
  
    cd newlib-cygwin/newlib
    autoreconf
    cd ../
    autoreconf
    cd ../
}

build_binutils () {
  echo "Build binutils"

  if [ ! -d binutils-build ]; then
    mkdir -p binutils-build
    cd binutils-build

    ../binutils/configure \
      --prefix="$(pwd)/../bin" \
      --target="$TARGET" \
      --with-sysroot="$SYSROOT" \
      --disable-werror \
      --disable-gdb

    cd ../

  fi

  cd binutils-build

  make -j"$(nproc)"
  make install

  cd ../
}

build_gcc () {
  echo "Build gcc"

  # The $PREFIX/bin dir _must_ be in the PATH. We did that above.
  which -- $TARGET-as || (echo $TARGET-as is not in the PATH && false)

  echo "Copy headers from newlibc"
  mkdir -p sysroot/usr/include
  cp -r newlib-cygwin/newlib/libc/include/ sysroot/usr/

  if [ ! -d gcc-build ]; then
    mkdir -p gcc-build
    cd gcc-build

    ../gcc/configure \
      --prefix="$(pwd)/../bin" \
      --target="$TARGET" \
      --with-sysroot="$SYSROOT" \
  		--enable-languages=c,c++ \
  		--with-newlib \
  		--with-abi=lp64d \
      --with-arch=rv64gc \
      --with-as="$(pwd)/../bin/bin/${TARGET}-as" \
      --with-ld="$(pwd)/../bin/bin/${TARGET}-ld"

    cd ../
  fi

  cd gcc-build

  make -j$(nproc) all-gcc all-target-libgcc
  make install-gcc install-target-libgcc

  cd ../
}

build_newlib () {
  echo "Build newlib"

  if [ ! -d newlib-build ]; then
    mkdir -p newlib-build
    cd newlib-build

    ../newlib-cygwin/configure \
      --prefix="$SYSROOT" \
      --target="$TARGET"

    cd ../

  fi

  cd newlib-build

  make -j$(nproc) all
  make install

  cd ../

  cp -r sysroot/riscv64-none-sentientos/* sysroot/usr/
  rm -rf sysroot/riscv64-none-sentientos
}

build_libstdcpp () {
  echo "Build libstdc++"  

  cd gcc-build

  make all-target-libstdc++-v3 -j$(nproc)
  make install-target-libstdc++-v3

  cd ../
}

build_and_ln_libuserspace () {
  cargo build --release --lib --manifest-path ../userspace/Cargo.toml
  ln -sf ../../../../target/riscv64gc-unknown-none-elf/release/libuserspace.a sysroot/usr/lib/
}

# clone_repositories
# apply_autoconf
# build_binutils
# build_gcc
build_newlib
# build_and_ln_libuserspace
# build_libstdcpp

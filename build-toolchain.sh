#!/bin/bash

cd "$(dirname "$0")"

rm -rf riscv-gnu-toolchain

set -xe

git clone https://github.com/riscv-collab/riscv-gnu-toolchain
cd riscv-gnu-toolchain

git checkout 2023.04.21

./configure --prefix=$(pwd)/build
make -j$(nproc)
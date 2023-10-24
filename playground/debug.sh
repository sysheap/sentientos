#!/bin/bash
set -x

cd "$(dirname "$0")"

gdb-multiarch playground -ex "target remote :1234" "$@"


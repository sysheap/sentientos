#!/bin/bash

set -e

cd "$(dirname "$0")"

QEMU_CMD="qemu-system-riscv64 \
    -machine virt \
    -cpu rv64 \
    -smp 1 \
    -m 128M \
    -nographic \
    -serial mon:stdio"

# Process options
while [[ $# -gt 0 ]]; do
    case "$1" in
        --gdb)
            QEMU_CMD+=" -s"
            shift
            ;;
        --log)
            QEMU_CMD+=" -d guest_errors,cpu_reset,unimp,int -D /tmp/yaros.log"
            shift
            ;;
        --net)
            QEMU_CMD+=" -netdev user,id=netdev1,hostfwd=udp::1234-:1234 -device virtio-net-pci,netdev=netdev1"
            shift
            ;;
        --capture)
            QEMU_CMD+=" -object filter-dump,id=f1,netdev=netdev1,file=network.pcap "
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS] <KERNEL_PATH>"
            echo ""
            echo "Options:"
            echo "  --gdb          Let qemu listen on :1234 for gdb connections"
            echo "  --log          Log qemu events to /tmp/yaros.log"
            echo "  --capture      Capture network traffic into network.pcap"
            echo "  --net          Enable network card"
            echo "  -h, --help     Show this help message"
            exit 0
            ;;
        -*)
            echo "Unknown option: $1"
            exit 1
            ;;
        *)
            # Assume the last non-option argument is the kernel path
            KERNEL_PATH="$1"
            shift
            ;;
    esac
done

# Validate kernel path
if [[ -z "$KERNEL_PATH" ]]; then
    echo "Error: You must specify the kernel path."
    echo "Use $0 --help for more information."
    exit 1
fi

# Add the kernel option
QEMU_CMD+=" -kernel $KERNEL_PATH"

# Execute the QEMU command
echo "Executing: $QEMU_CMD"

exec bash -c "$QEMU_CMD"

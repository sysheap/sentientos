#!/usr/bin/env bash

set -e

cd "$(dirname "$0")"

QEMU_CMD="qemu-system-riscv64 \
    -machine virt \
    -cpu rv64 \
    -m 512M \
    -nographic \
    -serial mon:stdio"

# Process options
while [[ $# -gt 0 ]]; do
    case "$1" in
        --capture)
            QEMU_CMD+=" -object filter-dump,id=f1,netdev=netdev1,file=network.pcap "
            shift
            ;;
        --gdb)
            QEMU_CMD+=" -s"
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS] <KERNEL_PATH>"
            echo ""
            echo "Options:"
            echo "  --gdb          Let qemu listen on :1234 for gdb connections"
            echo "  --log          Log qemu events to /tmp/sentientos.log"
            echo "  --capture      Capture network traffic into network.pcap"
            echo "  --net PORT     Enable network card with host port PORT (default: 1234)"
            echo "  -h, --help     Show this help message"
            echo "  --wait         Wait cpu until gdb is attached"
            exit 0
            ;;
        --log)
            QEMU_CMD+=" -d guest_errors,cpu_reset,unimp,int -D /tmp/sentientos.log"
            shift
            ;;
        --net)
            shift
            if [[ "$1" =~ ^[0-9]+$ ]]; then
                NET_PORT="$1"
                shift
            else
                NET_PORT="1234"
            fi
            QEMU_CMD+=" -netdev user,id=netdev1,hostfwd=udp::${NET_PORT}-:1234 -device virtio-net-pci,netdev=netdev1"
            ;;
        --smp)
            QEMU_CMD+=" -smp $(nproc)"
            shift
            ;;
        --wait)
            QEMU_CMD+=" -S"
            shift
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

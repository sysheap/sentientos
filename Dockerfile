# CI Docker image with pre-populated Nix store
# Avoids slow Nix cache operations by having dependencies pre-installed

# Explicitly specify amd64 to match GitHub Actions runner architecture
FROM --platform=linux/amd64 ubuntu:24.04

# Install dependencies
# - libc6, libstdc++6: Required for GitHub Actions runner-provided Node.js binaries
# - nodejs/npm: Fallback if runner binaries don't work
RUN apt-get update && apt-get install -y \
    curl \
    xz-utils \
    git \
    ca-certificates \
    libc6 \
    libstdc++6 \
    nodejs \
    npm \
    && rm -rf /var/lib/apt/lists/*

# Install Nix using Determinate Systems installer (maintained, secure)
# https://github.com/DeterminateSystems/nix-installer
RUN curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install linux --no-confirm --init none

# Set up Nix environment (Determinate installer uses /nix/var/nix/profiles/default)
ENV PATH="/nix/var/nix/profiles/default/bin:${PATH}"

# Flakes are enabled by default with Determinate installer

# Copy flake files for dependency resolution
WORKDIR /workspace
COPY flake.nix flake.lock rust-toolchain ./

# Build the CI devShell to populate /nix/store
# This is the expensive operation we want to cache in the image
# RUN nix develop .#ci --command true

# Create helper script to run commands in nix develop environment
RUN printf '#!/bin/bash\nset -e\ncd "${GITHUB_WORKSPACE:-$(pwd)}"\nexec nix develop .#ci --command "$@"\n' > /usr/local/bin/nix-shell-run && \
    chmod +x /usr/local/bin/nix-shell-run

# No ENTRYPOINT - GitHub Actions needs to run its setup commands directly
# Use "nix-shell-run <command>" to run commands in the Nix environment

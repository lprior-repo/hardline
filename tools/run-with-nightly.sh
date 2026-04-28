#!/bin/bash
# Run a command with nightly Rust while keeping mise tools (jj, zellij) available
# This prepends nightly rust to PATH to override mise's rust

export PATH="$HOME/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin:$PATH"
exec "$@"

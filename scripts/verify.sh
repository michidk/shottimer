#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings

# Shared crates must never acquire a board HAL or a firmware dependency.
dependencies="$(cargo tree --locked -p shottimer-app --prefix none --edges normal)"
if printf '%s\n' "$dependencies" | rg -q '^(esp-hal|rp2040-hal|cortex-m|waveshare-rp2040|firmware-)'; then
    printf '%s\n' "Board dependency leaked into the shared application." >&2
    exit 1
fi
cargo clippy --locked -p firmware-rp2040 --target thumbv6m-none-eabi -- -D warnings
cargo rp2040 --locked
if [[ "${1:-}" == "--esp" ]]; then
    # Source the espup export script before invoking this optional check.
    cargo +esp clippy --locked -p firmware-esp32s3 --target xtensa-esp32s3-none-elf -Zbuild-std=core -- -D warnings
    cargo +esp esp32s3 --locked
fi

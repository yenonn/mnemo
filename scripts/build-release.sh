#!/bin/bash
set -e

cd "$(dirname "$0")/.."

echo "Building mnemo releases..."

TARGETS=(
    "aarch64-apple-darwin"
    "x86_64-apple-darwin"
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-musl"
)

for target in "${TARGETS[@]}"; do
    echo "Building for $target..."
    rustup target add "$target" 2>/dev/null || true
    if cargo build --release --target "$target" 2>/dev/null; then
        echo "  ✓ Success: target/$target/release/mnemo"
    else
        echo "  ✗ Failed: $target (skipping)"
    fi
done

echo "Done. Binaries in target/*/release/mnemo"

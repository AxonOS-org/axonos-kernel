#!/bin/bash
# AxonOS Build Script

set -e

TARGET_M4F="thumbv7em-none-eabihf"
TARGET_M33="thumbv8m.main-none-eabihf"

echo "=== AxonOS Build ==="

# Check Rust version
rustc --version

echo ""
echo "--- Building for Cortex-M4F ---"
cargo build --target $TARGET_M4F --features cortex-m4f --release

echo ""
echo "--- Building for Cortex-M33 ---"
cargo build --target $TARGET_M33 --features cortex-m33,trustzone --release

echo ""
echo "--- Running tests ---"
cargo test --lib

echo ""
echo "--- Running clippy ---"
cargo clippy --all-features -- -D warnings

echo ""
echo "--- Checking formatting ---"
cargo fmt --check

echo ""
echo "--- Building examples ---"
cargo build --example basic_pipeline --target $TARGET_M4F --release
cargo build --example dualcore_demo --target $TARGET_M4F --release

echo ""
echo "=== Build complete ==="

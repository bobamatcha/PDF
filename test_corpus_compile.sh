#!/bin/bash
# Test script to capture corpus-core compilation errors

echo "=== Testing corpus-core compilation ==="
echo ""

# Temporarily enable corpus-core in workspace
sed -i.bak 's/# "crates\/corpus-core",/"crates\/corpus-core",/' Cargo.toml

echo "Building corpus-core..."
cargo build -p corpus-core 2>&1 | tee corpus_compile_error.log

# Restore original Cargo.toml
mv Cargo.toml.bak Cargo.toml

echo ""
echo "=== Error log saved to corpus_compile_error.log ==="

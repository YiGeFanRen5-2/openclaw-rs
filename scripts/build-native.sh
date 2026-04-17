#!/usr/bin/env bash
# Build and copy native module for node-bridge
set -e

echo "🔨 Building Rust (release)..."
cargo build --release

echo "📦 Copying libffi.so to node-bridge/..."
cp target/release/libffi.so node-bridge/openclaw-rs.node

echo "✅ Done. Native module ready at node-bridge/openclaw-rs.node"
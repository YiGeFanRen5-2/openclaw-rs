#!/bin/bash
# Generate Rust API documentation

set -e

echo "Generating Rust API documentation..."
cargo doc --all --no-deps

echo ""
echo "Documentation generated in: target/doc/"
echo ""
echo "To view locally:"
echo "  cd target/doc && python3 -m http.server 8080"
echo "  Then open: http://localhost:8080"
echo ""
echo "To publish to GitHub Pages:"
echo "  cp -r target/doc/* ../docs/"
echo "  git add docs && git commit && git push"

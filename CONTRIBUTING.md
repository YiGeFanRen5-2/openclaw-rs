# Contributing to OpenClaw Rust Workspace

Thank you for your interest in contributing!

## Development Setup

```bash
# Clone and enter the workspace
cd openclaw-rs

# Install Rust (MSRV 1.75)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build
cargo build --release

# Run tests
cargo test --all

# Run with debug logging
RUST_LOG=debug cargo test
```

## Code Standards

### Formatting
```bash
cargo fmt --all
```
Configuration: `rustfmt.toml`

### Linting
```bash
cargo clippy --all
cargo clippy --fix --lib --all --allow-dirty  # Auto-fix warnings
```
Configuration: `clippy.toml`

### Testing
```bash
cargo test --all              # All tests
cargo test -p <crate>        # Single crate
cargo bench --all            # Benchmarks
```

## Crate Architecture

```
openclaw-rs/
├── api-client/      # Provider trait + adapters. Change here to add new LLM providers.
├── runtime/         # Session + compression. Core business logic lives here.
│   ├── lsp.rs      # LSP Bridge (gateway to harness)
│   └── compression.rs  # zstd compression
├── tools/          # Tool system. Add new tools here.
├── plugins/        # Plugin hooks. Extend via plugins, not this crate.
├── harness/        # LSP Client. Language server integrations.
├── mcp-server/   # MCP Server. Protocol implementation.
├── mcp-client/    # MCP Client. Connect to external servers.
└── node-bridge/   # N-API. Expose to Node.js.
```

## Adding a New Crate

```bash
cargo new --lib crates/my-crate
```

Then add to `workspace/Cargo.toml` members array and add inter-crate dependencies carefully.

## Adding a New Tool

1. Implement `tools::Tool` trait in `tools/src/lib.rs` or a new file.
2. Register in `register_builtin_tools()` function.
3. Add permission level in `tools::Permission` enum.
4. Add tests.

## Adding a New Provider Adapter

1. Create file in `api-client/src/provider/adapters/`.
2. Implement `Provider` trait.
3. Export from `api-client/src/provider/mod.rs`.
4. Add tests with mock requests.

## Adding a New LSP Method

1. Add method to `harness::LspClient`.
2. Wrap in `runtime::lsp::LspBridge`.
3. Expose via `node-bridge` N-API methods.
4. Add LSP protocol test (no server needed for unit tests).

## Pull Request Checklist

- [ ] `cargo fmt --all` passes
- [ ] `cargo clippy --all` passes (or acknowledged acceptable warnings)
- [ ] `cargo test --all` passes
- [ ] `cargo build --release` succeeds
- [ ] New public APIs documented
- [ ] Update CHANGELOG.md

## Commit Message Format

```
<type>(<scope>): <description>

[optional body]

Types: feat, fix, docs, test, chore, refactor
```

Example:
```
feat(runtime): add session token counting

Implement approximate token counting using content length / 4.
Benchmark: ~7ns per call.
```

## Benchmarking

```bash
cargo bench --all
# View results
ls target/criterion/
```

Benchmarks should run < 10s each. Use `--profile-time=3` for faster iteration.

## Documentation

```bash
cargo doc --all --no-deps
# Opens in browser
cargo doc --all --no-deps --open
```

Public APIs should have doc comments (`///`).

## Getting Help

- Open an issue at https://github.com/openclaw/openclaw
- Check PARITY.md for feature status
- Review PROJECT-SUMMARY.md for architecture overview

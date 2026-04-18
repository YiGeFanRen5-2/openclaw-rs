# Performance Troubleshooting Guide

## Common Performance Issues and Solutions

### 1. High Memory Usage

**Symptoms**: Memory usage continuously growing

**Diagnosis**:
```bash
# Check memory usage
ps aux | grep openclaw
top -p $(pgrep -f openclaw)

# Monitor heap allocations
RUSTFLAGS="--instrumentation" cargo build
```

**Solutions**:
- Enable session compaction (auto-compress at 80% threshold)
- Reduce `max_tokens` setting
- Implement session persistence to disk
- Use JSON store instead of in-memory for large datasets

---

### 2. Slow Tool Execution

**Symptoms**: Tools taking > 1s to execute

**Diagnosis**:
```bash
# Enable debug logging
RUST_LOG=debug cargo run --example example_01
```

**Solutions**:
- Check sandbox overhead (consider `--no-sandbox` for trusted tools)
- Optimize tool input size
- Enable parallel tool execution where possible
- Cache repeated tool calls

---

### 3. Session Bloat

**Symptoms**: Session size growing large (> 1MB)

**Causes**:
- Large messages being added
- No compaction triggered
- Metadata accumulation

**Solutions**:
```rust
// Enable auto-compaction
runtime.set_compaction_threshold(0.8);

// Manually compact
runtime.compact_session(session_id);

// Or persist to reduce memory
runtime.persist_session(session_id);
```

---

### 4. Concurrent Request Bottleneck

**Symptoms**: Requests queuing, high latency

**Diagnosis**:
```bash
# Run with concurrency metrics
cargo run --release -- \
  --metrics-port 9090 \
  --max-concurrent-sessions 100
```

**Solutions**:
- Increase `max_concurrent_sessions`
- Use session pooling
- Implement request batching
- Consider horizontal scaling

---

### 5. Compilation Slow

**Symptoms**: `cargo build` taking > 5 minutes

**Solutions**:
- Use `cargo build --release` for production
- Enable sccache: `cargo install sccache`
- Use incremental compilation: `CARGO_INCREMENTAL=1`
- Use link-time optimization: `LTO = "thin"`

---

## Performance Profiling

### Using `perf`

```bash
# Profile running process
perf record -g -p $(pgrep -f openclaw) -- sleep 30
perf report
```

### Using `tokio-console`

```bash
# Add to Cargo.toml
tokio-console = "0.1"

# Run
console run --project-dir .
```

### Memory Profiling

```bash
# Install heaptrack
cargo build --release
heaptrack ./target/release/openclaw-cli

# View results
heaptrack_print ./heaptrack_openclaw.*.heaptrail
```

---

## Performance Checklist

Before reporting performance issues:

- [ ] Memory usage measured?
- [ ] Benchmark run with `--release`?
- [ ] Profiler attached?
- [ ] Reproduction steps documented?
- [ ] Environment details (Rust version, OS, etc.)?

---

*Last updated: 2026-04-18*

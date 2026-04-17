# Performance Guide

This document describes OpenClaw's performance characteristics and optimization strategies.

## Benchmark Results

### Session Operations

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| `session_new` | < 1ms | ~1.2µs | ✅ |
| `session_add_message` | < 1ms | ~3µs | ✅ |
| `session_token_count` | < 10ns | ~7.3ns | ✅ |

### Provider Operations

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| `chat_request_new` | < 1ms | ~300ns | ✅ |
| `chat_message_new` | < 1ms | ~23ns | ✅ |
| `provider_config_serialize` | < 1ms | ~150ns | ✅ |

### Tool Execution

| Tool | Typical Duration |
|------|------------------|
| `list_files` | < 1ms |
| `read_file` | < 5ms (depending on file size) |
| `write_file` | < 10ms (depending on file size) |
| `http_request` | Network dependent |
| `text_stats` | < 1ms |

## Performance Optimization Tips

### 1. Session Compaction

Enable automatic session compaction to manage memory:

```rust
Runtime::builder()
    .max_tokens(4000)
    .compact_threshold(0.8)
    .build()
```

### 2. Tool Batching

Group multiple tool calls when possible to reduce overhead.

### 3. Provider Selection

- **Mock**: Fastest for testing
- **Anthropic**: Best for production
- **OpenAI**: Good balance

### 4. Resource Limits

Configure appropriate limits to prevent resource exhaustion:

```rust
SandboxConfig {
    limits: ResourceLimits {
        max_memory: 512 * 1024 * 1024,  // 512MB
        max_cpu_time: 30,                 // 30s
        max_file_size: 10 * 1024 * 1024, // 10MB
    },
}
```

### 5. Connection Pooling

Use persistent connections for HTTP requests:

```rust
reqwest::Client::builder()
    .pool_max_idle_per_host(10)
    .build()
```

## Profiling

### CPU Profiling

```bash
# Install cargo-flamegraph
cargo install cargo-flamegraph

# Generate flamegraph
cargo flamegraph --bin openclaw-cli -- run --prompt "..."
```

### Memory Profiling

```bash
# Install heaptrack
cargo install cargo-heaptrack

# Profile memory
cargo heaptrack --bin openclaw-cli -- run --prompt "..."
```

### Benchmarking

```bash
# Run benchmarks
cargo bench

# View results
ls target/criterion/
```

## Performance Monitoring

Enable metrics collection:

```rust
let metrics = MetricsCollector::new();
metrics.record_tool_call("my_tool", true, duration);
```

Export metrics:

```rust
let prometheus = metrics.export_prometheus();
```

## Known Bottlenecks

| Component | Issue | Workaround |
|-----------|-------|------------|
| Session compaction | CPU intensive | Increase threshold |
| Large file reads | Memory usage | Stream processing |
| Network tools | Latency | Add caching |

## Contributing

When optimizing code:

1. Measure before optimizing
2. Use benchmarks to verify improvements
3. Consider trade-offs (readability vs. performance)
4. Document performance-sensitive code

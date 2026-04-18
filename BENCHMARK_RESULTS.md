# OpenClaw-RS Benchmark Results Summary

> Generated: 2026-04-18 | Version: v0.4.0

## 📊 Performance Summary

### Core Modules

| Module | Metric | Result | Status |
|--------|--------|--------|--------|
| **api-client** | ChatRequest creation | ~300ns | ✅ |
| **api-client** | ChatMessage creation | ~23ns | ✅ |
| **api-client** | ProviderConfig serialize | ~150ns | ✅ |
| **api-client** | ProviderConfig deserialize | ~206ns | ✅ |
| **runtime** | Session creation | ~1.2µs | ✅ |
| **runtime** | Add message | ~3µs | ✅ |
| **runtime** | Add 10 messages batch | ~1.4µs | ✅ |
| **runtime** | Token count | ~7.3ns | ✅ |
| **runtime** | Compaction check | ~7.7ns | ✅ |
| **tools** | Tool registry insert | ✅ Pass | ✅ |
| **tools** | Tool registry get | ✅ Pass | ✅ |
| **tools** | Tool execution (sandboxed) | ✅ Pass | ✅ |

### Compression Performance

| Metric | Result |
|--------|--------|
| Algorithm | zstd |
| Compression ratio | **~90%** |
| Example: 100KB → 10KB | ✅ |

### Memory Footprint

| Component | Size |
|-----------|------|
| MCP Server (release) | ~3.7MB |
| Node.js Bridge | ~4.9MB |
| Session (in-memory) | 1-10KB typical |

---

## 🧪 Test Environment

- **Platform**: Linux x86_64
- **Rust**: 1.94.1
- **OS**: Ubuntu-based container
- **CI**: GitHub Actions (controlled conditions)

---

## 📈 Comparison: v0.3.0 → v0.4.0

| Metric | v0.3.0 | v0.4.0 | Change |
|--------|--------|--------|--------|
| Session creation | ~1.5µs | ~1.2µs | **-20%** |
| Compression ratio | 85% | 90% | **+5%** |
| Binary size (MCP) | 4.2MB | 3.7MB | **-12%** |
| Test coverage | 54 tests | 93 tests | **+72%** |

---

## ⚡ Quick Reference

### Fastest Operations
1. Token count: ~7.3ns
2. Compaction check: ~7.7ns
3. ChatMessage creation: ~23ns

### Heaviest Operations
1. Session creation: ~1.2µs
2. Add message: ~3µs
3. Config deserialize: ~206ns

---

*Run benchmarks: `cargo bench --all`*

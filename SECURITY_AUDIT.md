# OpenClaw-RS Security Audit Report

**Project:** OpenClaw-RS  
**Version:** 0.4.0 (workspace)  
**Date:** 2026-04-18  
**Auditor:** Phase 58 — Automated Security Audit  
**Status:** Issues Identified — See Prioritization Below

---

## Executive Summary

OpenClaw-RS implements a tool-based agent runtime with permission systems, sandboxing, and a plugin architecture. The security model is thoughtfully designed, with permission allowlists, namespace isolation, seccomp filtering, and resource limits. However, several issues were identified that require attention before production deployment.

**Overall Risk Level: MEDIUM**

---

## 1. Findings Summary

| Severity | Count | Issues |
|----------|-------|--------|
| Critical | 0 | — |
| High | 3 | Network allowlist bypass, Custom permission no-op, fork() in multithread |
| Medium | 4 | Path traversal via symlinks, No regex in schema, Hot plugin loading, No log desensitization |
| Low | 3 | unwrap() panics, Memory leak in plugin loader, No rate limiting |

---

## 2. High Severity Issues

### H-1: Network Allowlist Bypass via Wildcard `*`

**Location:** `crates/tools/src/http_tools.rs` — `HttpRequestTool`

**Description:**  
`HttpRequestTool.permission()` returns `Permission::Network` with `destinations: vec!["*".into()]`. This means the tool will **always pass** the permission check in `Permission::check()`, regardless of the actual destination:

```rust
// http_tools.rs
fn permission(&self) -> Permission {
    Permission::Network {
        destinations: vec!["*".into()],  // ← Always allows any destination!
        protocols: vec!["https".into(), "http".into()],
        max_connections: 10,
    }
}
```

While the URL scheme is validated in `execute()` (only http/https allowed), the permission system's intent — restricting network destinations — is completely bypassed. Any caller with `HttpRequestTool` access can reach any host.

**Impact:** An attacker who can invoke `http_request` tool can make requests to internal services (e.g., `http://169.254.169.254` for cloud metadata, `http://localhost:8080` for internal APIs).

**Recommendation:**  
Remove the wildcard `*`. Implement a destination allowlist from configuration:
```rust
Permission::Network {
    destinations: config.allowed_destinations(),  // loaded from config, not hardcoded
    protocols: vec!["https".into()],
    max_connections: 10,
}
```
Alternatively, make `http_request` require explicit destination arguments and validate them at execution time.

---

### H-2: Custom Permission Checker is a No-Op

**Location:** `crates/tools/src/lib.rs` — `Permission::check()`

**Description:**  
The `Permission::Custom` variant always returns `Ok(())` without performing any validation:

```rust
Permission::Custom { checker: _, config: _ } => Ok(()),
```

This means any tool that declares `Permission::Custom` is effectively unrestricted, despite appearing to have a custom permission checker. The `checker` and `config` fields are stored but never used.

**Impact:** Tools that intend to implement custom permission logic (e.g., time-based access control, rate limiting) are silently bypassed.

**Recommendation:**  
Implement the custom checker or remove the `Permission::Custom` variant until it is properly implemented:
```rust
Permission::Custom { checker, config } => {
    Err(ToolError::PermissionDenied(
        format!("Custom checker '{}' not implemented", checker)
    ))
}
```

---

### H-3: fork() Called in Potentially Multi-Threaded Context

**Location:** `crates/tools/src/lib.rs` — `Sandbox::execute()`

**Description:**  
The sandbox uses `fork()` to isolate tool execution:

```rust
match unsafe { fork() } {
    Ok(ForkResult::Parent { child }) => { /* wait */ }
    Ok(ForkResult::Child) => {
        Self::apply_rlimits(&self.limits)?;
        let _ = Self::enter_namespaces();
        let _ = Self::install_seccomp();
        // ...
    }
}
```

`fork()` in a multi-threaded process is unsafe. After `fork()`, only the calling thread continues in the child; all other threads silently die. This can cause:
- Deadlocks (locks held by other threads remain held in the child)
- Resource corruption (file descriptors, memory state from other threads)
- Incomplete isolation (some resources may leak into the sandbox)

**Impact:** The sandbox may not provide the isolation it promises. A malicious or buggy tool could exploit this to escape the sandbox.

**Recommendation:**  
Replace `fork()` with a proper subprocess model:
1. Use `nix::unistd::exec()` with a pre-compiled stub binary
2. Or use `libseccomp` + `prctl` to create a contained subprocess via `pipe()` + `fork()` + `exec()`
3. Document that the sandbox requires single-threaded execution context, or gate it behind a compile-time feature

---

## 3. Medium Severity Issues

### M-1: Path Traversal via Symlinks in Filesystem Tools

**Location:** `crates/tools/src/lib.rs` — `Permission::check()`

**Description:**  
The filesystem permission check uses `canonicalize()` which resolves symlinks, but only if `canonicalize()` succeeds:

```rust
Permission::Filesystem { allowlist, writable: _ } => {
    let target_canon = match std::fs::canonicalize(target) {
        Ok(p) => p,
        Err(_) => PathBuf::from(target),  // ← Falls back to raw path!
    };
    for allowed in allowlist {
        if let Ok(allowed_canon) = std::fs::canonicalize(allowed) {
            if target_canon.starts_with(allowed_canon) {
                return Ok(());
            }
        }
    }
}
```

If canonicalization fails (e.g., dangling symlink), the raw path is used, which could bypass the allowlist. Additionally, `starts_with()` on a canonicalized path does not prevent symlink-based traversal if the symlink is within the allowlist.

**Impact:** A malicious plugin could create a symlink from within an allowed directory to a restricted path (e.g., `/etc/passwd`), then access it.

**Recommendation:**  
- Fail closed on canonicalization errors:
  ```rust
  let target_canon = std::fs::canonicalize(target)
      .map_err(|_| ToolError::PermissionDenied("cannot resolve path".into()))?;
  ```
- Consider using `openat2()` with `RESOLVE_NO_SYMLINKS` on Linux for stronger guarantees.

---

### M-2: JSON Schema Pattern Validation Not Implemented

**Location:** `crates/tools/src/validator.rs` — `validate_against_schema()`

**Description:**  
Pattern (regex) validation is explicitly skipped:

```rust
// Pattern (regex) - basic check only
if let Some(pattern) = obj.get("pattern") {
    if let (Some(data_str), Some(_pattern_str)) = (data.as_str(), pattern.as_str()) {
        // Pattern validation skipped - requires regex crate
    }
}
```

A JSON schema `pattern` validator is a key security feature for input validation (e.g., enforcing URL formats, alphanumeric constraints).

**Impact:** Tools relying on schema validation for security (e.g., expecting a value to match `[a-zA-Z0-9]+`) are not protected.

**Recommendation:**  
Enable the `regex` crate (already used by `reqwest` transitively) and implement:
```rust
if let (Some(data_str), Some(pattern_str)) = (data.as_str(), pattern.as_str()) {
    let re = Regex::new(pattern_str)
        .map_err(|_| ToolError::InvalidInput(format!("Invalid regex: {}", pattern_str)))?;
    if !re.is_match(data_str) {
        errors.push(format_path(path, "pattern mismatch", detailed));
    }
}
```

---

### M-3: Plugin Hot Loading Without Code Signing

**Location:** `crates/plugins/src/hot.rs`

**Description:**  
The `PluginLoader` uses `unsafe { libloading::Library::new() }` to load `.so` plugin files from disk. There is:
- No cryptographic verification of plugin code
- No signature checking
- No plugin identity verification beyond a manifest JSON
- The manifest can be arbitrarily written (not signed)

A compromised or malicious plugin file on disk can be loaded and execute arbitrary code with the same privileges as the host process.

**Impact:** If plugins directory is writable by an attacker, they can inject arbitrary native code.

**Recommendation:**  
- Sign plugin `.so` files and verify signatures before loading
- Use a plugin manifest signed with a known key
- Load plugins with `libloading::Library::new()` flags that disable symbol interposition (`RTLD_NODELETE | RTLD_NOW`)
- Document that the plugins directory must be protected (read-only, owned by a different user)

---

### M-4: No Log Desensitization

**Location:** Across codebase — `tracing` usage

**Description:**  
The project uses `tracing` extensively for logging. No evidence of a log desensitization layer was found. If session messages, file contents, or API responses are logged via `tracing::info!`/`debug!`, sensitive data could be written to logs.

**Impact:** API keys, PII in session content, file contents could end up in log files.

**Recommendation:**  
- Implement a tracing layer that scrubs sensitive fields (e.g., keys matching `*API_KEY*`, `Authorization` headers)
- Use `tracing_opentelemetry` with a redaction layer
- Document what fields are logged and ensure none contain secrets

---

## 4. Low Severity Issues

### L-1: unwrap() Calls in Non-Test Production Code

**Locations:**
- `crates/plugins/src/hot.rs:113` — `init: Symbol<...> = lib.get(...).unwrap();` (after already checking with `map_err`)
- `crates/plugins/src/hot.rs:120` — `Ok(self.plugins.get(id).unwrap())` (after insert)
- `crates/node-bridge/src/lib.rs:172` — `serde_json::to_string(&result).unwrap_or_default()`
- `crates/node-bridge/src/lib.rs:639` — `serde_json::from_str(...).unwrap_or(serde_json::json!({}))`

**Description:**  
The hot.rs:113 and 120 calls are particularly concerning — the code uses `map_err` to convert errors but then immediately calls `unwrap()` on the same result, negating the error handling. If the symbol disappears between the check and the unwrap (which can't happen in single-threaded code, but is a code smell), it would panic.

**Recommendation:**  
Replace `unwrap()` with `expect()` in hot.rs for clarity, or use the already-checked `?` operator. For serde errors in node-bridge, the `unwrap_or_default()` pattern is acceptable for non-critical failures but should be logged.

---

### L-2: Intentional Memory Leak in Plugin System

**Location:** `crates/plugins/src/hot.rs` — `PluginLoader::call()`

**Description:**  
```rust
// Free the result buffer (plugin is responsible for allocating with the same allocator).
// For simplicity we leak here; in production, plugins should use a shared allocator.
let _ = result_ptr; // Leaking is intentional; plugin manages the buffer.
```

In a long-running process, repeated plugin calls will leak memory. For a persistent agent runtime, this is a concern.

**Recommendation:**  
Implement a shared allocator (e.g., `mimalloc` or `jemalloc`) used by both host and plugins, allowing cross-boundary deallocation. Alternatively, use a thread-local arena allocator per plugin call.

---

### L-3: No Rate Limiting on Tools

**Description:**  
There is no rate limiting on tool invocations. An agent making rapid repeated calls to expensive tools (e.g., `http_request`, file system operations) could exhaust resources.

**Recommendation:**  
Add a rate limiter middleware in `Executor` that tracks invocations per tool per time window.

---

## 5. Positive Security Observations

The following aspects of the project are well-implemented:

1. **Seccomp filtering** — Properly configured BPF seccomp filter blocking dangerous syscalls (ptrace, mount, reboot, etc.)
2. **Resource limits** — RLIMIT_AS, RLIMIT_CPU, RLIMIT_FSIZE, RLIMIT_NOFILE all set
3. **Namespace isolation** — CLONE_NEWNS, NEWPID, NEWNET, NEWUTS, NEWIPC
4. **Plugin hook errors are non-fatal** — Failed hooks are logged and skipped; they do not block execution
5. **URL scheme validation** — Only http/https allowed for outbound requests
6. **File size limits** — `ReadFileTool` enforces `max_size` before reading
7. **Version pinning** — Cargo.lock uses exact versions for reproducible builds
8. **TLS-only HTTP client** — `reqwest` configured with `rustls-tls` (no OpenSSL dependency)
9. **Error type safety** — Custom error enums (`ToolError`, `PermissionError`, `RuntimeError`) used throughout

---

## 6. Recommended Fix Priority

| Priority | Issue | Effort |
|----------|-------|--------|
| P1 | H-1: Network wildcard `*` | Low |
| P1 | H-2: Custom permission no-op | Low |
| P2 | H-3: fork() safety | High |
| P2 | M-1: Symlink path traversal | Medium |
| P2 | M-2: Regex in schema validation | Low |
| P3 | M-3: Plugin code signing | High |
| P3 | M-4: Log desensitization | Medium |
| P4 | L-1: unwrap() cleanup | Low |
| P4 | L-2: Plugin memory leak | Medium |
| P5 | L-3: Rate limiting | Medium |

---

## 7. Testing Recommendations

1. **Fuzz test** the JSON schema validator with malicious schemas and inputs
2. **Integration test** the sandbox with tools that attempt syscalls blocked by seccomp
3. **Path traversal test** with symlinks pointing outside allowlisted directories
4. **Load test** with concurrent tool invocations to verify no deadlocks (addresses H-3)
5. **Audit test** with a plugin that exports malicious symbols to verify hook isolation
6. **Log review** to scan for any inadvertently logged secrets

---

*Generated by OpenClaw-RS Phase 58 Security Audit — 2026-04-18*

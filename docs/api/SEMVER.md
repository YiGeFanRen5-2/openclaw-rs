# API Versioning & Stability Guarantees

> OpenClaw-RS follows semantic versioning with explicit stability tiers.

---

## Public vs Internal API

### Public API
Everything exported at the crate root level (`lib.rs`) is considered **public API**:

```rust
// Public API examples:
pub use models::{ChatMessage, ChatResponse};
pub use provider::{Provider, ProviderCapabilities, ProviderError};
```

**Rules:**
- Semantically versioned (MAJOR.MINOR.PATCH)
- Covered by stability guarantees (see below)
- Breaking changes require MAJOR version bump
- Documented in CHANGELOG with migration notes

### Internal API
Everything with `pub(crate)`, `mod` (private), or under `sys/` paths is **internal**:

```rust
// Internal API examples:
pub(crate) fn internal_helper();
mod private_module;
mod sys;
```

**Rules:**
- No stability guarantees
- Can change any time without notice
- No CHANGELOG entries required

---

## Stability Tiers

### 🥇 Stable (`stable`)
**Best-effort compatibility.** Minor updates may introduce deprecations but won't break functionality.

- ✅ Backward compatible within major version
- ⚠️ Deprecations announced 1 minor version ahead
- 🚫 No breaking changes without MAJOR bump

**Crate roots at this tier:**
- `api-client` (core provider interface)

### 🥈 Beta (`beta`)
**Opt-in preview.** May change based on feedback; not recommended for production.

- ⚠️ API may change in future releases
- ⚠️ Features may be renamed or removed
- ⚠️ Breaking changes possible without MAJOR bump

**Crate roots at this tier:**
- `mcp-client`, `mcp-server` (Model Context Protocol)

### 🥉 Experimental (`experimental`)
**Unstable, rapid iteration.** Use at your own risk.

- 🚫 No compatibility guarantees
- 🚫 May be renamed, removed, or replaced
- 🚫 No migration path guaranteed

**Crate roots at this tier:**
- `node-bridge`, `plugin` (early-stage integrations)

---

## Version Format

OpenClaw-RS uses [Semantic Versioning 2.0.0](https://semver.org/):

```
MAJOR.MINOR.PATCH[-prerelease][+build]
  │      │     │
  │      │     └── Patch: Bug fixes, no API changes
  │      └──────── Minor: New features, backward compatible
  └─────────────── Major: Breaking changes
```

**Examples:**
- `0.1.0` → Initial development
- `0.2.0` → New provider added (backward compatible)
- `1.0.0` → First stable release
- `2.0.0` → Breaking changes from 1.x

---

## Breaking Change Definition

A **breaking change** is any modification that causes existing code to fail to compile or produce different behavior:

| Change Type | Breaking? |
|-------------|-----------|
| Remove or rename `pub` item | ✅ Yes |
| Change function signature | ✅ Yes |
| Change trait method signature | ✅ Yes |
| Change public struct field | ✅ Yes |
| Change enum variant or fields | ✅ Yes |
| Change default values | ✅ Yes |
| Add new required parameter | ✅ Yes |
| Add new `pub` item | ❌ No |
| Add new optional parameter | ❌ No |
| Relax constraints (more permissive) | ❌ No |
| Bug fixes | ❌ No |

---

## Deprecation Policy

1. Deprecated items marked with `#[deprecated(note = "...")]`
2. Deprecation announced in CHANGELOG under "Deprecated"
3. Deprecated items **must** remain functional for at least 1 minor version
4. Removal requires MAJOR version bump

---

## Changelog Categories

Entries must be categorized using these labels:

| Category | Meaning |
|----------|---------|
| **Added** | New features |
| **Changed** | Changes in existing functionality |
| **Deprecated** | Soon-to-be removed features |
| **Removed** | Previously deprecated features now removed |
| **Fixed** | Bug fixes |
| **Security** | Vulnerability fixes |
| **BREAKING** | Breaking changes (prominent) |

---

## Crate Stability Matrix

| Crate | Version | Tier | Notes |
|-------|---------|------|-------|
| `api-client` | 0.1.0 | 🥇 Stable | Core model provider interface |
| `runtime` | 0.1.0 | 🥇 Stable | Execution runtime |
| `tools` | 0.1.0 | 🥇 Stable | Tool execution framework |
| `plugins` | 0.1.0 | 🥈 Beta | Plugin system |
| `ffi` | 0.1.0 | 🥈 Beta | FFI bindings |
| `harness` | 0.1.0 | 🥈 Beta | Test harness |
| `mcp-client` | 0.1.0 | 🥈 Beta | MCP client |
| `mcp-server` | 0.1.0 | 🥈 Beta | MCP server |
| `node-bridge` | 0.1.0 | 🥉 Experimental | Node.js bridge |
| `openclaw-core` | 0.1.0 | 🥉 Experimental | Core logic |
| `plugin` | 0.1.0 | 🥉 Experimental | Plugin interface |

---

_This document follows [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) and [Semantic Versioning 2.0.0](https://semver.org/)._

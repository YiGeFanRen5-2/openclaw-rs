# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | ✅ Yes - Production |
| < 0.2.0 | ⚠️  Use at own risk |

## Reporting a Vulnerability

If you discover a security vulnerability within OpenClaw Rust Workspace, please report it responsibly:

1. **Do NOT** open a public GitHub issue for security vulnerabilities.
2. Email the maintainers directly or use GitHub's private vulnerability reporting.
3. Include a detailed description and steps to reproduce.
4. Allow 48 hours for initial response.

## Sandbox Security

The tool execution sandbox (`tools` crate) uses Linux namespaces and seccomp for isolation:

```text
CLONE_NEWNS   - Mount namespace isolation
CLONE_NEWPID  - Process isolation
CLONE_NEWUTS  - Hostname isolation
CLONE_NEWIPC   - IPC isolation
CLONE_NEWNET   - Network isolation (denied by default)
```

Resource limits enforced:
- Memory: 512MB per tool execution
- CPU time: 30s hard limit
- File size: 10MB per write
- Open files: 1024

Seccomp deny list includes:
```text
ptrace, mount, pivot_root, unshare, setns,
reboot, kexec_load, init_module, finit_module,
syslog, lookup_dcookie, perf_event_open,
}
```

## Tool Permissions

Tools require explicit permission levels:

| Permission | Allowed Operations |
|-----------|------------------|
| `Safe` | No restrictions |
| `Filesystem { allowlist }` | Only paths in allowlist |
| `Shell { allowlist }` | Only commands in allowlist |
| `Network { allowlist }` | Only targets in allowlist |
| `Custom` | Custom validation |

## Plugin Hot-Reload

Dynamically loaded `.so` plugins are executed in the same process. Only load plugins from trusted sources. Use `PluginLoader::discover()` with a controlled `base_dir`.

## Node.js API Security

When exposing `OpenClawRuntime` over network, ensure:
- API key authentication for provider access
- Session isolation between users
- Rate limiting on `execute_tool()` calls
- Allowlist-only tool permissions

## Known Limitations

- Plugin hot-reload runs plugins in-process (no process isolation yet)
- Anthropic function/tool calling not yet implemented
- MCP client streaming not yet implemented

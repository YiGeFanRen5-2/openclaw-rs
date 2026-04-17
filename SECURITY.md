# Security Policy

## Reporting Security Issues

If you discover a security vulnerability, please report it by:
- Opening a private security advisory on GitHub
- Or contacting the maintainers directly

**DO NOT** create public issues for security vulnerabilities.

## Security Model

OpenClaw implements a multi-layer security model:

### 1. Permission System

Every tool declares its required permissions:

```rust
fn permission(&self) -> Permission {
    Permission::Safe  // No restrictions
    // OR
    Permission::Filesystem {
        allowlist: vec!["/tmp".into()],
        writable: false,
    }
    // OR
    Permission::Network {
        destinations: vec!["api.example.com".into()],
        protocols: vec!["https".into()],
        max_connections: 10,
    }
}
```

### 2. Permission Types

| Type | Description |
|------|-------------|
| `Safe` | No restrictions |
| `Filesystem` | Access to specific paths only |
| `Shell` | Execution of allowed commands only |
| `Network` | Access to specific destinations only |
| `Custom` | Custom permission checker |

### 3. Sandbox Execution

Tools can be executed in isolated sandboxes:

```rust
SandboxConfig {
    namespaces: vec![
        CLONE_NEWNS,  // Mount namespace
        NEWPID,       // PID namespace
        NEWNET,       // Network namespace
    ],
    limits: ResourceLimits {
        max_memory: 512 * 1024 * 1024,  // 512MB
        max_cpu_time: 30,                 // 30 seconds
        max_file_size: 10 * 1024 * 1024, // 10MB
        max_open_files: 1024,
    },
    seccomp: vec![
        // Block dangerous syscalls
        SYS_ptrace,
        SYS_mount,
        SYS_pivot_root,
        SYS_unshare,
        SYS_setns,
        SYS_reboot,
    ],
}
```

## Best Practices

### For Tool Developers

1. **Minimize Permissions**: Request only what's needed
2. **Validate Input**: Always validate tool inputs
3. **Sanitize Output**: Escape sensitive data in responses
4. **Log Actions**: Record security-relevant events

### For Deployment

1. **Run as Non-Root**: Use dedicated service accounts
2. **Limit Resources**: Configure appropriate limits
3. **Network Isolation**: Use firewall rules for production
4. **Monitor Logs**: Watch for permission denied events

## Security Checklist

Before deploying OpenClaw:

- [ ] Review all tool permissions
- [ ] Configure firewall rules
- [ ] Set up resource limits
- [ ] Enable audit logging
- [ ] Use environment variables for secrets
- [ ] Run as non-root user
- [ ] Regular security updates

## Incident Response

If a security incident occurs:

1. **Contain**: Isolate affected components
2. **Assess**: Determine scope and impact
3. **Remediate**: Apply fixes
4. **Report**: Document and notify users
5. **Recover**: Restore normal operations

## CVEs and Security Updates

Security updates are prioritized and released immediately.
Check the [CHANGELOG](CHANGELOG.md) for security-related changes.

## Version Support

| Version | Supported | Notes |
|---------|-----------|-------|
| 0.2.x | ✅ Yes | Current stable |
| 0.1.x | ⚠️ Limited | Security fixes only |
| < 0.1 | ❌ No | Upgrade recommended |

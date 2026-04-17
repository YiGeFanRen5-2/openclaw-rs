# Phase 3: 沙箱执行 + 权限强化

**开始日期**：2026-04-06  
**目标**：实现安全的工具执行环境，确保恶意或 buggy 工具不会破坏主机系统

---

## 🎯 核心目标

1. **Linux 沙箱**：使用 namespaces + seccomp 隔离工具执行
2. **路径安全**：realpath + canonicalize 防止路径遍历攻击
3. **资源限制**：ulimit 风格的内存、CPU、文件大小限制
4. **网络策略**：隔离或严格白名单网络访问
5. **权限检查前移**：执行前强制验证

---

## 📋 任务分解

### Task 3.1: 路径规范化验证

**问题**：当前白名单检查使用 `starts_with`，易受符号链接/`..` 绕过

**方案**：
- 在 `Permission::Filesystem::check()` 中对 `target` 和 `allowlist` 都调用 `std::fs::canonicalize`
- 缓存 canonicalized 结果避免重复 I/O

**修改文件**：
- `crates/tools/src/lib.rs` - `Permission::check()`

**测试**：
- 添加测试：`/root/../etc/passwd` 应被拒绝
- 添加测试：符号链接指向白名单外应被拒绝

---

### Task 3.2: 基础沙箱结构

**设计**：
```rust
pub struct Sandbox {
    pub root: Option<PathBuf>,      // chroot 目录（如果指定）
    pub limits: ResourceLimits,     // rlimit 设置
    pub network_policy: NetworkPolicy,
    pub seccomp_filter: Option<SeccompFilter>,
}

impl Sandbox {
    pub fn new() -> Self { ... }
    pub async fn execute<T: Tool>(&self, tool: &T, input: JsonValue) -> Result<JsonValue, ToolError> {
        // 1. 验证权限（调用 permission.check）
        // 2. 设置资源限制（prlimit）
        // 3. 进入 namespace（如果启用）
        // 4. 执行工具
        // 5. 返回结果
    }
}
```

**文件**：
- 修改 `crates/tools/src/lib.rs` - 实现 `Sandbox::execute()`

**依赖**：
- `nix` crate（系统调用）
- `seccompiler` 或 `libseccomp`（seccomp 过滤器）

---

### Task 3.3: Linux Namespaces 隔离

**所需 namespace**：
- `CLONE_NEWNS` - 挂载隔离（chroot 可选）
- `CLONE_NEWPID` - 进程隔离
- `CLONE_NEWNET` - 网络隔离
- `CLONE_NEWUTS` - 主机名隔离
- `CLONE_NEWIPC` - IPC 隔离

**实现方式**：
- 使用 `nix::unistd::clone` 创建子进程
- 子进程中设置 namespace，然后 exec 工具执行（或直接调用同步函数）

**注意**：需要 root 权限或 user namespace（`CLONE_NEWUSER`）

**安全考虑**：
- 默认启用所有 namespaces
- 提供配置开关（某些场景可能不需要 PID namespace）

---

### Task 3.4: Seccomp 过滤器

**目标**：限制工具可用的系统调用

**默认策略**（strict）：
- 允许：`read`, `write`, `open`, `close`, `fstat`, `mmap`, `mprotect`, `munmap`, `brk`, `rt_sigaction`, `rt_sigprocmask`, `sigaltstack`, `pipe`, `pipe2`, `dup`, `dup2`, `getpid`, `getuid`, `getgid`, `clock_gettime`, `exit_group`
- 拒绝：所有其他系统调用 → 返回 `EPERM`

**实现**：
- 使用 `seccompiler` crate 生成过滤器
- 通过 `prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, ...)` 激活

**性能**：seccomp 过滤器在内核态，开销极小

---

### Task 3.5: 资源限制 (ulimit)

**限制项**：
- `RLIMIT_AS` - 最大内存（虚拟内存）
- `RLIMIT_CPU` - CPU 时间（秒）
- `RLIMIT_FSIZE` - 文件大小
- `RLIMIT_NOFILE` - 最大打开文件数
- `RLIMIT_NPROC` - 最大进程数

**实现**：
- 使用 `nix::sys::resource::setrlimit`
- 在 fork 子进程后、exec 前设置

**配置**：
```rust
pub struct ResourceLimits {
    pub max_memory_mb: Option<u64>,
    pub max_cpu_seconds: Option<u64>,
    pub max_file_size_mb: Option<u64>,
    pub max_open_files: Option<u64>,
    pub max_processes: Option<u64>,
}
```

---

### Task 3.6: 网络策略

**NetworkPolicy 枚举**：
- `Isolated` - 完全隔离（默认）
- `LoopbackOnly` - 仅本地回环
- `Allowlist(Vec<String>)` - 允许连接指定 IP/Domain
- `Unrestricted` - 无限制（危险，仅调试）

**实现**：
- 如果 `CLONE_NEWNET` 启用，创建新 network namespace
- 对于 `Allowlist`，使用 iptables/eBPF 规则（Phase 4 细化）

---

### Task 3.7: 集成到 Runtime

**修改 Runtime**：
- 在 `Runtime::new()` 中配置默认沙箱
- `execute_tool` 改为使用 `sandbox.execute(tool, args)` 而非直接调用
- 考虑 A/B 测试：同时运行 sandboxed 和 non-sandboxed，比较结果

**API 保持兼容**：
- 对 FFI/Node 透明
- 错误处理统一（`ToolError::PermissionDenied`, `ResourceLimit`）

---

## 🧪 测试计划

### 单元测试

- `Sandbox::execute` 正常路径（`list_files` 在沙箱内）
- 权限拒绝：尝试访问白名单外路径
- 资源限制：OOM 测试、CPU 时间耗尽
- seccomp 拒绝非法 syscall

### 集成测试

- Node.js 调用沙箱工具，验证隔离性
- 恶意工具尝试逃逸（chroot, symlink）应失败

### 性能基准

- 沙箱启动开销（fork + namespaces）
- 工具执行延迟增加

---

## 🚨 安全考量

⚠️ **警告**：
- Namespaces 需要 Linux 3.10+
- 某些容器环境可能禁用了某些 namespaces（检查 `/proc/self/ns/`）
- 生产环境应启用所有可用安全特性
- 考虑 dropping capabilities（`CAP_NET_RAW`, `CAP_SYS_ADMIN` 等）

---

## 📅 时间估算

| Task | 预估（天） | 备注 |
|------|-----------|------|
| 3.1 路径规范化 | 0.5 | 小改 |
| 3.2 沙箱结构 | 1 | 核心 |
| 3.3 Namespaces | 2 | 最复杂 |
| 3.4 Seccomp | 1.5 | 策略调试 |
| 3.5 资源限制 | 1 | 相对简单 |
| 3.6 网络策略 | 1 | eBPF 可延后 |
| 3.7 Runtime 集成 | 1 | 测试+调试 |
| **Total** | **8** | 约 1.5 周 |

---

## 🔮 后续 Phase 关联

- Phase 4: 压缩算法需要沙箱环境（处理大上下文）
- Phase 5: 技能系统调用工具应自动通过沙箱

---

**Phase 3 负责人**：待分配  
**代码审查**：需要安全专家审计 seccomp 策略  
**文档更新**：ADDR-003 (Sandbox Design), Security Guide

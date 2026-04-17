# 热重载开发

热重载（Hot Reload）允许在运行时自动检测插件文件变化并重新加载，无需重启 OpenClaw。

## 启用热重载

CLI 支持 `--watch-plugins` 或配置文件中设置：

```bash
openclaw repl --watch-plugins --plugin ./plugins/libmy_plugin.so
```

或在配置文件中启用：

```toml
[hot_reload]
watch_dir = "./plugins"
debounce_ms = 500
auto_reload = true
```

## 工作流程

1. 启动 OpenClaw 开发服务器（REPL 或 daemon）
2. 编辑你的插件 Rust 代码
3. 重新编译插件：`cargo build --release` (覆盖 `.so` 文件)
4. 文件修改自动触发热重载，新版本插件生效

**注意**: 当前热重载仅支持**重新加载整个插件系统**，精细到单个插件的重载计划在未来版本中实现。

## 配置选项

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `watch_dir` | string | `"./plugins"` | 监控的插件目录 |
| `debounce_ms` | integer | `500` | 防抖延迟（毫秒） |
| `auto_reload` | boolean | `true` | 变化时自动重载 |

## 限制

- 热重载仅适用于**动态库插件**（`.so`/`.dll`）
- 重载会丢弃插件的状态（`state` 字段不会保留）
- 重载期间可能有短暂停顿（<100ms）
- 插件 ABI 必须保持兼容（结构体字段顺序一致）

## 手动重载

如需手动触发：

```bash
openclaw hot-reload
```

或在 REPL 中：

```
> /reload
```

## 开发建议

- 使用 `cargo watch -x "build --release"` 自动编译
- 保持插件 ABI 稳定：使用 `#[repr(C)]` 或仅通过 trait 交互
- 状态保存在 `HookContext.state` 而非插件结构体中
- 测试热重载后所有功能正常

---

下一步：[会话持久化](persistence.md)

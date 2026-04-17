# 会话持久化

OpenClaw 支持将会话历史保存到磁盘，实现断点续聊和多轮对话记忆。

## 启用持久化

在配置文件中设置 `session_store`：

```toml
session_store = "./sessions"
```

目录不存在时会自动创建。

## 会话存储格式

每个会话保存为单独的 JSON 文件：

```
sessions/
├── sess_001.json
├── sess_002.json
└── ...
```

**JSON 结构**:

```json
{
  "id": "sess_001",
  "created_at": "2026-04-05T02:00:00Z",
  "updated_at": "2026-04-05T02:10:00Z",
  "status": "active",
  "model": "claude-3-sonnet-20240229",
  "history": [
    [1, {"tool":"http_get","content":{...}}, "model response 1"],
    [2, {"tool":"calculator","content":{...}}, "model response 2"],
    ...
  ]
}
```

`history` 数组元素：`(turn_index, tool_output, model_output)`

## 自动保存

REPL 模式下，每次 turn 结束后自动保存（如果配置了 `session_store`）。

## 恢复会话

```bash
openclaw repl --resume sess_001
```

或在 REPL 内使用命令：

```
> /resume sess_001
```

会话状态（包括历史记录）会完整恢复。

## 会话管理

列出所有会话：

```bash
openclaw sessions list --store ./sessions
```

删除会话：

```bash
openclaw sessions delete sess_001 --store ./sessions
```

清理旧会话（按时间）：

```bash
openclaw sessions prune --older-than 30d --store ./sessions
```

## 编程接口

```rust
use openclaw_runtime::persistence::JsonFileSessionStore;

let store = JsonFileSessionStore::new("./sessions")?;
let session = Session::new("my-session").with_model("claude-3");
store.save(&session)?;

let loaded = store.load("my-session")?;
```

## 注意事项

- 会话文件包含完整历史，长期使用会占用磁盘空间，定期清理
- 敏感信息（如 API 响应）会持久化到磁盘，确保目录权限安全
- 不同 provider 的会话可能无法互相加载（模型不兼容）

---

下一步：[API 参考](api/runtime.md)

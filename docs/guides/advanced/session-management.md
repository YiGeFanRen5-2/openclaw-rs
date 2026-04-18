# 会话管理

本指南深入讲解 OpenClaw 的会话管理机制，包括会话生命周期、存储策略、并发控制和高级用法。

## 会话概述

会话（Session）是 OpenClaw 中管理对话上下文的核心概念。每个会话维护：

- **消息历史** - 用户与模型的完整对话记录
- **元数据** - 创建时间、更新时间、状态、使用的模型
- **状态** - active、paused、archived
- **上下文窗口** - 滑动窗口管理消息数量

## 会话生命周期

```
create_session()
    │
    ▼
┌─────────────────────────────────┐
│          active                 │
│  (正常对话，可工具调用)          │
└───────────────┬─────────────────┘
                │
    ┌───────────┼───────────┐
    ▼           ▼           ▼
 paused      archived    expired
 (手动暂停)   (归档)     (超时)
```

### 创建会话

```rust
use openclaw_runtime::{Runtime, SessionId};

// 方式 1: 自动创建
let session_id = runtime.create_session().await?;

// 方式 2: 带元数据创建
let session_id = runtime.create_session_with_meta(
    serde_json::json!({
        "user_id": "user-123",
        "channel": "api",
        "tags": ["support", "billing"]
    })
).await?;
```

### 获取会话

```rust
// 获取会话信息
let session = runtime.get_session(&session_id).await?;
println!("Created: {}", session.created_at);
println!("Messages: {}", session.message_count);

// 检查会话是否存在
if runtime.session_exists(&session_id).await? {
    // ...
}
```

### 更新会话状态

```rust
use openclaw_runtime::SessionStatus;

// 暂停会话
runtime.update_session_status(&session_id, SessionStatus::Paused).await?;

// 归档会话
runtime.update_session_status(&session_id, SessionStatus::Archived).await?;
```

### 删除会话

```rust
// 删除单个会话
runtime.delete_session(&session_id).await?;

// 批量删除
runtime.delete_sessions(&[
    session_id_1.clone(),
    session_id_2.clone(),
]).await?;
```

## 上下文窗口管理

### 滑动窗口

OpenClaw 使用滑动窗口管理上下文：

```toml
window_capacity = 20  # 保留最近 20 条消息
```

当消息超过窗口容量时，旧消息会被自动压缩或丢弃。

### 自定义窗口策略

```rust
use openclaw_runtime::{Runtime, WindowStrategy};

let config = RuntimeConfig::builder()
    .provider("anthropic")
    .model("claude-3-sonnet-20240229")
    .window_strategy(WindowStrategy::Sliding { capacity: 30 })
    .build()?;
```

### 窗口策略类型

| 策略 | 说明 |
|------|------|
| `Sliding` | 保留最近 N 条消息 |
| `Summarize` | 超过容量时自动摘要旧消息 |
| `KeepAll` | 保留所有消息（需足够上下文窗口） |

### 摘要策略

```rust
use openclaw_runtime::{Runtime, WindowStrategy, SummarizeConfig};

let config = RuntimeConfig::builder()
    .window_strategy(WindowStrategy::Summarize {
        capacity: 20,
        summary_model: "claude-3-haiku-20240307",
        threshold: 15,
    })
    .build()?;
```

## 会话存储

### 后端类型

#### 文件系统存储（默认）

```toml
session_store = "./sessions"
```

存储结构：

```
sessions/
├── index.json          # 会话索引
├── session-001.json    # 会话数据
├── session-002.json
└── ...
```

#### SQLite 存储

```rust
use openclaw_runtime::{Runtime, SqliteSessionStore};

let store = SqliteSessionStore::new("./openclaw.db")?;
let runtime = Runtime::with_session_store(store).await?;
```

#### Redis 存储（生产环境推荐）

```rust
use openclaw_runtime::{Runtime, RedisSessionStore};

let store = RedisSessionStore::new("redis://localhost:6379").await?;
let runtime = Runtime::with_session_store(store).await?;
```

### 自定义存储后端

```rust
use openclaw_runtime::{Session, SessionStore, SessionId};
use async_trait::async_trait;

#[derive(Clone)]
struct MySessionStore {
    // 你的存储字段
}

#[async_trait]
impl SessionStore for MySessionStore {
    async fn save(&self, session: &Session) -> Result<(), Box<dyn std::error::Error>> {
        // 保存逻辑
        Ok(())
    }

    async fn load(&self, id: &SessionId) -> Result<Option<Session>, Box<dyn std::error::Error>> {
        // 加载逻辑
        Ok(None)
    }

    async fn list(&self) -> Result<Vec<SessionId>, Box<dyn std::error::Error>> {
        // 列表逻辑
        Ok(vec![])
    }

    async fn delete(&self, id: &SessionId) -> Result<(), Box<dyn std::error::Error>> {
        // 删除逻辑
        Ok(())
    }
}
```

## 并发控制

### 单会话并发

默认情况下，每个会话同时只能有一个请求在处理中：

```rust
// 等待前一个请求完成
let response = runtime.chat_session(&session_id, "Hello").await?;
```

### 会话锁

```rust
use openclaw_runtime::SessionLock;

// 获取会话锁
let lock = runtime.lock_session(&session_id).await?;
let response = runtime.chat_session(&session_id, "Hello").await?;
drop(lock);  // 释放锁
```

### 并发会话处理

```rust
use tokio::task::JoinSet;

let mut set = JoinSet::new();

// 并发处理多个会话
for session_id in session_ids {
    set.spawn(async move {
        let response = runtime.chat_session(&session_id, "Status check").await;
        (session_id, response)
    });
}

while let Some(result) = set.join_next().await {
    let (session_id, response) = result?;
    println!("{}: {}", session_id, response);
}
```

## 会话搜索

### 全文搜索

```rust
use openclaw_runtime::SessionQuery;

// 搜索包含关键词的会话
let results = runtime.search_sessions(SessionQuery {
    query: "payment issue".to_string(),
    limit: 10,
    ..Default::default()
}).await?;

for hit in results {
    println!("Session: {} (score: {:.2})", hit.session_id, hit.score);
    println!("Snippet: {}", hit.snippet);
}
```

### 按条件过滤

```rust
let results = runtime.search_sessions(SessionQuery {
    query: "".to_string(),
    filters: serde_json::json!({
        "status": "active",
        "created_after": "2026-04-01T00:00:00Z",
        "tags": ["support"]
    }),
    limit: 50,
}).await?;
```

### 按时间范围查询

```rust
use chrono::{DateTime, Utc};

let recent = runtime.list_sessions_filtered(
    DateTime::parse_from_rfc3339("2026-04-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc),
    None,  // 无结束时间
).await?;
```

## 会话迁移

### 导出

```rust
use openclaw_runtime::SessionExporter;

// 导出所有会话
let exporter = SessionExporter::new(&runtime);
exporter.export_all("./backup-2026-04-18.zip").await?;

// 导出特定会话
exporter.export_sessions(&[&sid1, &sid2], "./partial.zip").await?;
```

### 导入

```rust
use openclaw_runtime::SessionImporter;

let importer = SessionImporter::new(&runtime);
importer.import("./backup-2026-04-18.zip").await?;
```

## 会话监控

### 获取统计信息

```rust
let stats = runtime.session_stats().await?;
println!("Total sessions: {}", stats.total);
println!("Active: {}", stats.active);
println!("Archived: {}", stats.archived);
println!("Total messages: {}", stats.total_messages);
```

### 活跃会话列表

```rust
let active = runtime.list_active_sessions().await?;
for session in active {
    println!("{} - {} messages, last: {}",
        session.id,
        session.message_count,
        session.updated_at
    );
}
```

### 监控事件

```rust
use openclaw_runtime::{RuntimeEvent, SessionEvent};

runtime.subscribe(|event| async move {
    match event {
        RuntimeEvent::SessionCreated(id) => {
            println!("New session: {}", id);
        }
        RuntimeEvent::SessionMessage(id, msg) => {
            println!("Message in {}: {:?}", id, msg);
        }
        RuntimeEvent::SessionClosed(id) => {
            println!("Session closed: {}", id);
        }
        _ => {}
    }
}).await?;
```

## 性能优化

### 会话缓存

```rust
use openclaw_runtime::{Runtime, SessionCacheConfig};

let config = RuntimeConfig::builder()
    .session_cache(SessionCacheConfig {
        max_entries: 1000,
        ttl_seconds: 3600,
    })
    .build()?;
```

### 批量操作

```rust
// 批量归档旧会话
let old_sessions: Vec<_> = runtime.list_sessions_filtered(
    DateTime::parse_from_rfc3339("2026-03-01T00:00:00Z").unwrap().with_timezone(&Utc),
    DateTime::parse_from_rfc3339("2026-04-01T00:00:00Z").unwrap().with_timezone(&Utc),
).await?;

runtime.batch_update_status(&old_sessions, SessionStatus::Archived).await?;
```

## 故障排除

### 会话丢失

```bash
# 检查存储目录权限
ls -la sessions/

# 修复索引
openclaw repair --session-store ./sessions
```

### 存储空间清理

```bash
# 清理归档会话
openclaw cleanup --session-store ./sessions --before 2026-01-01

# 压缩存储
openclaw compact --session-store ./sessions
```

## 下一步

- [插件开发](plugin-development.md)
- [自定义工具](custom-tools.md)
- [持久化配置](../../guide/persistence.md)

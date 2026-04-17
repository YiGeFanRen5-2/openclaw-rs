//! Runtime Core - 扩展工具支持
//!
//! 简化设计：将核心类型放在 lib.rs，避免复杂的模块边界问题
//! 后续再拆分为多个文件

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use thiserror::Error;
use tools::{Sandbox, Tool, ToolCall, ToolResult};

pub mod compression;
pub mod lsp;
pub mod provider;

/// Session ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

/// Message in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Session error
#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session error: {0}")]
    General(String),
    #[error("Tool error: {0}")]
    Tool(String),
}

/// Session - 存储对话历史
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub messages: Vec<Message>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Session {
    pub fn new(id: impl Into<SessionId>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            messages: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn id(&self) -> &SessionId {
        &self.id
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn add_message(&mut self, role: Role, content: String) -> Result<(), SessionError> {
        self.messages.push(Message {
            role,
            content,
            timestamp: Utc::now(),
        });
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn token_count(&self) -> usize {
        self.messages.iter().map(|m| m.content.len() / 4).sum()
    }

    pub fn should_compact(&self, max_tokens: usize) -> bool {
        self.token_count() > max_tokens
    }
}

#[derive(Debug, Clone)]
pub struct SummaryRequest {
    pub session_id: Option<SessionId>,
    pub messages: Vec<Message>,
    pub max_summary_chars: usize,
}

impl SummaryRequest {
    pub fn to_prompt(&self) -> String {
        let session = self
            .session_id
            .as_ref()
            .map(|s| s.as_str().to_string())
            .unwrap_or_else(|| "unknown-session".to_string());

        let mut prompt = format!(
            "Summarize the following conversation history for later context reuse. \
Keep key facts, decisions, requests, tool outcomes, and unresolved items. \
Be concise. Max summary chars target: {}. Session: {}.\n\n",
            self.max_summary_chars, session
        );

        for message in &self.messages {
            let role = match message.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => "system",
                Role::Tool => "tool",
            };
            prompt.push_str(&format!("[{}] {}\n", role, message.content));
        }

        prompt
    }
}

pub trait Summarizer: Send + Sync {
    fn summarize(&self, request: &SummaryRequest) -> String;
}

#[derive(Debug, Default)]
pub struct DeterministicSummarizer;

impl Summarizer for DeterministicSummarizer {
    fn summarize(&self, request: &SummaryRequest) -> String {
        let messages = &request.messages;
        let total = messages.len();
        let mut parts = Vec::new();

        for message in messages.iter().take(6) {
            let role = match message.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => "system",
                Role::Tool => "tool",
            };

            let snippet: String = if message.content.chars().count() > 80 {
                message.content.chars().take(80).collect::<String>() + "…"
            } else {
                message.content.clone()
            };
            parts.push(format!("[{}] {}", role, snippet.replace('\n', " ")));
        }

        let mut summary = format!(
            "Summary of {} earlier messages:\n{}",
            total,
            parts.join("\n")
        );
        if summary.chars().count() > request.max_summary_chars {
            summary = summary
                .chars()
                .take(request.max_summary_chars)
                .collect::<String>()
                + "…";
        }
        summary
    }
}

pub trait SummaryBackend: Send + Sync {
    fn generate_summary(&self, request: &SummaryRequest) -> Result<String, RuntimeError>;
}

pub trait SummaryPromptBackend: Send + Sync {
    fn generate_from_prompt(&self, prompt: &str) -> Result<String, RuntimeError>;
}

/// Type alias for the summary backend closure: takes a prompt, returns summary text.
pub type SummaryPromptFn = Box<dyn Fn(&str) -> Result<String, RuntimeError> + Send + Sync>;

pub trait ProviderGenerate: Send + Sync {
    fn generate(&self, prompt: &str) -> Result<String, RuntimeError>;
}

pub struct ClosureSummaryPromptBackend {
    inner: SummaryPromptFn,
}

impl ClosureSummaryPromptBackend {
    pub fn new(inner: SummaryPromptFn) -> Self {
        Self { inner }
    }
}

impl SummaryPromptBackend for ClosureSummaryPromptBackend {
    fn generate_from_prompt(&self, prompt: &str) -> Result<String, RuntimeError> {
        (self.inner)(prompt)
    }
}

pub struct ProviderGenerateBridge {
    inner: Box<dyn ProviderGenerate>,
}

impl ProviderGenerateBridge {
    pub fn new(inner: Box<dyn ProviderGenerate>) -> Self {
        Self { inner }
    }
}

impl SummaryPromptBackend for ProviderGenerateBridge {
    fn generate_from_prompt(&self, prompt: &str) -> Result<String, RuntimeError> {
        self.inner.generate(prompt)
    }
}

pub struct ProviderSummaryBackend {
    inner: Box<dyn SummaryPromptBackend>,
}

impl ProviderSummaryBackend {
    pub fn new(inner: Box<dyn SummaryPromptBackend>) -> Self {
        Self { inner }
    }

    pub fn from_closure(inner: SummaryPromptFn) -> Self {
        Self {
            inner: Box::new(ClosureSummaryPromptBackend::new(inner)),
        }
    }
}

impl SummaryBackend for ProviderSummaryBackend {
    fn generate_summary(&self, request: &SummaryRequest) -> Result<String, RuntimeError> {
        self.inner.generate_from_prompt(&request.to_prompt())
    }
}

#[derive(Debug, Default)]
pub struct FallbackSummaryBackend;

impl SummaryBackend for FallbackSummaryBackend {
    fn generate_summary(&self, request: &SummaryRequest) -> Result<String, RuntimeError> {
        Ok(DeterministicSummarizer.summarize(request))
    }
}

pub struct LlmSummarizer {
    backend: Box<dyn SummaryBackend>,
}

impl LlmSummarizer {
    pub fn new(backend: Box<dyn SummaryBackend>) -> Self {
        Self { backend }
    }
}

impl Default for LlmSummarizer {
    fn default() -> Self {
        Self {
            backend: Box::new(FallbackSummaryBackend),
        }
    }
}

impl Summarizer for LlmSummarizer {
    fn summarize(&self, request: &SummaryRequest) -> String {
        self.backend
            .generate_summary(request)
            .unwrap_or_else(|_| DeterministicSummarizer.summarize(request))
    }
}

/// Runtime 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub max_session_tokens: usize,
    pub compaction_threshold: f64,
    pub min_recent_messages: usize,
    pub persist_path: Option<std::path::PathBuf>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_session_tokens: 4000,
            compaction_threshold: 0.8,
            min_recent_messages: 4,
            persist_path: None,
        }
    }
}

impl RuntimeConfig {
    /// Set persist path (consumes self, returns new config)
    pub fn with_persist_path(mut self, path: Option<std::path::PathBuf>) -> Self {
        self.persist_path = path;
        self
    }
}

/// 运行时错误
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Runtime error: {0}")]
    General(String),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Session error: {0}")]
    Session(String),
}

impl From<SessionError> for RuntimeError {
    fn from(err: SessionError) -> Self {
        RuntimeError::Session(err.to_string())
    }
}

/// OpenClaw 运行时
pub struct Runtime {
    config: RuntimeConfig,
    sessions: HashMap<SessionId, Session>,
    tools: HashMap<String, Box<dyn Tool>>,
    sandbox: Sandbox,
    summarizer: Box<dyn Summarizer>,
}

impl Runtime {
    pub fn new(config: RuntimeConfig) -> Result<Self, RuntimeError> {
        let mut runtime = Self {
            config,
            sessions: HashMap::new(),
            tools: HashMap::new(),
            sandbox: Sandbox::new(),
            summarizer: Box::new(DeterministicSummarizer),
        };

        // 注册内置工具
        runtime.register_tool(Box::new(tools::ListFilesTool::new()))?;
        runtime.register_tool(Box::new(tools::ReadFileTool::new()))?;

        Ok(runtime)
    }

    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Set persist path for session storage
    pub fn set_persist_path(&mut self, path: Option<std::path::PathBuf>) {
        self.config.persist_path = path;
    }

    pub fn with_summarizer(mut self, summarizer: Box<dyn Summarizer>) -> Self {
        self.summarizer = summarizer;
        self
    }

    pub fn set_summarizer(&mut self, summarizer: Box<dyn Summarizer>) {
        self.summarizer = summarizer;
    }

    /// 创建新会话
    pub fn create_session(&mut self, id: impl Into<SessionId>) -> Result<SessionId, RuntimeError> {
        let session = Session::new(id);
        let session_id = session.id().clone();
        self.sessions
            .entry(session_id.clone())
            .or_insert_with(|| session);
        Ok(session_id)
    }

    /// 获取会话
    pub fn get_session(&self, id: &SessionId) -> Result<&Session, RuntimeError> {
        self.sessions
            .get(id)
            .ok_or_else(|| RuntimeError::SessionNotFound(id.as_str().to_string()))
    }

    /// 获取会话（可变）
    pub fn get_session_mut(&mut self, id: &SessionId) -> Result<&mut Session, RuntimeError> {
        self.sessions
            .get_mut(id)
            .ok_or_else(|| RuntimeError::SessionNotFound(id.as_str().to_string()))
    }

    /// 删除会话
    pub fn delete_session(&mut self, id: &SessionId) -> Result<(), RuntimeError> {
        self.sessions
            .remove(id)
            .ok_or_else(|| RuntimeError::SessionNotFound(id.as_str().to_string()))?;
        Ok(())
    }

    /// 列出所有会话 ID
    pub fn list_sessions(&self) -> Vec<SessionId> {
        self.sessions.keys().cloned().collect()
    }

    /// 注册工具
    pub fn register_tool(&mut self, tool: Box<dyn Tool>) -> Result<(), RuntimeError> {
        let name = tool.name().to_string();
        if self.tools.contains_key(&name) {
            return Err(RuntimeError::General(format!(
                "Tool '{}' already registered",
                name
            )));
        }
        self.tools.insert(name, tool);
        Ok(())
    }

    /// 获取工具
    pub fn get_tool(&self, name: &str) -> Result<&dyn Tool, RuntimeError> {
        self.tools
            .get(name)
            .map(|boxed| boxed.as_ref())
            .ok_or_else(|| RuntimeError::General(format!("Tool '{}' not found", name)))
    }

    /// 列出所有工具
    pub fn list_tools(&self) -> Vec<&dyn Tool> {
        self.tools.values().map(|boxed| boxed.as_ref()).collect()
    }

    /// 执行工具（通过沙箱）
    pub fn execute_tool(
        &mut self,
        session_id: &SessionId,
        call: ToolCall,
    ) -> Result<ToolResult, RuntimeError> {
        // 1. 获取工具（不可借用 self.tools）
        let tool = {
            let tool_name = call.name.clone();
            self.get_tool(&tool_name)?
        };

        // 2. 解析参数（独立，不借用 self）
        let args: serde_json::Value = serde_json::from_str(&call.arguments)
            .map_err(|e| RuntimeError::General(format!("Invalid JSON arguments: {}", e)))?;

        // 3. 通过沙箱执行工具（借用 sandbox 和 tool）
        let result = self
            .sandbox
            .execute(tool, args.clone())
            .map_err(|e| RuntimeError::General(format!("Tool execution failed: {}", e)))?;

        // 4. 记录工具调用到会话历史（需要 mutable borrow of session）
        let content = result.to_string();
        let session = self.get_session_mut(session_id)?;
        session
            .add_message(Role::Tool, content.clone())
            .map_err(|e| RuntimeError::Session(e.to_string()))?;

        Ok(ToolResult {
            content,
            error: None,
        })
    }

    /// 压缩会话：保留最近 N 条消息，并把更早消息折叠为一条系统摘要
    pub fn compact_session(&mut self, session_id: &SessionId) -> Result<(), RuntimeError> {
        let max_tokens = self.config.max_session_tokens;
        let threshold = ((max_tokens as f64) * self.config.compaction_threshold).ceil() as usize;
        let min_recent = self.config.min_recent_messages.max(1);

        let (older_messages, recent_messages) = {
            let session = self.get_session(session_id)?;

            if session.token_count() <= threshold || session.messages.len() <= min_recent {
                return Ok(());
            }

            let split_index = session.messages.len().saturating_sub(min_recent);
            if split_index == 0 {
                return Ok(());
            }

            (
                session.messages[..split_index].to_vec(),
                session.messages[split_index..].to_vec(),
            )
        };

        let summary = self.summarizer.summarize(&SummaryRequest {
            session_id: Some(session_id.clone()),
            messages: older_messages,
            max_summary_chars: 512,
        });
        let session = self.get_session_mut(session_id)?;
        session.messages = vec![Message {
            role: Role::System,
            content: summary,
            timestamp: Utc::now(),
        }];
        session.messages.extend(recent_messages);

        while session.should_compact(max_tokens) && session.messages.len() > 1 {
            session.messages.remove(1);
        }

        session.updated_at = Utc::now();
        Ok(())
    }

    /// 持久化会话（写入 JSON 文件，格式：<persist_path>/<session_id>.json）
    pub fn persist_session(&self, session_id: &SessionId) -> Result<(), RuntimeError> {
        if let Some(base_path) = &self.config.persist_path {
            fs::create_dir_all(base_path)
                .map_err(|e| RuntimeError::General(format!("mkdir failed: {}", e)))?;
            let file_path = base_path.join(format!("{}.json", session_id.as_str()));
            let session = self.get_session(session_id)?;
            let data = serde_json::to_string_pretty(session)
                .map_err(|e| RuntimeError::General(format!("serialize failed: {}", e)))?;
            fs::write(&file_path, data)
                .map_err(|e| RuntimeError::General(format!("write failed: {}", e)))?;
        }
        Ok(())
    }

    /// 恢复会话（从 JSON 文件加载并替换内存中的对应会话）
    pub fn restore_session(&mut self, session_id: &SessionId) -> Result<(), RuntimeError> {
        if let Some(base_path) = &self.config.persist_path {
            let file_path = base_path.join(format!("{}.json", session_id.as_str()));
            if !file_path.exists() {
                return Err(RuntimeError::SessionNotFound(
                    session_id.as_str().to_string(),
                ));
            }
            let data = fs::read_to_string(&file_path)
                .map_err(|e| RuntimeError::General(format!("read failed: {}", e)))?;
            let session: Session = serde_json::from_str(&data)
                .map_err(|e| RuntimeError::General(format!("deserialize failed: {}", e)))?;
            // 覆盖（或插入）到 sessions map
            self.sessions.insert(session_id.clone(), session);
        }
        Ok(())
    }
}

/// 运行时启动器（创建默认配置的运行时）
pub fn create_runtime(config: RuntimeConfig) -> Result<Runtime, RuntimeError> {
    Runtime::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedSummarizer;

    impl Summarizer for FixedSummarizer {
        fn summarize(&self, request: &SummaryRequest) -> String {
            format!("CUSTOM SUMMARY [{}]", request.messages.len())
        }
    }

    struct MockSummaryBackend;

    impl SummaryBackend for MockSummaryBackend {
        fn generate_summary(&self, request: &SummaryRequest) -> Result<String, RuntimeError> {
            Ok(format!("LLM SUMMARY [{}]", request.messages.len()))
        }
    }

    struct CapturingPromptBackend;

    impl SummaryPromptBackend for CapturingPromptBackend {
        fn generate_from_prompt(&self, prompt: &str) -> Result<String, RuntimeError> {
            Ok(format!(
                "PROMPT_BACKEND: {}",
                prompt.lines().next().unwrap_or("")
            ))
        }
    }

    struct MockGenerateProvider;

    impl ProviderGenerate for MockGenerateProvider {
        fn generate(&self, prompt: &str) -> Result<String, RuntimeError> {
            Ok(format!(
                "PROVIDER_GENERATE: {}",
                prompt.lines().next().unwrap_or("")
            ))
        }
    }

    #[test]
    fn test_compact_session_replaces_old_messages_with_summary_and_keeps_recent() {
        let mut runtime = Runtime::new(RuntimeConfig {
            max_session_tokens: 100,
            compaction_threshold: 0.5,
            min_recent_messages: 2,
            persist_path: None,
        })
        .unwrap();

        let session_id = runtime.create_session("compact-test").unwrap();
        let session = runtime.get_session_mut(&session_id).unwrap();

        for i in 0..6 {
            session
                .add_message(
                    Role::User,
                    format!(
                        "long message {} with many characters to increase token count",
                        i
                    ),
                )
                .unwrap();
        }

        runtime.compact_session(&session_id).unwrap();

        let session_after = runtime.get_session(&session_id).unwrap();
        assert!(!session_after.messages.is_empty());
        assert!(matches!(session_after.messages[0].role, Role::System));
        assert!(session_after.messages[0].content.contains("Summary of"));
        assert!(
            session_after
                .messages
                .iter()
                .any(|m| m.content.contains("message 4"))
                || session_after
                    .messages
                    .iter()
                    .any(|m| m.content.contains("message 5"))
        );
        assert!(
            session_after.messages.len() <= 3,
            "expected summary + recent messages only"
        );
    }

    #[test]
    fn test_runtime_can_use_custom_summarizer() {
        let mut runtime = Runtime::new(RuntimeConfig {
            max_session_tokens: 100,
            compaction_threshold: 0.5,
            min_recent_messages: 2,
            persist_path: None,
        })
        .unwrap()
        .with_summarizer(Box::new(FixedSummarizer));

        let session_id = runtime.create_session("custom-summary-test").unwrap();
        let session = runtime.get_session_mut(&session_id).unwrap();
        for i in 0..6 {
            session
                .add_message(
                    Role::User,
                    format!("message {} with enough content to compact", i),
                )
                .unwrap();
        }

        runtime.compact_session(&session_id).unwrap();
        let session_after = runtime.get_session(&session_id).unwrap();
        assert!(matches!(session_after.messages[0].role, Role::System));
        assert_eq!(session_after.messages[0].content, "CUSTOM SUMMARY [4]");
    }

    #[test]
    fn test_summary_request_to_prompt_contains_context() {
        let request = SummaryRequest {
            session_id: Some(SessionId::from("prompt-test")),
            messages: vec![
                Message {
                    role: Role::User,
                    content: "hello".into(),
                    timestamp: Utc::now(),
                },
                Message {
                    role: Role::Tool,
                    content: "tool-result".into(),
                    timestamp: Utc::now(),
                },
            ],
            max_summary_chars: 128,
        };
        let prompt = request.to_prompt();
        assert!(prompt.contains("Session: prompt-test"));
        assert!(prompt.contains("[user] hello"));
        assert!(prompt.contains("[tool] tool-result"));
    }

    #[test]
    fn test_provider_summary_backend_uses_prompt() {
        let backend = ProviderSummaryBackend::new(Box::new(CapturingPromptBackend));
        let output = backend
            .generate_summary(&SummaryRequest {
                session_id: Some(SessionId::from("provider-summary-test")),
                messages: vec![Message {
                    role: Role::User,
                    content: "hello provider".into(),
                    timestamp: Utc::now(),
                }],
                max_summary_chars: 128,
            })
            .unwrap();
        assert!(output.starts_with("PROMPT_BACKEND: Summarize the following conversation history"));
    }

    #[test]
    fn test_provider_summary_backend_from_closure() {
        let backend = ProviderSummaryBackend::from_closure(Box::new(|prompt| {
            Ok(format!("CLOSURE_BACKEND: {}", prompt))
        }));
        let output = backend
            .generate_summary(&SummaryRequest {
                session_id: Some(SessionId::from("closure-summary-test")),
                messages: vec![Message {
                    role: Role::User,
                    content: "hello from closure".into(),
                    timestamp: Utc::now(),
                }],
                max_summary_chars: 128,
            })
            .unwrap();
        assert!(output.contains("Session: closure-summary-test"));
        assert!(output.contains("[user] hello from closure"));
    }

    #[test]
    fn test_provider_generate_bridge_uses_provider_style_generate() {
        let backend = ProviderSummaryBackend::new(Box::new(ProviderGenerateBridge::new(Box::new(
            MockGenerateProvider,
        ))));
        let output = backend
            .generate_summary(&SummaryRequest {
                session_id: Some(SessionId::from("provider-generate-test")),
                messages: vec![Message {
                    role: Role::User,
                    content: "hello from provider bridge".into(),
                    timestamp: Utc::now(),
                }],
                max_summary_chars: 128,
            })
            .unwrap();
        assert!(
            output.starts_with("PROVIDER_GENERATE: Summarize the following conversation history")
        );
    }

    #[test]
    fn test_llm_summarizer_uses_backend() {
        let summarizer = LlmSummarizer::new(Box::new(MockSummaryBackend));
        let output = summarizer.summarize(&SummaryRequest {
            session_id: Some(SessionId::from("llm-summary-test")),
            messages: vec![
                Message {
                    role: Role::User,
                    content: "a".into(),
                    timestamp: Utc::now(),
                },
                Message {
                    role: Role::Assistant,
                    content: "b".into(),
                    timestamp: Utc::now(),
                },
                Message {
                    role: Role::Tool,
                    content: "c".into(),
                    timestamp: Utc::now(),
                },
            ],
            max_summary_chars: 128,
        });
        assert_eq!(output, "LLM SUMMARY [3]");
    }

    #[test]
    fn test_compacted_session_persists_and_restores_summary() {
        let dir = tempfile::tempdir().unwrap();
        let persist_path = dir.path().to_path_buf();

        let mut runtime = Runtime::new(RuntimeConfig {
            max_session_tokens: 100,
            compaction_threshold: 0.5,
            min_recent_messages: 2,
            persist_path: Some(persist_path.clone()),
        })
        .unwrap();

        let session_id = runtime.create_session("summary-persist-test").unwrap();
        {
            let session = runtime.get_session_mut(&session_id).unwrap();
            for i in 0..6 {
                session
                    .add_message(
                        Role::User,
                        format!(
                            "long message {} with many characters to increase token count",
                            i
                        ),
                    )
                    .unwrap();
            }
        }

        runtime.compact_session(&session_id).unwrap();
        runtime.persist_session(&session_id).unwrap();

        let mut runtime2 = Runtime::new(RuntimeConfig {
            max_session_tokens: 100,
            compaction_threshold: 0.5,
            min_recent_messages: 2,
            persist_path: Some(persist_path),
        })
        .unwrap();
        runtime2.restore_session(&session_id).unwrap();

        let restored = runtime2.get_session(&session_id).unwrap();
        assert!(matches!(restored.messages[0].role, Role::System));
        assert!(restored.messages[0].content.contains("Summary of"));
        assert!(restored.messages.len() <= 3);
    }

    #[test]
    fn test_persist_and_restore_session() {
        let dir = tempfile::tempdir().unwrap();
        let persist_path = dir.path().to_path_buf();

        let config = RuntimeConfig {
            max_session_tokens: 4000,
            compaction_threshold: 0.8,
            min_recent_messages: 4,
            persist_path: Some(persist_path.clone()),
        };
        let mut runtime = Runtime::new(config).unwrap();

        let session_id = runtime.create_session("persist-test").unwrap();
        {
            let session = runtime.get_session_mut(&session_id).unwrap();
            session
                .add_message(Role::User, "hello".to_string())
                .unwrap();
            session
                .add_message(Role::Assistant, "hi".to_string())
                .unwrap();
        }

        // Persist
        runtime.persist_session(&session_id).unwrap();

        // New runtime restore
        let mut runtime2 = Runtime::new(RuntimeConfig {
            max_session_tokens: 4000,
            compaction_threshold: 0.8,
            min_recent_messages: 4,
            persist_path: Some(persist_path),
        })
        .unwrap();
        runtime2.restore_session(&session_id).unwrap();

        let restored = runtime2.get_session(&session_id).unwrap();
        assert_eq!(restored.id.as_str(), "persist-test");
        assert_eq!(restored.messages.len(), 2);
        assert_eq!(restored.messages[0].content, "hello");
        assert_eq!(restored.messages[1].content, "hi");
    }
}

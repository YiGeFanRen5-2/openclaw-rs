//! Example 09: Batch Processing
//!
//! This example demonstrates how to process multiple tasks concurrently:
//! - Concurrent tool execution using tokio::spawn
//! - Session-based batch processing
//! - Progress tracking with atomic counters
//! - Result aggregation with error handling
//! - Configurable concurrency limits
//! - Timeout per task
//!
//! Run with: cargo run --example example_09_batch_processor

use runtime::{create_runtime, RuntimeConfig, Runtime};
use runtime::SessionId;
use tools::ToolCall;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{Semaphore, Barrier};
use tokio::time::Duration;

// ─── Task Types ───────────────────────────────────────────────────────────────

/// A single work item in a batch.
#[derive(Debug, Clone)]
struct BatchTask {
    pub id: usize,
    pub tool_name: String,
    pub args: serde_json::Value,
    pub timeout_secs: u64,
}

/// The result of a single task execution.
#[derive(Debug)]
struct TaskResult {
    pub task_id: usize,
    pub success: bool,
    pub output: String,
    pub duration_ms: u64,
    pub error: Option<String>,
}

/// Summary of the entire batch run.
#[derive(Debug)]
struct BatchSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub total_duration_ms: u64,
    pub results: Vec<TaskResult>,
}

// ─── Batch Processor ─────────────────────────────────────────────────────────

struct BatchProcessor {
    /// Runtime protected by Mutex for interior mutability across concurrent tasks.
    runtime: Arc<Mutex<Runtime>>,
    concurrency: usize,
}

impl BatchProcessor {
    fn new(runtime: Runtime, concurrency: usize) -> Self {
        Self {
            runtime: Arc::new(Mutex::new(runtime)),
            concurrency,
        }
    }

    /// Process a batch of tasks with configurable concurrency.
    async fn process_batch(&self, tasks: Vec<BatchTask>) -> BatchSummary {
        let total = tasks.len();
        let start = std::time::Instant::now();

        // Semaphore to limit concurrency
        let semaphore = Arc::new(Semaphore::new(self.concurrency));

        // Shared counters
        let succeeded = Arc::new(AtomicUsize::new(0));
        let failed = Arc::new(AtomicUsize::new(0));

        // Barrier so we can wait for all tasks to complete
        let barrier = Arc::new(Barrier::new(total));

        // Progress counter
        let completed = Arc::new(AtomicUsize::new(0));
        let total_arc = Arc::new(AtomicUsize::new(total));

        // Spawn a task for each work item
        let handles: Vec<_> = tasks
            .into_iter()
            .map(|task| {
                let runtime = Arc::clone(&self.runtime);
                let sem = Arc::clone(&semaphore);
                let barrier = Arc::clone(&barrier);
                let succeeded = Arc::clone(&succeeded);
                let failed = Arc::clone(&failed);
                let completed = Arc::clone(&completed);
                let total_arc = Arc::clone(&total_arc);

                tokio::spawn(async move {
                    // Acquire a permit (limits concurrency)
                    let _permit = sem.acquire().await.unwrap();

                    let task_start = std::time::Instant::now();
                    let task_id = task.id;

                    // Execute with timeout
                    let result = tokio::time::timeout(
                        Duration::from_secs(task.timeout_secs),
                        execute_single_task(runtime, task),
                    )
                    .await;

                    let duration_ms = task_start.elapsed().as_millis() as u64;

                    let task_result = match result {
                        Ok(Ok(r)) => {
                            succeeded.fetch_add(1, Ordering::Relaxed);
                            TaskResult { task_id, success: true, output: r, duration_ms, error: None }
                        }
                        Ok(Err(e)) => {
                            failed.fetch_add(1, Ordering::Relaxed);
                            TaskResult { task_id, success: false, output: String::new(), duration_ms, error: Some(e) }
                        }
                        Err(_) => {
                            failed.fetch_add(1, Ordering::Relaxed);
                            TaskResult { task_id, success: false, output: String::new(), duration_ms, error: Some("timeout".into()) }
                        }
                    };

                    // Update progress
                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    let total = total_arc.load(Ordering::Relaxed);
                    if done % 5 == 0 || done == total {
                        let pct = (done as f64 / total as f64 * 100.0) as u32;
                        println!("   Progress: {}/{} ({}%)", done, total, pct);
                    }

                    // Wait at barrier (all tasks reach here before we exit)
                    barrier.wait().await;

                    task_result
                })
            })
            .collect();

        // Wait for all tasks
        let mut results = Vec::with_capacity(total);
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }

        // Sort by task_id for consistent output
        results.sort_by_key(|r| r.task_id);

        let total_duration_ms = start.elapsed().as_millis() as u64;

        BatchSummary {
            total,
            succeeded: succeeded.load(Ordering::Relaxed),
            failed: failed.load(Ordering::Relaxed),
            total_duration_ms,
            results,
        }
    }
}

/// Execute a single task using the shared Runtime (protected by Mutex).
async fn execute_single_task(
    runtime: Arc<Mutex<Runtime>>,
    task: BatchTask,
) -> Result<String, String> {
    let session_id = {
        // Create session with lock
        let rt = runtime.lock().map_err(|e| format!("lock poisoned: {}", e))?;
        rt.create_session(format!("batch-task-{}", task.id))
            .map_err(|e| e.to_string())?
    };

    let call = ToolCall {
        name: task.tool_name.clone(),
        arguments: serde_json::to_string(&task.args).map_err(|e| e.to_string())?,
    };

    // Execute tool with lock
    let rt = runtime.lock().map_err(|e| format!("lock poisoned: {}", e))?;
    rt.execute_tool(&session_id, call)
        .map(|r| r.to_string())
        .map_err(|e| e.to_string())
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== OpenClaw Batch Processing Example ===\n");

    // 1. Setup runtime
    println!("1. Setup Runtime...");
    let config = RuntimeConfig::default();
    let runtime = create_runtime(config)?;
    println!("   ✓ Runtime ready\n");

    // 2. Create batch processor with concurrency limit
    println!("2. Create BatchProcessor (concurrency=4)...\n");
    let processor = BatchProcessor::new(runtime, 4);

    // ── Multiple text_stats tasks ─────────────────────────────────────────────
    println!("=== Batch: Multiple text_stats calls ===");

    let texts = [
        ("Hello world", "greeting"),
        ("The quick brown fox jumps over the lazy dog", "pangram"),
        ("Rust is a systems programming language", "rust-desc"),
        ("OpenClaw is an agent runtime", "openclaw-desc"),
        ("Async programming with tokio", "async-desc"),
        ("Concurrent batch processing example", "batch-desc"),
        ("Tokio spawn and semaphores", "concurrency-desc"),
        ("Tool execution in sandbox", "sandbox-desc"),
        ("Session management and persistence", "session-desc"),
        ("Plugin lifecycle and hooks", "plugin-desc"),
        ("HTTP client with retry logic", "http-desc"),
        ("JSON parsing and validation", "json-desc"),
    ];

    let tasks: Vec<BatchTask> = texts
        .iter()
        .enumerate()
        .map(|(i, (text, _))| BatchTask {
            id: i,
            tool_name: "text_stats".to_string(),
            args: serde_json::json!({ "text": text }),
            timeout_secs: 10,
        })
        .collect();

    println!("3. Processing {} tasks...\n", tasks.len());
    let summary = processor.process_batch(tasks).await;
    println!();

    // 4. Print summary
    println!("=== Batch Summary ===");
    println!("   Total tasks:    {}", summary.total);
    println!("   Succeeded:      {} ✓", summary.succeeded);
    println!("   Failed:          {} ✗", summary.failed);
    println!("   Total duration: {} ms", summary.total_duration_ms);
    println!("   Throughput:     {:.1} tasks/sec",
        summary.total as f64 / (summary.total_duration_ms as f64 / 1000.0));
    println!();

    // 5. Print individual results
    println!("=== Task Results ===");
    for result in &summary.results {
        let status = if result.success { "✓" } else { "✗" };
        let preview = if result.output.len() > 80 {
            format!("{}...", &result.output[..80])
        } else {
            result.output.clone()
        };
        let error_info = result.error.as_ref().map(|e| format!(" [ERROR: {}]", e)).unwrap_or_default();
        println!("   [{}] Task {}: {}{}", status, result.task_id, preview, error_info);
    }
    println!();

    // ── Mixed tool types ─────────────────────────────────────────────────────
    println!("=== Batch: Mixed tool types ===");

    let mixed_tasks = vec![
        BatchTask { id: 100, tool_name: "uuid".to_string(), args: serde_json::json!({}), timeout_secs: 5 },
        BatchTask { id: 101, tool_name: "text_stats".to_string(), args: serde_json::json!({ "text": "Generate UUID and stats" }), timeout_secs: 5 },
        BatchTask { id: 102, tool_name: "hash".to_string(), args: serde_json::json!({ "algorithm": "sha256", "data": "batch demo" }), timeout_secs: 5 },
        BatchTask { id: 103, tool_name: "random_string".to_string(), args: serde_json::json!({ "length": 32 }), timeout_secs: 5 },
    ];

    println!("4. Processing {} mixed tool tasks...\n", mixed_tasks.len());
    let mixed_summary = processor.process_batch(mixed_tasks).await;
    println!();

    println!("   Mixed batch: {}/{} succeeded in {} ms",
        mixed_summary.succeeded, mixed_summary.total, mixed_summary.total_duration_ms);
    for result in &mixed_summary.results {
        let status = if result.success { "✓" } else { "✗" };
        let preview = if result.output.len() > 80 {
            format!("{}...", &result.output[..80])
        } else {
            result.output.clone()
        };
        println!("   [{}] Task {}: {}", status, result.task_id, preview);
    }
    println!();

    // ── Error handling batch ─────────────────────────────────────────────────
    println!("=== Batch: Error handling ===");

    let error_tasks = vec![
        BatchTask { id: 200, tool_name: "text_stats".to_string(), args: serde_json::json!({ "text": "valid input" }), timeout_secs: 5 },
        BatchTask { id: 201, tool_name: "nonexistent_tool".to_string(), args: serde_json::json!({}), timeout_secs: 5 },
        BatchTask { id: 202, tool_name: "text_stats".to_string(), args: serde_json::json!({ "text": "another valid" }), timeout_secs: 5 },
    ];

    println!("5. Processing 3 tasks (1 intentionally invalid)...\n");
    let error_summary = processor.process_batch(error_tasks).await;
    println!();

    println!("   Error batch: {}/{} succeeded",
        error_summary.succeeded, error_summary.total);
    for result in &error_summary.results {
        let status = if result.success { "✓" } else { "✗" };
        let info = result.error.as_ref().map(|s| s.as_str()).unwrap_or(result.output.as_str());
        println!("   [{}] Task {}: {}", status, result.task_id, info);
    }
    println!();

    println!("=== Batch Processing Example Complete ===");
    println!("Demonstrated: concurrent execution, semaphore-based concurrency limits,");
    println!("progress tracking, timeout handling, error aggregation.");
    Ok(())
}

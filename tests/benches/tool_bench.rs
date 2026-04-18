//! Tool execution benchmarks.
//!
//! Performance benchmarks for tool creation, execution,
//! parameter handling, and concurrent tool operations.

use std::sync::Arc;
use tokio::sync::RwLock;
use openclaw_integration_tests::common::TestTool;

/// Benchmark runtime setup
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
}

/// Simple async function for benchmarks
async fn noop() {}

/// Benchmark: Tool creation
fn bench_tool_create(c: &mut criterion::Criterion) {
    c.bench_function("tool_create_single", |b| {
        b.iter(|| {
            let tool = TestTool::new("benchmark-tool");
            criterion::black_box(tool);
        });
    });
}

/// Benchmark: Tool creation batch
fn bench_tool_create_batch(c: &mut criterion::Criterion) {
    c.bench_function("tool_create_batch_100", |b| {
        b.iter(|| {
            let tools: Vec<_> = (0..100)
                .map(|i| TestTool::new(format!("tool-{}", i)))
                .collect();
            criterion::black_box(tools);
        });
    });
}

/// Benchmark: Tool execution (single)
fn bench_tool_execute(c: &mut criterion::Criterion) {
    let rt = runtime();
    let tool = TestTool::new("exec-tool");
    let input = serde_json::json!({"data": "test"});

    c.bench_function("tool_execute_single", |b| {
        b.to_async(rt).iter(async {
            let result = tool.execute(input.clone()).await;
            criterion::black_box(result);
        });
    });
}

/// Benchmark: Tool execution batch
fn bench_tool_execute_batch(c: &mut criterion::Criterion) {
    let rt = runtime();
    let tool = TestTool::new("batch-exec-tool");
    let input = serde_json::json!({"data": "test"});

    c.bench_function("tool_execute_batch_50", |b| {
        b.to_async(rt).iter(async {
            let handles: Vec<_> = (0..50)
                .map(|_| {
                    let tool = tool.clone();
                    let input = input.clone();
                    async move {
                        tool.execute(input).await
                    }
                })
                .collect();

            futures::future::join_all(handles).await;
        });
    });
}

/// Benchmark: Tool execution - concurrent
fn bench_tool_execute_concurrent(c: &mut criterion::Criterion) {
    let rt = runtime();
    let tool = TestTool::new("concurrent-exec-tool");
    let input = serde_json::json!({"data": "test"});

    c.bench_function("tool_execute_concurrent_100", |b| {
        b.to_async(rt).iter(async {
            let handles: Vec<_> = (0..100)
                .map(|_| {
                    let tool = tool.clone();
                    let input = input.clone();
                    rt.spawn(async move {
                        tool.execute(input).await
                    })
                })
                .collect();

            for handle in handles {
                let _ = handle.await;
            }
        });
    });
}

/// Benchmark: Tool invocation count read
fn bench_invocation_count_read(c: &mut criterion::Criterion) {
    let rt = runtime();
    let tool = TestTool::new("count-tool");

    // Pre-invoke tool
    rt.block_on(async {
        for _ in 0..100 {
            let _ = tool.execute(serde_json::json!({})).await;
        }
    });

    c.bench_function("invocation_count_read", |b| {
        b.to_async(rt).iter(async {
            let count = tool.invocation_count().await;
            criterion::black_box(count);
        });
    });
}

/// Benchmark: JSON parameter parsing
fn bench_json_parse(c: &mut criterion::Criterion) {
    let json_str = r#"{"name":"test","params":{"a":1,"b":"hello","c":[1,2,3]}}"#;

    c.bench_function("json_parse_tool_params", |b| {
        b.iter(|| {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(json_str);
            criterion::black_box(parsed);
        });
    });
}

/// Benchmark: JSON result serialization
fn bench_json_serialize(c: &mut criterion::Criterion) {
    let result = serde_json::json!({
        "tool_id": "benchmark-tool",
        "status": "success",
        "output": {
            "data": "result",
            "metadata": {"key": "value"}
        },
        "duration_ms": 42
    });

    c.bench_function("json_serialize_tool_result", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&result).unwrap();
            criterion::black_box(json);
        });
    });
}

/// Benchmark: Tool clone cost
fn bench_tool_clone(c: &mut criterion::Criterion) {
    let tool = TestTool::new("clone-source");

    c.bench_function("tool_clone", |b| {
        b.iter(|| {
            let cloned = tool.clone();
            criterion::black_box(cloned);
        });
    });
}

/// Benchmark: Tool with large input
fn bench_tool_large_input(c: &mut criterion::Criterion) {
    let rt = runtime();
    let tool = TestTool::new("large-input-tool");
    let large_input = serde_json::json!({
        "data": {
            "items": (0..1000).map(|i| serde_json::json!({"id": i, "value": format!("item-{}", i)})).collect::<Vec<_>>(),
            "metadata": "x".repeat(10000)
        }
    });

    c.bench_function("tool_execute_large_input", |b| {
        b.to_async(rt).iter(async {
            let result = tool.execute(large_input.clone()).await;
            criterion::black_box(result);
        });
    });
}

/// Benchmark: Multiple tools selection
fn bench_tool_selection(c: &mut criterion::Criterion) {
    let rt = runtime();
    let tools: Vec<TestTool> = (0..50)
        .map(|i| TestTool::new(format!("select-tool-{}", i)))
        .collect();

    c.bench_function("tool_selection_50", |b| {
        b.to_async(rt).iter(async {
            // Simulate selecting and executing a random tool
            for _ in 0..100 {
                let idx = rand_index(tools.len());
                let tool = &tools[idx];
                let _ = tool.execute(serde_json::json!({})).await;
            }
        });
    });
}

/// Simple random index for benchmarks
fn rand_index(max: usize) -> usize {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as usize;
    nanos % max
}

criterion::criterion_group!(
    tool_benches,
    bench_tool_create,
    bench_tool_create_batch,
    bench_tool_execute,
    bench_tool_execute_batch,
    bench_tool_execute_concurrent,
    bench_invocation_count_read,
    bench_json_parse,
    bench_json_serialize,
    bench_tool_clone,
    bench_tool_large_input,
    bench_tool_selection
);
criterion::criterion_main!(tool_benches);

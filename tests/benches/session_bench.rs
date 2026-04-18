//! Session management benchmarks.
//!
//! Performance benchmarks for session creation, state updates,
//! and concurrent session handling.

use std::sync::Arc;
use tokio::sync::RwLock;
use openclaw_integration_tests::common::test_session_id;

/// Benchmark runtime setup
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
}

/// Session state for benchmarks
#[derive(Debug, Clone)]
struct SessionBenchState {
    id: uuid::Uuid,
    data: serde_json::Value,
    created_at: std::time::Instant,
}

/// Benchmark: Single session creation
fn bench_session_create(c: &mut criterion::Criterion) {
    c.bench_function("session_create_single", |b| {
        b.to_async(runtime())
            .iter(async {
                let state = Arc::new(RwLock::new(SessionBenchState {
                    id: test_session_id(),
                    data: serde_json::json!({}),
                    created_at: std::time::Instant::now(),
                }));
                let _guard = state.read().await;
            });
    });
}

/// Benchmark: Multiple session creation
fn bench_session_create_batch(c: &mut criterion::Criterion) {
    let rt = runtime();

    c.bench_function("session_create_batch_100", |b| {
        b.to_async(rt)
            .iter(async {
                let handles: Vec<_> = (0..100)
                    .map(|_| {
                        let state = Arc::new(RwLock::new(SessionBenchState {
                            id: test_session_id(),
                            data: serde_json::json!({}),
                            created_at: std::time::Instant::now(),
                        }));
                        async move {
                            let _guard = state.read().await;
                        }
                    })
                    .collect();

                futures::future::join_all(handles).await;
            });
    });
}

/// Benchmark: Session state read
fn bench_session_state_read(c: &mut criterion::Criterion) {
    let rt = runtime();
    let state = Arc::new(RwLock::new(SessionBenchState {
        id: test_session_id(),
        data: serde_json::json!({"key": "value", "number": 42, "array": [1, 2, 3]}),
        created_at: std::time::Instant::now(),
    }));

    c.bench_function("session_state_read", |b| {
        b.to_async(rt).iter(async {
            let _guard = state.read().await;
        });
    });
}

/// Benchmark: Session state write
fn bench_session_state_write(c: &mut criterion::Criterion) {
    let rt = runtime();
    let state = Arc::new(RwLock::new(SessionBenchState {
        id: test_session_id(),
        data: serde_json::json!({}),
        created_at: std::time::Instant::now(),
    }));

    c.bench_function("session_state_write", |b| {
        b.to_async(rt).iter(async {
            let mut guard = state.write().await;
            guard.data = serde_json::json!({"updated": true});
        });
    });
}

/// Benchmark: Concurrent session operations
fn bench_concurrent_sessions(c: &mut criterion::Criterion) {
    let rt = runtime();
    let state = Arc::new(RwLock::new(SessionBenchState {
        id: test_session_id(),
        data: serde_json::json!({}),
        created_at: std::time::Instant::now(),
    }));

    c.bench_function("concurrent_session_ops_10", |b| {
        b.to_async(rt).iter(async {
            let handles: Vec<_> = (0..10)
                .map(|_| {
                    let state = state.clone();
                    async move {
                        for _ in 0..100 {
                            let mut guard = state.write().await;
                            guard.data = serde_json::json!({"count": uuid::Uuid::new_v4()});
                        }
                    }
                })
                .collect();

            futures::future::join_all(handles).await;
        });
    });
}

/// Benchmark: Session ID generation
fn bench_session_id_gen(c: &mut criterion::Criterion) {
    c.bench_function("session_id_gen_1000", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                criterion::black_box(test_session_id());
            }
        });
    });
}

/// Benchmark: Session serialization
fn bench_session_serialize(c: &mut criterion::Criterion) {
    let state = SessionBenchState {
        id: test_session_id(),
        data: serde_json::json!({
            "user": "test",
            "metadata": {"key": "value"},
            "items": [1, 2, 3, 4, 5]
        }),
        created_at: std::time::Instant::now(),
    };

    c.bench_function("session_serialize", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&state).unwrap();
            criterion::black_box(json);
        });
    });
}

/// Benchmark: Session deserialization
fn bench_session_deserialize(c: &mut criterion::Criterion) {
    let json = serde_json::json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "data": {"user": "test", "metadata": {"key": "value"}},
        "created_at": null
    });
    let json_str = serde_json::to_string(&json).unwrap();

    c.bench_function("session_deserialize", |b| {
        b.iter(|| {
            let _parsed: Result<SessionBenchState, _> =
                serde_json::from_str(&json_str);
        });
    });
}

criterion::criterion_group!(
    session_benches,
    bench_session_create,
    bench_session_create_batch,
    bench_session_state_read,
    bench_session_state_write,
    bench_concurrent_sessions,
    bench_session_id_gen,
    bench_session_serialize,
    bench_session_deserialize
);
criterion::criterion_main!(session_benches);

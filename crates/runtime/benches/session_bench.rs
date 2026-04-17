//! Session benchmarks for runtime crate

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use runtime::{Role, Session};

fn bench_session_creation(c: &mut Criterion) {
    c.bench_function("session_new", |b| {
        b.iter(|| Session::new(black_box("bench-session")))
    });
}

fn bench_session_add_message(c: &mut Criterion) {
    let content = "This is a test message content.".repeat(10);

    c.bench_function("session_add_message", |b| {
        b.iter(|| {
            let mut s = Session::new(black_box("bench"));
            s.add_message(black_box(Role::User), black_box(content.clone()))
                .unwrap();
        })
    });
}

fn bench_session_add_multiple_messages(c: &mut Criterion) {
    let contents: Vec<String> = (0..10)
        .map(|i| {
            format!(
                "Message {} with some content to simulate realistic conversation",
                i
            )
        })
        .collect();

    c.bench_function("session_add_multiple_messages", |b| {
        b.iter(|| {
            let mut s = Session::new(black_box("bench"));
            for content in &contents {
                s.add_message(Role::User, content.clone()).unwrap();
            }
        })
    });
}

fn bench_session_token_count(c: &mut Criterion) {
    let mut session = Session::new("bench");
    for i in 0..20 {
        session
            .add_message(
                Role::User,
                format!(
                    "This is message {} with substantial content for testing token counting",
                    i
                ),
            )
            .unwrap();
    }

    c.bench_function("session_token_count", |b| {
        b.iter(|| black_box(&session).token_count())
    });
}

fn bench_session_should_compact(c: &mut Criterion) {
    let mut session = Session::new("bench");
    for i in 0..20 {
        session
            .add_message(
                Role::User,
                format!(
                    "This is message {} with substantial content for testing token counting",
                    i
                ),
            )
            .unwrap();
    }

    c.bench_function("session_should_compact", |b| {
        b.iter(|| black_box(&session).should_compact(black_box(100)))
    });
}

criterion_group!(
    benches,
    bench_session_creation,
    bench_session_add_message,
    bench_session_add_multiple_messages,
    bench_session_token_count,
    bench_session_should_compact
);
criterion_main!(benches);

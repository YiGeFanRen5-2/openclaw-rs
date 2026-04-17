//! Chat API benchmarks for api-client crate

use api_client::{models::ChatRequest, ChatMessage};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_chat_request_creation(c: &mut Criterion) {
    let messages = vec![
        ChatMessage {
            role: "user".to_string(),
            content: "Hello, how are you?".to_string(),
        },
        ChatMessage {
            role: "assistant".to_string(),
            content: "I'm doing well, thanks for asking!".to_string(),
        },
    ];

    c.bench_function("chat_request_new", |b| {
        b.iter(|| api_client::models::ChatRequest {
            messages: black_box(messages.clone()),
            model: black_box("gpt-4".to_string()),
            temperature: Some(0.7),
            max_tokens: Some(1024),
            stream: false,
        })
    });
}

fn bench_chat_message_creation(c: &mut Criterion) {
    c.bench_function("chat_message_new", |b| {
        b.iter(|| ChatMessage {
            role: black_box("user".to_string()),
            content: black_box("Hello, world!".to_string()),
        })
    });
}

fn bench_provider_config_serialization(c: &mut Criterion) {
    use api_client::provider::ProviderConfig;

    let config = ProviderConfig::new("openai").api_key("sk-test");

    c.bench_function("provider_config_serialize", |b| {
        b.iter(|| serde_json::to_string(black_box(&config)).unwrap())
    });
}

fn bench_provider_config_deserialization(c: &mut Criterion) {
    use api_client::provider::ProviderConfig;

    let json = r#"{"kind":"openai","api_key":"sk-test","timeout_seconds":30}"#;

    c.bench_function("provider_config_deserialize", |b| {
        b.iter(|| serde_json::from_str::<ProviderConfig>(black_box(json)).unwrap())
    });
}

criterion_group!(
    benches,
    bench_chat_request_creation,
    bench_chat_message_creation,
    bench_provider_config_serialization,
    bench_provider_config_deserialization
);
criterion_main!(benches);

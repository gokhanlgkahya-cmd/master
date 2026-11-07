//! Sağlayıcıların sahte HTTP sunucusuna (wiremock) karşı entegrasyon testleri.

// Yalnız test kodu: yardımcı fonksiyonlarda unwrap/panic serbest (üretim kodunda yasak).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};
use notai_ai::{
    AiError, AnthropicClient, AnthropicConfig, ClassifyContext, ContextTopic, LlmClient,
    OpenAiCompatClient, OpenAiCompatConfig, ScriptedLlm, SecretKey, classify,
};
use notai_core::TopicId;
use notai_core::entry::{Entry, Source};
use notai_core::text::{EntryText, TopicName};
use notai_core::topic::ParaKind;
use serde_json::{Value, json};
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn ctx() -> ClassifyContext {
    ClassifyContext {
        entry: Entry::capture(
            EntryText::new("Kostik tedarikçisini ara – perşembe").unwrap(),
            Source::App,
            Utc.with_ymd_and_hms(2026, 11, 6, 6, 12, 0).unwrap(),
            None,
        ),
        topics: vec![ContextTopic {
            id: TopicId::new(),
            name: TopicName::new("Kostik tedarik").unwrap(),
            kind: ParaKind::Project { due: None },
            parent: None,
        }],
        facts: vec![],
        recent: vec![],
        tz: chrono_tz::Europe::Istanbul,
        user_name: "Gökhan".into(),
        examples: vec![],
        qa: vec![],
        can_ask: true,
    }
}

fn model_output(topic_id: TopicId) -> String {
    json!({
        "topic": { "existing_id": topic_id.to_string(), "new_name": null, "new_kind": null, "confidence": 0.9 },
        "tasks": [{ "title": "Kostik firmasını ara", "due_date": "2026-11-12", "due_time": null,
                    "priority": "normal", "confidence": 0.8 }],
        "tags": [],
        "people": [],
        "reminder": null
    })
    .to_string()
}

fn anthropic(server: &MockServer) -> AnthropicClient {
    let mut cfg = AnthropicConfig::new(SecretKey::new("sk-ant-test").unwrap());
    cfg.base_url = server.uri();
    AnthropicClient::new(cfg).unwrap()
}

#[tokio::test]
async fn anthropic_sends_structured_output_request_and_parses_text() {
    let server = MockServer::start().await;
    let c = ctx();
    let reply = json!({
        "id": "msg_1", "type": "message", "role": "assistant", "model": "claude-opus-5-5",
        "content": [
            { "type": "thinking", "thinking": "" },
            { "type": "text", "text": model_output(c.topics[0].id) }
        ],
        "stop_reason": "end_turn", "stop_details": null
    });
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "sk-ant-test"))
        .and(header("anthropic-version", "2026-10-21"))
        .and(header("anthropic-beta", "server-side-fallback-2026-07-01"))
        .and(body_partial_json(json!({
            "model": "claude-opus-5-5",
            "fallbacks": "default",
            "output_config": { "effort": "low", "format": { "type": "json_schema" } }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(reply))
        .expect(1)
        .mount(&server)
        .await;

    let v = classify(&anthropic(&server), &c).await.unwrap();
    assert_eq!(v.suggestions.len(), 2);

    let req: &Request = &server.received_requests().await.unwrap()[0];
    let body: Value = serde_json::from_slice(&req.body).unwrap();
    assert!(
        body.get("temperature").is_none(),
        "örnekleme parametresi gönderilmez"
    );
    assert!(body.get("thinking").is_none());
    assert!(
        body["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("<kayit>")
    );
}

#[tokio::test]
async fn anthropic_refusal_and_truncation_are_typed_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "content": [], "stop_reason": "refusal", "stop_details": { "type": "refusal", "category": "cyber" }
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "content": [{ "type": "text", "text": "{\"topic\":" }], "stop_reason": "max_tokens"
        })))
        .mount(&server)
        .await;

    let client = anthropic(&server);
    let err = classify(&client, &ctx()).await.unwrap_err();
    assert!(matches!(&err, AiError::Refused { category } if category == "cyber"));
    assert!(!err.is_retryable());
    let err = classify(&client, &ctx()).await.unwrap_err();
    assert!(matches!(err, AiError::Truncated));
    assert!(err.is_retryable());
}

#[tokio::test]
async fn anthropic_http_errors_classified_for_retry() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(529).set_body_string("overloaded"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad request"))
        .mount(&server)
        .await;
    let client = anthropic(&server);
    let e = classify(&client, &ctx()).await.unwrap_err();
    assert!(matches!(e, AiError::Status { status: 529, .. }) && e.is_retryable());
    let e = classify(&client, &ctx()).await.unwrap_err();
    assert!(matches!(e, AiError::Status { status: 400, .. }) && !e.is_retryable());
}

#[tokio::test]
async fn openai_compatible_local_model() {
    let server = MockServer::start().await;
    let c = ctx();
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_partial_json(json!({
            "model": "gemma3:12b",
            "response_format": { "type": "json_schema", "json_schema": { "strict": true } }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{ "message": { "role": "assistant", "content": model_output(c.topics[0].id) },
                          "finish_reason": "stop" }]
        })))
        .mount(&server)
        .await;
    let client =
        OpenAiCompatClient::new(OpenAiCompatConfig::new(server.uri(), "gemma3:12b")).unwrap();
    assert_eq!(client.name(), "openai-compat");
    let v = classify(&client, &c).await.unwrap();
    assert_eq!(v.suggestions.len(), 2);
}

#[tokio::test]
async fn model_returning_garbage_never_produces_suggestions() {
    let llm = ScriptedLlm::new()
        .reply("Tabii! İşte sınıflandırma: konu kostik.")
        .reply(r#"{"topic":null,"tasks":[],"tags":[],"people":[],"reminder":null,"sil":true}"#);
    assert!(matches!(
        classify(&llm, &ctx()).await,
        Err(AiError::InvalidJson(_))
    ));
    assert!(classify(&llm, &ctx()).await.is_err());
    assert_eq!(llm.requests().len(), 2);
}

#[tokio::test]
async fn whisper_transcription_multipart() {
    use notai_ai::media::{LocalServiceConfig, Transcriber, WhisperClient};
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "text": " Yarın sabah onda İzmir'deki kostik firmasını ara… yok, aslında perşembe yap. "
        })))
        .expect(1)
        .mount(&server)
        .await;
    let w = WhisperClient::new(LocalServiceConfig::new(server.uri(), "whisper-large-v3")).unwrap();
    let text = w.transcribe(b"OggS-sahte-ses", "audio/ogg").await.unwrap();
    assert!(text.starts_with("Yarın sabah onda"));
    let req = &server.received_requests().await.unwrap()[0];
    let body = String::from_utf8_lossy(&req.body);
    assert!(body.contains("name=\"language\"\r\n\r\ntr"));
    assert!(body.contains("filename=\"kayit.ogg\""));
    assert!(w.transcribe(b"", "audio/ogg").await.is_err());
}

#[tokio::test]
async fn embeddings_ordered_and_validated() {
    use notai_ai::media::{Embedder, EmbeddingClient, LocalServiceConfig};
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .and(body_partial_json(json!({ "model": "bge-m3" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [ { "index": 1, "embedding": [0.0, 1.0] }, { "index": 0, "embedding": [1.0, 0.0] } ]
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [ { "index": 0, "embedding": [1.0] }, { "index": 1, "embedding": [1.0, 2.0] } ]
        })))
        .mount(&server)
        .await;
    let e = EmbeddingClient::new(LocalServiceConfig::new(server.uri(), "bge-m3")).unwrap();
    let v = e.embed(&["kostik".into(), "kombi".into()]).await.unwrap();
    assert_eq!(v, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
    assert!(matches!(
        e.embed(&["a".into(), "b".into()]).await,
        Err(AiError::Schema(_))
    ));
    assert_eq!(e.model(), "bge-m3");
}

#[tokio::test]
async fn ask_end_to_end_with_scripted_model() {
    use notai_ai::ask::{AskContext, AskSource, ask};
    let id = notai_core::EntryId::new();
    let llm = ScriptedLlm::new().reply(
        json!({ "answer": "1.850 ₺", "source_ids": [id.to_string()], "found": true }).to_string(),
    );
    let ctx = AskContext {
        question: "Kostik fiyatı?".into(),
        sources: vec![AskSource {
            id,
            captured_at: Utc.with_ymd_and_hms(2026, 11, 3, 7, 0, 0).unwrap(),
            text: "NaOH 25 kg 1.850 ₺".into(),
            topic: None,
        }],
        tz: chrono_tz::Europe::Istanbul,
        user_name: "Gökhan".into(),
    };
    let a = ask(&llm, &ctx).await.unwrap();
    assert_eq!(a.sources, vec![id]);
}

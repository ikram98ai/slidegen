//! Integration tests for `AIService` against a mocked Gemini API.
//!
//! Run with: `make test-int` or `cargo test --test ai_service_test`

use base64::{Engine as _, engine::general_purpose};
use slidegen::services::AIService;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TTS_MODEL: &str = "gemini-2.5-flash-preview-tts";

fn service_for(server: &MockServer) -> AIService {
    AIService::with_config("test-key", "test-model", server.uri())
}

fn text_response(text: &str) -> serde_json::Value {
    serde_json::json!({
        "candidates": [{
            "content": { "parts": [{ "text": text }] }
        }]
    })
}

#[tokio::test]
async fn analyze_book_toc_parses_fenced_json_completion() {
    let server = MockServer::start().await;
    let completion = "Here are the chapters:\n```json\n{\"chapters\": [\
        {\"title\": \"Introduction\", \"page_start\": 1, \"page_end\": 12},\
        {\"title\": \"Ownership\", \"page_start\": 13, \"page_end\": 40}\
    ]}\n```";

    Mock::given(method("POST"))
        .and(path("/v1beta/models/test-model:generateContent"))
        .and(header("x-goog-api-key", "test-key"))
        .and(body_partial_json(serde_json::json!({
            "contents": [{ "parts": [{ "text": "Table Of Contents:\nCh1 ... Ch2 ..." }] }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(text_response(completion)))
        .expect(1)
        .mount(&server)
        .await;

    let svc = service_for(&server);
    let chapters = svc.analyze_book_toc("Ch1 ... Ch2 ...").await.unwrap();

    assert_eq!(chapters.len(), 2);
    assert_eq!(chapters[0].title, "Introduction");
    assert_eq!(chapters[0].page_start, 1);
    assert_eq!(chapters[0].page_end, 12);
    assert_eq!(chapters[1].title, "Ownership");
}

#[tokio::test]
async fn analyze_book_toc_concatenates_multiple_text_parts() {
    let server = MockServer::start().await;
    let body = serde_json::json!({
        "candidates": [{
            "content": { "parts": [
                { "text": "{\"chapters\": [{\"title\": \"A\"," },
                { "text": " \"page_start\": 1, \"page_end\": 2}]}" }
            ]}
        }]
    });

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let svc = service_for(&server);
    let chapters = svc.analyze_book_toc("toc").await.unwrap();
    assert_eq!(chapters.len(), 1);
    assert_eq!(chapters[0].title, "A");
}

#[tokio::test]
async fn analyze_book_toc_fails_on_malformed_json() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(text_response("I cannot help with that.")),
        )
        .mount(&server)
        .await;

    let svc = service_for(&server);
    let err = svc.analyze_book_toc("toc").await.unwrap_err();
    assert!(
        err.to_string().contains("Failed to parse chapters JSON"),
        "got: {err}"
    );
}

#[tokio::test]
async fn analyze_book_toc_propagates_api_errors_with_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate limited"))
        .mount(&server)
        .await;

    let svc = service_for(&server);
    let err = svc.analyze_book_toc("toc").await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("429"), "got: {msg}");
    assert!(msg.contains("rate limited"), "got: {msg}");
}

#[tokio::test]
async fn analyze_book_toc_fails_when_no_candidates() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "candidates": [] })),
        )
        .mount(&server)
        .await;

    let svc = service_for(&server);
    let err = svc.analyze_book_toc("toc").await.unwrap_err();
    assert!(err.to_string().contains("no text"), "got: {err}");
}

#[tokio::test]
async fn generate_slides_parses_completion() {
    let server = MockServer::start().await;
    let completion = "{\"slides\": [{\"title\": \"What is Rust?\", \
        \"bullets\": [\"Fast\", \"Safe\"], \"explanation\": \"Rust is a systems language.\"}]}";

    Mock::given(method("POST"))
        .and(path("/v1beta/models/test-model:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(text_response(completion)))
        .mount(&server)
        .await;

    let svc = service_for(&server);
    let slides = svc.generate_slides("Some chapter text").await.unwrap();

    assert_eq!(slides.len(), 1);
    assert_eq!(slides[0].title, "What is Rust?");
    assert_eq!(slides[0].bullets, vec!["Fast", "Safe"]);
    assert_eq!(slides[0].explanation, "Rust is a systems language.");
}

#[tokio::test]
async fn plan_chapter_scenes_parses_paragraph_ids() {
    let server = MockServer::start().await;
    let completion = r#"{
      "kid_lede": "Packets are labeled boxes.",
      "scenes": [{
        "id": "packets",
        "title": "What is a packet?",
        "focus": "A packet is a labeled box of data.",
        "paragraph_ids": ["p14-2", "p15-1"]
      }]
    }"#;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/test-model:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(text_response(completion)))
        .mount(&server)
        .await;

    let svc = service_for(&server);
    let plan = svc
        .plan_chapter_scenes("Packets", "[p14-2] A packet is a small labeled box.")
        .await
        .unwrap();

    assert_eq!(plan.scenes.len(), 1);
    assert_eq!(plan.scenes[0].paragraph_ids, vec!["p14-2", "p15-1"]);
}

#[tokio::test]
async fn generate_chapter_parses_scene_spec() {
    let server = MockServer::start().await;
    let completion = r#"{
      "kid_lede": "Packets are labeled boxes.",
      "scenes": [{
        "id": "packets",
        "title": "What is a packet?",
        "kid_summary": "A small labeled box of data.",
        "viz": { "type": "steps", "params": { "title": "Send", "steps": [{"title": "Write", "caption": "Put data in."}] } },
        "depth": "A packet has a header and a payload.",
        "citations": [{ "pdf_page": 14, "printed_page": 3, "paragraph_id": "p14-2", "quote": "A packet is a small labeled box of data." }]
      }]
    }"#;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/test-model:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(text_response(completion)))
        .mount(&server)
        .await;

    let svc = service_for(&server);
    let chapter = svc
        .generate_chapter(
            "Packets",
            "[p14-2] A packet is a small labeled box of data.",
        )
        .await
        .unwrap();

    assert_eq!(chapter.kid_lede, "Packets are labeled boxes.");
    assert_eq!(chapter.scenes.len(), 1);
    assert_eq!(chapter.scenes[0].id, "packets");
    assert_eq!(chapter.scenes[0].viz.kind, "steps");
    assert_eq!(chapter.scenes[0].citations[0].paragraph_id, "p14-2");
}

#[tokio::test]
async fn generate_slide_audio_returns_wav_with_mime_sample_rate() {
    let server = MockServer::start().await;
    let pcm = vec![10u8, 20, 30, 40, 50, 60];
    let body = serde_json::json!({
        "candidates": [{
            "content": { "parts": [{
                "inlineData": {
                    "mimeType": "audio/L16;codec=pcm;rate=16000",
                    "data": general_purpose::STANDARD.encode(&pcm)
                }
            }]}
        }]
    });

    Mock::given(method("POST"))
        .and(path(format!(
            "/v1beta/models/{}:generateContent",
            TTS_MODEL
        )))
        .and(header("x-goog-api-key", "test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let svc = service_for(&server);
    let wav = svc.generate_slide_audio("Hello world").await.unwrap();

    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 16_000);
    assert_eq!(&wav[44..], &pcm[..]);
}

#[tokio::test]
async fn generate_slide_audio_fails_without_inline_data() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(text_response("no audio here")))
        .mount(&server)
        .await;

    let svc = service_for(&server);
    let err = svc.generate_slide_audio("Hello").await.unwrap_err();
    assert!(err.to_string().contains("no audio data"), "got: {err}");
}

#[tokio::test]
async fn generate_slide_audio_fails_on_invalid_base64() {
    let server = MockServer::start().await;
    let body = serde_json::json!({
        "candidates": [{
            "content": { "parts": [{
                "inlineData": { "mimeType": "audio/L16;rate=24000", "data": "!!!not-base64!!!" }
            }]}
        }]
    });
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let svc = service_for(&server);
    let err = svc.generate_slide_audio("Hello").await.unwrap_err();
    assert!(err.to_string().contains("base64"), "got: {err}");
}

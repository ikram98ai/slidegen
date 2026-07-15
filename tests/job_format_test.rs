//! Tests for the background `Job` wire format.
//!
//! The JSON format is a contract between the API Lambda (producer) and the
//! worker Lambda (consumer) — breaking it strands in-flight SQS messages.
//!
//! Run with: `make test-int` or `cargo test --test job_format_test`

use slidegen::services::bg_tasks::Job;

#[test]
fn job_serializes_with_stable_tagged_format() {
    let job = Job::GenerateSlides {
        user_id: "u1".to_string(),
        subject_id: "s1".to_string(),
        chapter_id: "c1".to_string(),
    };
    let json = serde_json::to_string(&job).unwrap();
    assert_eq!(
        json,
        r#"{"type":"generate_slides","user_id":"u1","subject_id":"s1","chapter_id":"c1"}"#
    );

    let parsed: Job = serde_json::from_str(&json).unwrap();
    match parsed {
        Job::GenerateSlides { chapter_id, .. } => assert_eq!(chapter_id, "c1"),
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn process_subject_job_roundtrips() {
    let json = r#"{"type":"process_subject","user_id":"u1","subject_id":"s1","file_s3path":"subjects/u1/s1.pdf"}"#;
    let parsed: Job = serde_json::from_str(json).unwrap();
    match parsed {
        Job::ProcessSubject { file_s3path, .. } => {
            assert_eq!(file_s3path, "subjects/u1/s1.pdf")
        }
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn unknown_job_type_fails_to_parse() {
    let json = r#"{"type":"drop_all_tables","user_id":"u1"}"#;
    assert!(serde_json::from_str::<Job>(json).is_err());
}

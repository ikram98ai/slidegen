use crate::models::scene::{ChapterManifest, RUNTIME_VERSION};
use anyhow::{Context, Result};

const KIT_JS: &str = include_str!("../runtime/v1/kit.js");
const PLAYER_JS: &str = include_str!("../runtime/v1/player.js");
const STYLE_CSS: &str = include_str!("../runtime/v1/style.css");

const HTML_SHELL: &str = r##"<!DOCTYPE html>
<html lang="en" data-theme="light">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="slidegen-runtime" content="__RUNTIME__">
  <title>__TITLE__</title>
  <style>
__CSS__
  </style>
</head>
<body class="embed">
  <header id="topbar" class="topbar"></header>
  <main id="chapter-root"></main>
  <script>
window.SLIDEGEN_CHAPTER = __MANIFEST__;
  </script>
  <script>
__KIT__
  </script>
  <script>
__PLAYER__
  </script>
</body>
</html>
"##;

/// Compiles a chapter manifest into a self-contained HTML document.
/// The model never writes this HTML — only SceneSpec JSON.
pub fn compile_chapter(manifest: &ChapterManifest) -> Result<String> {
    let json = serde_json::to_string(manifest).context("failed to serialize chapter manifest")?;
    // Prevent a `</script>` in scene text from breaking out of the inline script.
    let json = json.replace("</", "<\\/");
    let html = HTML_SHELL
        .replace("__RUNTIME__", RUNTIME_VERSION)
        .replace("__TITLE__", &escape_html(&manifest.title))
        .replace("__CSS__", STYLE_CSS)
        .replace("__KIT__", KIT_JS)
        .replace("__PLAYER__", PLAYER_JS)
        .replace("__MANIFEST__", &json);
    Ok(html)
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::scene::{
        Citation, SceneDepth, SceneSpec, VizSpec, chapter_package_html_key,
        retain_grounded_citations, scene_id_slug,
    };

    fn sample_manifest() -> ChapterManifest {
        ChapterManifest::new(
            "book-1",
            "Sample Book",
            "ch-1",
            "Packets on the Wire",
            "A packet is a small labeled box of data.",
            0,
            12,
            18,
            vec![SceneSpec {
                id: "packets".into(),
                title: "What is a packet?".into(),
                kid_summary: "Think of a letter in an envelope.".into(),
                viz: VizSpec {
                    kind: "steps".into(),
                    params: serde_json::json!({
                        "title": "Send a packet",
                        "steps": [
                            {"title": "Write", "caption": "Put the data in the box."},
                            {"title": "Label", "caption": "Add the destination."}
                        ]
                    }),
                },
                depth: SceneDepth {
                    text: "A packet carries a header and a payload.".into(),
                    audio_key: Some("subjects/u/s/chapters/ch-1/audio/packets.wav".into()),
                    audio_url: Some("https://example.test/packets.wav".into()),
                },
                citations: vec![Citation {
                    pdf_page: 14,
                    printed_page: Some(3),
                    paragraph_id: "p14-2".into(),
                    quote: "A packet is a small labeled box of data.".into(),
                }],
                quiz: None,
            }],
        )
    }

    #[test]
    fn compiles_self_contained_html() {
        let html = compile_chapter(&sample_manifest()).expect("compile");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("slidegen-runtime\" content=\"v1\""));
        assert!(html.contains("window.SLIDEGEN_CHAPTER"));
        assert!(html.contains("Packets on the Wire"));
        assert!(html.contains("p14-2"));
        assert!(html.contains("window.K"));
        assert!(html.contains("const h ="));
        assert!(html.contains("slidegen:citation"));
        assert!(html.contains("https://example.test/packets.wav"));
        assert!(!html.contains("__MANIFEST__"));
        assert!(!html.contains("__KIT__"));
        assert!(!html.contains("__PLAYER__"));
    }

    #[test]
    fn escapes_script_breakout_in_manifest() {
        let mut manifest = sample_manifest();
        manifest.scenes[0].kid_summary = "see </script><img src=x>".into();
        let html = compile_chapter(&manifest).expect("compile");
        assert!(!html.contains("</script><img"));
        assert!(html.contains("<\\/script>"));
    }

    #[test]
    fn package_key_is_stable() {
        assert_eq!(
            chapter_package_html_key("u1", "s1", "c1"),
            "subjects/u1/s1/chapters/c1/index.html"
        );
    }

    #[test]
    fn slugs_scene_ids() {
        assert_eq!(scene_id_slug("What is a Packet?", 1), "what-is-a-packet");
        assert_eq!(scene_id_slug("!!!", 2), "scene-2");
    }

    #[test]
    fn drops_ungrounded_citations() {
        let text = "A packet is a small labeled box of data. See [p14-2].";
        let kept = retain_grounded_citations(
            text,
            vec![
                Citation {
                    pdf_page: 14,
                    printed_page: Some(3),
                    paragraph_id: "p14-2".into(),
                    quote: "A packet is a small labeled box of data.".into(),
                },
                Citation {
                    pdf_page: 99,
                    printed_page: None,
                    paragraph_id: "p99-1".into(),
                    quote: "This sentence is not in the book.".into(),
                },
            ],
        );
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].paragraph_id, "p14-2");
    }

    #[test]
    fn uniquifies_duplicate_scene_ids() {
        let scene = |id: &str| SceneSpec {
            id: id.into(),
            title: "T".into(),
            kid_summary: "S".into(),
            viz: VizSpec {
                kind: "steps".into(),
                params: serde_json::json!({}),
            },
            depth: SceneDepth {
                text: "D".into(),
                audio_key: None,
                audio_url: None,
            },
            citations: vec![],
            quiz: None,
        };
        let manifest = ChapterManifest::new(
            "b",
            "B",
            "c",
            "T",
            "L",
            0,
            1,
            2,
            vec![scene("same"), scene("same")],
        );
        assert_eq!(manifest.scenes[0].id, "same");
        assert_eq!(manifest.scenes[1].id, "same-2");
    }

    #[test]
    fn citation_match_tolerates_whitespace() {
        let text = "A packet   is a small\nlabeled box of data.";
        let kept = retain_grounded_citations(
            text,
            vec![Citation {
                pdf_page: 1,
                printed_page: None,
                paragraph_id: "p1-1".into(),
                quote: "A packet is a small labeled box of data.".into(),
            }],
        );
        assert_eq!(kept.len(), 1);
    }
}

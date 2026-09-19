use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const CHAPTER_SCHEMA: &str = "slidegen.chapter.v1";
pub const RUNTIME_VERSION: &str = "v1";
pub const MAX_SCENES_PER_CHAPTER: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VizType {
    Steps,
    TwoLane,
    Tradeoff,
    Flow,
    SliderCompare,
    Quiz,
}

impl VizType {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "two_lane" | "twolane" | "compare" => Self::TwoLane,
            "tradeoff" | "trade-off" => Self::Tradeoff,
            "flow" | "pipeline" => Self::Flow,
            "slider_compare" | "slider" => Self::SliderCompare,
            "quiz" => Self::Quiz,
            _ => Self::Steps,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Steps => "steps",
            Self::TwoLane => "two_lane",
            Self::Tradeoff => "tradeoff",
            Self::Flow => "flow",
            Self::SliderCompare => "slider_compare",
            Self::Quiz => "quiz",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VizSpec {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

impl VizSpec {
    pub fn normalized(self) -> Self {
        Self {
            kind: VizType::parse(&self.kind).as_str().to_string(),
            params: self.params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Citation {
    pub pdf_page: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printed_page: Option<i32>,
    pub paragraph_id: String,
    pub quote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct QuizItem {
    pub q: String,
    pub options: Vec<String>,
    pub answer: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SceneDepth {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SceneSpec {
    pub id: String,
    pub title: String,
    pub kid_summary: String,
    pub viz: VizSpec,
    pub depth: SceneDepth,
    #[serde(default)]
    pub citations: Vec<Citation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiz: Option<Vec<QuizItem>>,
}

impl SceneSpec {
    pub fn from_generated(generated: GeneratedScene, index: usize) -> Self {
        Self {
            id: scene_id_slug(&generated.id, index + 1),
            title: generated.title,
            kid_summary: generated.kid_summary,
            viz: generated.viz.normalized(),
            depth: SceneDepth {
                text: generated.depth,
                audio_key: None,
                audio_url: None,
            },
            citations: generated.citations,
            quiz: generated.quiz,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ChapterManifest {
    pub schema: String,
    pub runtime: String,
    pub book_id: String,
    pub book_title: String,
    pub chapter_id: String,
    pub title: String,
    pub kid_lede: String,
    pub order_index: i32,
    pub page_start: i32,
    pub page_end: i32,
    pub scenes: Vec<SceneSpec>,
}

impl ChapterManifest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        book_id: impl Into<String>,
        book_title: impl Into<String>,
        chapter_id: impl Into<String>,
        title: impl Into<String>,
        kid_lede: impl Into<String>,
        order_index: i32,
        page_start: i32,
        page_end: i32,
        mut scenes: Vec<SceneSpec>,
    ) -> Self {
        if scenes.len() > MAX_SCENES_PER_CHAPTER {
            scenes.truncate(MAX_SCENES_PER_CHAPTER);
        }
        uniquify_scene_ids(&mut scenes);
        Self {
            schema: CHAPTER_SCHEMA.to_string(),
            runtime: RUNTIME_VERSION.to_string(),
            book_id: book_id.into(),
            book_title: book_title.into(),
            chapter_id: chapter_id.into(),
            title: title.into(),
            kid_lede: kid_lede.into(),
            order_index,
            page_start,
            page_end,
            scenes,
        }
    }

    pub fn strip_ephemeral_urls(&mut self) {
        for scene in &mut self.scenes {
            scene.depth.audio_url = None;
        }
    }
}

/// Planner output: which extract paragraphs each scene should be written from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterPlan {
    #[serde(default)]
    pub kid_lede: String,
    pub scenes: Vec<ScenePlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenePlan {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub focus: String,
    #[serde(default)]
    pub paragraph_ids: Vec<String>,
}

/// Model completion shape before we attach audio keys and drop bad citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedChapter {
    #[serde(default)]
    pub kid_lede: String,
    pub scenes: Vec<GeneratedScene>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedScene {
    pub id: String,
    pub title: String,
    pub kid_summary: String,
    pub viz: VizSpec,
    pub depth: String,
    #[serde(default)]
    pub citations: Vec<Citation>,
    #[serde(default)]
    pub quiz: Option<Vec<QuizItem>>,
}

pub fn chapter_package_prefix(user_id: &str, subject_id: &str, chapter_id: &str) -> String {
    format!("subjects/{user_id}/{subject_id}/chapters/{chapter_id}")
}

pub fn chapter_package_html_key(user_id: &str, subject_id: &str, chapter_id: &str) -> String {
    format!(
        "{}/index.html",
        chapter_package_prefix(user_id, subject_id, chapter_id)
    )
}

pub fn chapter_manifest_key(user_id: &str, subject_id: &str, chapter_id: &str) -> String {
    format!(
        "{}/manifest.json",
        chapter_package_prefix(user_id, subject_id, chapter_id)
    )
}

pub fn chapter_audio_key(
    user_id: &str,
    subject_id: &str,
    chapter_id: &str,
    scene_id: &str,
) -> String {
    format!(
        "{}/audio/{}.wav",
        chapter_package_prefix(user_id, subject_id, chapter_id),
        scene_id
    )
}

fn collapsed_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Drops citations whose quote does not appear in the grounded chapter text.
pub fn retain_grounded_citations(text: &str, citations: Vec<Citation>) -> Vec<Citation> {
    let haystack = collapsed_ws(text);
    citations
        .into_iter()
        .filter(|c| {
            let quote = collapsed_ws(&c.quote);
            quote.len() >= 8 && haystack.contains(&quote)
        })
        .collect()
}

pub fn scene_id_slug(raw: &str, fallback: usize) -> String {
    let slug: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        format!("scene-{fallback}")
    } else {
        slug
    }
}

pub fn uniquify_scene_ids(scenes: &mut [SceneSpec]) {
    let mut used = HashSet::new();
    for (i, scene) in scenes.iter_mut().enumerate() {
        let mut id = scene_id_slug(&scene.id, i + 1);
        if !used.insert(id.clone()) {
            id = format!("{id}-{}", i + 1);
            used.insert(id.clone());
        }
        scene.id = id;
    }
}

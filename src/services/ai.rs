use crate::config::Settings;
use crate::models::{ChapterPlan, GeneratedChapter};
use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com";
const TTS_MODEL: &str = "gemini-2.5-flash-preview-tts";
// Gemini TTS returns 16-bit mono PCM at 24kHz unless the mimeType says otherwise.
const DEFAULT_TTS_SAMPLE_RATE: u32 = 24_000;

#[derive(Clone, Debug)]
pub struct AIService {
    client: Client,
    api_key: String,
    text_model: String,
    base_url: String,
}

// Internal Gemini REST API Request/Response structs
#[derive(Serialize, Clone)]
struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "inlineData")]
    inline_data: Option<InlineData>,
}

#[derive(Serialize, Clone)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Clone)]
struct SystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<SystemInstruction>,
    contents: Vec<GeminiContent>,
}

#[derive(Serialize)]
struct PrebuiltVoiceConfig {
    #[serde(rename = "voiceName")]
    voice_name: String,
}

#[derive(Serialize)]
struct VoiceConfig {
    #[serde(rename = "prebuiltVoiceConfig")]
    prebuilt_voice_config: PrebuiltVoiceConfig,
}

#[derive(Serialize)]
struct SpeechConfig {
    #[serde(rename = "voiceConfig")]
    voice_config: VoiceConfig,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "responseModalities")]
    response_modalities: Vec<String>,
    #[serde(rename = "speechConfig")]
    speech_config: SpeechConfig,
}

// Note: the model is addressed via the URL path; it must not be repeated in the body.
#[derive(Serialize)]
struct GeminiTTSRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: Content,
}

#[derive(Deserialize, Debug)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Deserialize, Debug)]
struct Part {
    text: Option<String>,
    #[serde(rename = "inlineData")]
    inline_data: Option<InlineDataResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct InlineData {
    #[serde(rename = "mimeType")]
    mime_type: String,
    data: String,
}

#[derive(Deserialize, Debug)]
struct InlineDataResponse {
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
    data: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChapterData {
    pub title: String,
    pub page_start: i32,
    pub page_end: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Chapters {
    pub chapters: Vec<ChapterData>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SlideData {
    pub title: String,
    pub bullets: Vec<String>,
    pub explanation: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Slides {
    pub slides: Vec<SlideData>,
}

/// Parses the JSON object embedded in a model completion, tolerating markdown
/// fences, surrounding prose, and reasoning preambles that quote JSON schema
/// examples before the final answer. Later candidates win because the model's
/// final answer comes last. `T` must have required fields so partial inner
/// objects don't parse successfully.
fn parse_json_completion<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    // Prefer the last ```json fence: that's where the final answer lives.
    if let Some(pos) = text.rfind("```json") {
        let body = &text[pos + "```json".len()..];
        let body = match body.rfind("```") {
            Some(end) => &body[..end],
            None => body,
        };
        if let Ok(v) = serde_json::from_str(body.trim()) {
            return Ok(v);
        }
    }

    // Otherwise find the last '{' that starts a parseable object, ignoring
    // whatever text follows it.
    let mut search_end = text.len();
    while let Some(start) = text[..search_end].rfind('{') {
        let mut candidates = serde_json::Deserializer::from_str(&text[start..]).into_iter::<T>();
        if let Some(Ok(v)) = candidates.next() {
            return Ok(v);
        }
        search_end = start;
    }

    anyhow::bail!("No valid JSON object found in model completion")
}

/// Parses the sample rate from a Gemini audio mime type such as
/// "audio/L16;codec=pcm;rate=24000".
fn parse_sample_rate(mime_type: &str) -> u32 {
    mime_type
        .split(';')
        .filter_map(|part| part.trim().strip_prefix("rate="))
        .find_map(|rate| rate.parse().ok())
        .unwrap_or(DEFAULT_TTS_SAMPLE_RATE)
}

/// Wraps raw 16-bit PCM samples in a WAV (RIFF) header so browsers can play them.
fn pcm_to_wav(pcm: &[u8], sample_rate: u32, num_channels: u16, bits_per_sample: u16) -> Vec<u8> {
    let bytes_per_sample = u32::from(bits_per_sample / 8);
    let byte_rate = sample_rate * u32::from(num_channels) * bytes_per_sample;
    let block_align = num_channels * (bits_per_sample / 8);
    let data_len = pcm.len() as u32;

    let mut wav = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // audio format: PCM
    wav.extend_from_slice(&num_channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(pcm);
    wav
}

impl AIService {
    pub fn new(settings: &Settings) -> Self {
        let api_key = settings
            .gemini_api_key
            .as_ref()
            .expect("GEMINI_API_KEY must be set")
            .clone();

        Self::with_config(api_key, settings.text_model.clone(), DEFAULT_BASE_URL)
    }

    /// Builds a service with explicit configuration. Primarily useful for
    /// pointing the service at a mock server in tests.
    pub fn with_config(
        api_key: impl Into<String>,
        text_model: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        AIService {
            client: Client::new(),
            api_key: api_key.into(),
            text_model: text_model.into(),
            base_url: base_url.into(),
        }
    }

    async fn call_gemini_text(
        &self,
        model: &str,
        system_instruction: Option<String>,
        prompt: String,
    ) -> Result<String> {
        let url = format!("{}/v1beta/models/{}:generateContent", self.base_url, model);

        let body = GeminiRequest {
            system_instruction: system_instruction.map(|si| SystemInstruction {
                parts: vec![GeminiPart {
                    text: Some(si),
                    inline_data: None,
                }],
            }),
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: Some(prompt),
                    inline_data: None,
                }],
            }],
        };

        let response = self
            .client
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        if !response.status().is_success() {
            let status = response.status();
            let err_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Gemini API error ({}): {}", status, err_text);
        }

        let resp_body: GeminiResponse = response
            .json()
            .await
            .context("Failed to parse Gemini API response")?;

        // Concatenate every text part: Gemini may split a completion across parts.
        let text = resp_body
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .map(|c| {
                c.content
                    .parts
                    .iter()
                    .filter_map(|p| p.text.as_deref())
                    .collect::<String>()
            })
            .filter(|t| !t.is_empty())
            .context("Gemini API returned no text")?;

        Ok(text)
    }

    pub async fn analyze_book_toc(&self, toc: &str) -> Result<Vec<ChapterData>> {
        let system_instruction = "You are an expert educational content analyzer.
            Analyze the Table of Contents.
            Identify the main chapters or sections.
            Return a list of chapters with their titles, page_start, page_end.
            Do not hallucinate chapters if they are not clear.
            page_start and page_end MUST be 1-based PDF page numbers from the 'PDF p.N' labels in the prompt (not printed folios, unless they are the only numbers given). Never return 0.
            Reply ONLY with a valid JSON strictly matching this schema: { \"chapters\": [ { \"title\": \"string\", \"page_start\": 0, \"page_end\": 0 } ] }".to_string();

        let prompt = format!("Table Of Contents:\n{}", toc);

        let completion = self
            .call_gemini_text(&self.text_model, Some(system_instruction), prompt)
            .await?;

        let chapters: Chapters = parse_json_completion(&completion)
            .with_context(|| format!("Failed to parse chapters JSON from: {}", completion))?;

        Ok(chapters.chapters)
    }

    pub async fn generate_slides(&self, page_text: &str) -> Result<Vec<SlideData>> {
        let system_instruction = "You are an expert presentation designer.
            Create detailed educational presentation slides.
            Reply ONLY with a valid JSON strictly matching this schema:
            { \"slides\": [ { \"title\": \"string\", \"bullets\": [\"string\"], \"explanation\": \"string\" } ] }".to_string();

        let prompt = format!("Context for the slides: \"{}\".", page_text);

        let completion = self
            .call_gemini_text(&self.text_model, Some(system_instruction), prompt)
            .await?;

        let slides: Slides = parse_json_completion(&completion)
            .with_context(|| format!("Failed to parse slides JSON from: {}", completion))?;

        Ok(slides.slides)
    }

    /// Picks scene topics and the extract paragraphs each scene may use.
    pub async fn plan_chapter_scenes(
        &self,
        chapter_title: &str,
        catalog: &str,
    ) -> Result<ChapterPlan> {
        let system_instruction = r#"You plan interactive book chapters from an extract catalog.
Reply ONLY with JSON:
{
  "kid_lede": "one-sentence hook",
  "scenes": [
    {
      "id": "kebab-case-id",
      "title": "string",
      "focus": "what this scene must teach, one sentence",
      "paragraph_ids": ["p12-2", "p12-3"]
    }
  ]
}
Rules:
- Plan 4–6 scenes, never more than 8.
- paragraph_ids MUST be copied from the catalog. Do not invent IDs.
- Each scene needs 2–6 paragraph_ids that actually support its focus.
- Prefer contiguous paragraphs on the same page when they tell one idea."#
            .to_string();

        let prompt = format!("Chapter title: {chapter_title}\n\nParagraph catalog:\n{catalog}");

        let completion = self
            .call_gemini_text(&self.text_model, Some(system_instruction), prompt)
            .await?;

        let plan: ChapterPlan = parse_json_completion(&completion)
            .with_context(|| format!("Failed to parse chapter plan JSON from: {}", completion))?;

        Ok(plan)
    }

    /// Writes a grounded SceneSpec chapter. The model must cite paragraph IDs
    /// from the extract (`[p12-2] ...`); it must not invent HTML.
    pub async fn generate_chapter(
        &self,
        chapter_title: &str,
        chapter_text: &str,
    ) -> Result<GeneratedChapter> {
        let system_instruction = r#"You are an expert educational designer for interactive book chapters.
Turn the supplied source text into 4–8 short scenes a 12-year-old can follow.
Reply ONLY with JSON matching this schema:
{
  "kid_lede": "one-sentence chapter hook",
  "scenes": [
    {
      "id": "kebab-case-id",
      "title": "string",
      "kid_summary": "2–3 sentences in plain language",
      "viz": {
        "type": "steps | two_lane | tradeoff | flow | slider_compare | quiz",
        "params": {}
      },
      "depth": "2–4 paragraph voice-over that explains the idea more carefully",
      "citations": [
        {
          "pdf_page": 1,
          "printed_page": 1,
          "paragraph_id": "p12-2",
          "quote": "exact contiguous phrase copied from that paragraph"
        }
      ],
      "quiz": [{ "q": "string", "options": ["A","B","C"], "answer": 0, "why": "string" }]
    }
  ]
}
Rules:
- Every scene MUST include at least one citation.
- citation.quote MUST be copied verbatim from the paragraph marked [paragraph_id].
- Cite ONLY paragraph IDs that appear in the source below. Do not invent IDs or quotes.
- pdf_page MUST match the 'PDF p.N' label. printed_page is the printed folio when given, else omit it.
- viz.params by type:
  steps: { "title": "...", "steps": [{ "title": "...", "caption": "..." }] }
  two_lane: { "title": "...", "left": { "title": "...", "points": ["..."] }, "right": { "title": "...", "points": ["..."] } }
  tradeoff: { "title": "...", "left": { "title": "...", "text": "..." }, "right": { "title": "...", "text": "..." } }
  flow: { "title": "...", "nodes": ["A","B","C"], "caption": "..." }
  slider_compare: { "title": "...", "label": "...", "left_label": "...", "right_label": "...", "captions": ["..."] }
  quiz: put items in the scene-level "quiz" array; params may be {}
- Do not write HTML, CSS, or JavaScript. Do not invent facts that are not in the source.
- Prefer 4–6 scenes. Never more than 8."#
            .to_string();

        let prompt = format!(
            "Chapter title: {chapter_title}\n\nSource text (paragraphs are labeled [pPAGE-N]):\n{chapter_text}"
        );

        let completion = self
            .call_gemini_text(&self.text_model, Some(system_instruction), prompt)
            .await?;

        let chapter: GeneratedChapter = parse_json_completion(&completion)
            .with_context(|| format!("Failed to parse chapter JSON from: {}", completion))?;

        Ok(chapter)
    }

    /// Generates narration audio for a slide and returns playable WAV bytes.
    pub async fn generate_slide_audio(&self, text: &str) -> Result<Vec<u8>> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, TTS_MODEL
        );

        let body = GeminiTTSRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: Some(text.to_string()),
                    inline_data: None,
                }],
            }],
            generation_config: GenerationConfig {
                response_modalities: vec!["AUDIO".to_string()],
                speech_config: SpeechConfig {
                    voice_config: VoiceConfig {
                        prebuilt_voice_config: PrebuiltVoiceConfig {
                            voice_name: "Kore".to_string(),
                        },
                    },
                },
            },
        };

        let response = self
            .client
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Gemini TTS API")?;

        if !response.status().is_success() {
            let status = response.status();
            let err_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Gemini TTS API error ({}): {}", status, err_text);
        }

        let resp_body: GeminiResponse = response
            .json()
            .await
            .context("Failed to parse Gemini TTS API response")?;

        let inline_data = resp_body
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.content.parts.iter().find_map(|p| p.inline_data.as_ref()))
            .context("Gemini TTS API returned no audio data")?;

        let pcm = general_purpose::STANDARD
            .decode(&inline_data.data)
            .context("Failed to decode base64 audio data")?;

        let sample_rate = inline_data
            .mime_type
            .as_deref()
            .map(parse_sample_rate)
            .unwrap_or(DEFAULT_TTS_SAMPLE_RATE);

        Ok(pcm_to_wav(&pcm, sample_rate, 1, 16))
    }
}

// Unit tests for private helpers live here; behavioral tests of the service
// against a mocked Gemini API live in tests/ai_service_test.rs.
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, serde::Deserialize)]
    struct Doc {
        a: i32,
    }

    #[test]
    fn parse_json_completion_handles_plain_json() {
        assert_eq!(
            parse_json_completion::<Doc>("{\"a\": 1}").unwrap(),
            Doc { a: 1 }
        );
    }

    #[test]
    fn parse_json_completion_strips_markdown_fences() {
        let input = "```json\n{\"a\": 1}\n```";
        assert_eq!(parse_json_completion::<Doc>(input).unwrap(), Doc { a: 1 });
    }

    #[test]
    fn parse_json_completion_strips_surrounding_prose() {
        let input = "Sure! Here is the JSON you asked for:\n{\"a\": 2}\nLet me know if you need anything else.";
        assert_eq!(parse_json_completion::<Doc>(input).unwrap(), Doc { a: 2 });
    }

    #[test]
    fn parse_json_completion_keeps_fences_inside_strings_intact() {
        #[derive(serde::Deserialize)]
        struct Code {
            code: String,
        }
        let input = "```json\n{\"code\": \"```rust\\nfn main() {}\\n```\"}\n```";
        let parsed: Code = parse_json_completion(input).unwrap();
        assert_eq!(parsed.code, "```rust\nfn main() {}\n```");
    }

    #[test]
    fn parse_json_completion_prefers_final_answer_over_schema_example() {
        // Reasoning models may quote the schema example from the prompt and
        // think out loud before emitting the real answer in a final fence.
        let input = "The schema is { \"chapters\": [ { \"title\": \"string\", \"page_start\": 0, \"page_end\": 0 } ] }`\n\
                     Let me analyze...\n\
                     Final JSON: ```json\n{\"chapters\": [{\"title\": \"Real\", \"page_start\": 3, \"page_end\": 17}]}\n```";
        let parsed: Chapters = parse_json_completion(input).unwrap();
        assert_eq!(parsed.chapters.len(), 1);
        assert_eq!(parsed.chapters[0].title, "Real");
        assert_eq!(parsed.chapters[0].page_start, 3);
    }

    #[test]
    fn parse_json_completion_errors_on_non_json() {
        assert!(parse_json_completion::<Doc>("  not json  ").is_err());
    }

    #[test]
    fn parse_sample_rate_reads_gemini_mime_type() {
        assert_eq!(parse_sample_rate("audio/L16;codec=pcm;rate=24000"), 24_000);
        assert_eq!(
            parse_sample_rate("audio/L16; codec=pcm; rate=16000"),
            16_000
        );
    }

    #[test]
    fn parse_sample_rate_defaults_when_missing_or_invalid() {
        assert_eq!(parse_sample_rate("audio/L16"), DEFAULT_TTS_SAMPLE_RATE);
        assert_eq!(
            parse_sample_rate("audio/L16;rate=abc"),
            DEFAULT_TTS_SAMPLE_RATE
        );
    }

    #[test]
    fn pcm_to_wav_writes_valid_riff_header() {
        let pcm = [1u8, 2, 3, 4];
        let wav = pcm_to_wav(&pcm, 24_000, 1, 16);

        assert_eq!(wav.len(), 44 + pcm.len());
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        // chunk size = 36 + data length
        assert_eq!(u32::from_le_bytes(wav[4..8].try_into().unwrap()), 36 + 4);
        // PCM format, mono
        assert_eq!(u16::from_le_bytes(wav[20..22].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(wav[22..24].try_into().unwrap()), 1);
        // sample rate and byte rate (24000 * 1 channel * 2 bytes)
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 24_000);
        assert_eq!(u32::from_le_bytes(wav[28..32].try_into().unwrap()), 48_000);
        // bits per sample
        assert_eq!(u16::from_le_bytes(wav[34..36].try_into().unwrap()), 16);
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 4);
        assert_eq!(&wav[44..], &pcm);
    }
}

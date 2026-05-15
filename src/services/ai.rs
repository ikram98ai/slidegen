use crate::config::Settings;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct AIService {
    client: Client,
    api_key: String,
    text_model: String,
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

#[derive(Serialize)]
struct GeminiTTSRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
    model: String,
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

impl AIService {
    pub fn new(settings: &Settings) -> Self {
        let api_key = settings
            .gemini_api_key
            .as_ref()
            .expect("GEMINI_API_KEY must be set")
            .clone();
        let client = Client::new();
        let text_model = settings.text_model.clone();

        AIService { client, api_key, text_model }
    }

    async fn call_gemini_text(&self, model: &str, system_instruction: Option<String>, prompt: String) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            model
        );

        let body = GeminiRequest {
            system_instruction: system_instruction.map(|si| SystemInstruction {
                parts: vec![GeminiPart { text: Some(si), inline_data: None }],
            }),
            contents: vec![GeminiContent {
                parts: vec![GeminiPart { text: Some(prompt), inline_data: None }],
            }],
        };

        let response = self.client
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

        let resp_body: GeminiResponse = response.json().await.context("Failed to parse Gemini API response")?;
        
        let text = resp_body.candidates
            .as_ref()
            .and_then(|c| c.get(0))
            .and_then(|c| c.content.parts.get(0))
            .and_then(|p| p.text.clone())
            .context("Gemini API returned no text")?;

        Ok(text)
    }

    pub async fn analyze_book_toc(&self, toc: &str) -> Result<Vec<ChapterData>> {
        let system_instruction = "You are an expert educational content analyzer.
            Analyze the Table of Contents.
            Identify the main chapters or sections.
            Return a list of chapters with their titles, page_start, page_end.
            Do not hallucinate chapters if they are not clear.
            Reply ONLY with a valid JSON strictly matching this schema: { \"chapters\": [ { \"title\": \"string\", \"page_start\": 0, \"page_end\": 0 } ] }".to_string();

        let prompt = format!(
            "Table Of Contents:\n{}",
            toc
        );

        let completion = self.call_gemini_text(&self.text_model, Some(system_instruction), prompt).await?;

        // simple heuristic to extract json if wrapped in markdown
        let json_str = completion.replace("```json", "").replace("```", "").trim().to_string();
        let chapters: Chapters = serde_json::from_str(&json_str).with_context(|| format!("Failed to parse chapters JSON: {}", json_str))?;

        Ok(chapters.chapters)
    }

    pub async fn generate_slides(&self, page_text: &str) -> Result<Vec<SlideData>> {
        let system_instruction = "You are an expert presentation designer.
            Create detailed educational presentation slides.
            Reply ONLY with a valid JSON strictly matching this schema: 
            { \"slides\": [ { \"title\": \"string\", \"bullets\": [\"string\"], \"explanation\": \"string\" } ] }".to_string();

        let prompt = format!(
            "Context for the slides: \"{}\".",
            page_text
        );

        let completion = self.call_gemini_text(&self.text_model, Some(system_instruction), prompt).await?;

        let json_str = completion.replace("```json", "").replace("```", "").trim().to_string();
        let slides: Slides = serde_json::from_str(&json_str).with_context(|| format!("Failed to parse slides JSON: {}", json_str))?;

        Ok(slides.slides)
    }

    pub async fn generate_slide_audio(&self, text: &str) -> Result<String> {
        let model = "gemini-2.5-flash-preview-tts";
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            model
        );

        let body = GeminiTTSRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart { text: Some(text.to_string()), inline_data: None }],
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
            model: model.to_string(),
        };

        let response = self.client
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

        let resp_body: GeminiResponse = response.json().await.context("Failed to parse Gemini TTS API response")?;
        
        let audio_data = resp_body.candidates
            .as_ref()
            .and_then(|c| c.get(0))
            .and_then(|c| c.content.parts.get(0))
            .and_then(|p| p.inline_data.as_ref())
            .map(|id| id.data.clone())
            .context("Gemini TTS API returned no audio data")?;

        Ok(audio_data)
    }
}

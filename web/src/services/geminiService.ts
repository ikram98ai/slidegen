import { GoogleGenAI, Type, Modality } from "@google/genai";
import type { Chapter, Slide } from '../types';

// Initialize default instance for standard calls
const ai = new GoogleGenAI({ apiKey: "" });

const MODEL_NAME = 'gemini-2.5-flash';
const TTS_MODEL_NAME = 'gemini-2.5-flash-preview-tts';

// --- HELPERS FOR AUDIO PROCESSING ---

function base64ToUint8Array(base64: string): Uint8Array {
  const binaryString = atob(base64);
  const len = binaryString.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  return bytes;
}

function uint8ArrayToBase64(bytes: Uint8Array): string {
  let binary = '';
  const len = bytes.byteLength;
  for (let i = 0; i < len; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

/**
 * Adds a WAV header to raw PCM data (16-bit, 24kHz, Mono).
 */
function addWavHeader(samples: Uint8Array, sampleRate: number = 24000, numChannels: number = 1): Uint8Array {
  const buffer = new ArrayBuffer(44 + samples.length);
  const view = new DataView(buffer);

  const writeString = (view: DataView, offset: number, string: string) => {
    for (let i = 0; i < string.length; i++) {
      view.setUint8(offset + i, string.charCodeAt(i));
    }
  };

  // RIFF identifier
  writeString(view, 0, 'RIFF');
  // file length
  view.setUint32(4, 36 + samples.length, true);
  // RIFF type
  writeString(view, 8, 'WAVE');
  // format chunk identifier
  writeString(view, 12, 'fmt ');
  // format chunk length
  view.setUint32(16, 16, true);
  // sample format (1 is PCM)
  view.setUint16(20, 1, true);
  // channel count
  view.setUint16(22, numChannels, true);
  // sample rate
  view.setUint32(24, sampleRate, true);
  // byte rate (sampleRate * blockAlign)
  view.setUint32(28, sampleRate * numChannels * 2, true);
  // block align (channel count * bytes per sample)
  view.setUint16(32, numChannels * 2, true);
  // bits per sample
  view.setUint16(34, 16, true);
  // data chunk identifier
  writeString(view, 36, 'data');
  // data chunk length
  view.setUint32(40, samples.length, true);

  const bytes = new Uint8Array(buffer);
  bytes.set(samples, 44);
  
  return bytes;
}

// --- MAIN SERVICES ---

export const analyzeBookStructure = async (fileBase64: string): Promise<Chapter[]> => {
  const prompt = `
    You are an expert educational content analyzer.
    Analyze the first 50 pages of this PDF document (or the Table of Contents if available).
    Identify the main chapters or sections.
    Return a list of chapters with their titles and a very brief 1-sentence description of what the chapter covers.
    Do not hallucinate chapters if they are not clear.
  `;

  try {
    const response = await ai.models.generateContent({
      model: MODEL_NAME,
      contents: {
        parts: [
          { inlineData: { mimeType: 'application/pdf', data: fileBase64 } },
          { text: prompt }
        ]
      },
      config: {
        responseMimeType: "application/json",
        responseSchema: {
          type: Type.ARRAY,
          items: {
            type: Type.OBJECT,
            properties: {
              title: { type: Type.STRING },
              description: { type: Type.STRING }
            },
            required: ["title", "description"]
          }
        }
      }
    });

    const json = JSON.parse(response.text || "[]");
    return json.map((item: any, index: number) => ({
      id: `chap-${index}`,
      title: item.title,
      description: item.description,
      slides: undefined
    }));

  } catch (error) {
    console.error("Book analysis failed:", error);
    throw new Error("Failed to analyze book structure.");
  }
};

export const generateChapterSlides = async (fileBase64: string, chapterTitle: string, chapterDesc: string): Promise<Slide[]> => {
  const prompt = `
    You are an expert presentation designer.
    Using the attached PDF, create detailed educational presentation slides specifically for the chapter titled: "${chapterTitle}".
    Context for this chapter: "${chapterDesc}".
    
    Extract key concepts, definitions, and important points.
    Create 3 to 6 slides.
    Each slide must have:
    1. A clear title.
    2. 3-5 concise bullet points.
    3. A "explanation": A detailed, engaging paragraph (approx 80-120 words) explaining the slide's content as if a professor were speaking. This will be used for text-to-speech.
  `;

  try {
    const response = await ai.models.generateContent({
      model: MODEL_NAME,
      contents: {
        parts: [
          { inlineData: { mimeType: 'application/pdf', data: fileBase64 } },
          { text: prompt }
        ]
      },
      config: {
        responseMimeType: "application/json",
        responseSchema: {
          type: Type.ARRAY,
          items: {
            type: Type.OBJECT,
            properties: {
              title: { type: Type.STRING },
              bullets: { 
                type: Type.ARRAY, 
                items: { type: Type.STRING } 
              },
              explanation: { type: Type.STRING }
            },
            required: ["title", "bullets", "explanation"]
          }
        }
      }
    });

    return JSON.parse(response.text || "[]");
  } catch (error) {
    console.error("Chapter slide generation failed:", error);
    throw new Error("Failed to generate slides for chapter.");
  }
};

export const generateReportSlides = async (fileBase64: string): Promise<Slide[]> => {
  const prompt = `
    You are an expert analyst.
    Summarize this entire PDF report into a comprehensive slide deck for a presentation.
    Cover the main introduction, key findings, methodology (if applicable), and conclusions.
    Create between 5 to 10 slides depending on the density of information.
    
    Each slide must have:
    1. A clear title.
    2. 3-5 concise bullet points.
    3. A "explanation": A detailed, engaging paragraph (approx 80-120 words) explaining the slide's content as if a professional presenter were speaking. This will be used for text-to-speech.
  `;

  try {
    const response = await ai.models.generateContent({
      model: MODEL_NAME,
      contents: {
        parts: [
          { inlineData: { mimeType: 'application/pdf', data: fileBase64 } },
          { text: prompt }
        ]
      },
      config: {
        responseMimeType: "application/json",
        responseSchema: {
          type: Type.ARRAY,
          items: {
            type: Type.OBJECT,
            properties: {
              title: { type: Type.STRING },
              bullets: { 
                type: Type.ARRAY, 
                items: { type: Type.STRING } 
              },
              explanation: { type: Type.STRING }
            },
            required: ["title", "bullets", "explanation"]
          }
        }
      }
    });

    return JSON.parse(response.text || "[]");
  } catch (error) {
    console.error("Report generation failed:", error);
    throw new Error("Failed to generate report slides.");
  }
};

/**
 * Generate audio speech for a given text explanation.
 * WRAPS raw PCM output in a WAV header so browsers can play it.
 */
export const generateSlideAudio = async (text: string): Promise<string> => {
  try {
    const response = await ai.models.generateContent({
      model: TTS_MODEL_NAME,
      contents: {
        parts: [{ text: text }]
      },
      config: {
        responseModalities: [Modality.AUDIO],
        speechConfig: {
          voiceConfig: {
            prebuiltVoiceConfig: { voiceName: 'Kore' },
          },
        },
      },
    });

    const pcmBase64 = response.candidates?.[0]?.content?.parts?.[0]?.inlineData?.data;
    
    if (!pcmBase64) {
      throw new Error("No audio data returned from model.");
    }
    
    // Convert Raw PCM to WAV
    const pcmBytes = base64ToUint8Array(pcmBase64);
    const wavBytes = addWavHeader(pcmBytes);
    const wavBase64 = uint8ArrayToBase64(wavBytes);
    
    return wavBase64;
  } catch (error) {
    console.error("Audio generation failed:", error);
    throw new Error("Failed to generate audio.");
  }
};
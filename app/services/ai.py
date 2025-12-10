import os
import base64
import struct
from typing import List, Dict, Any
from google import genai
from google.genai import types
from pydantic import BaseModel
from app.config import settings


client = genai.Client(api_key=settings.GEMINI_API_KEY)

MODEL_NAME = 'gemini-2.5-flash'
TTS_MODEL_NAME = 'gemini-2.5-flash-preview-tts'

# --- HELPERS FOR AUDIO PROCESSING ---

def add_wav_header(pcm_data: bytes, sample_rate: int = 24000, num_channels: int = 1) -> bytes:
    """
    Adds a WAV header to raw PCM data (16-bit, 24kHz, Mono).
    """
    byte_count = len(pcm_data)
    
    # 16-bit audio = 2 bytes per sample
    bits_per_sample = 16
    block_align = num_channels * (bits_per_sample // 8)
    byte_rate = sample_rate * block_align

    # struct format: < (little-endian), 4s (string), I (uint32), ...
    # RIFF header
    header = struct.pack(
        '<4sI4s', 
        b'RIFF', 
        36 + byte_count, 
        b'WAVE'
    )

    # fmt chunk
    header += struct.pack(
        '<4sIHHIIHH',
        b'fmt ',          # Chunk ID
        16,               # Chunk Size (16 for PCM)
        1,                # Audio Format (1 = PCM)
        num_channels,     # Num Channels
        sample_rate,      # Sample Rate
        byte_rate,        # Byte Rate
        block_align,      # Block Align
        bits_per_sample   # Bits Per Sample
    )

    # data chunk
    header += struct.pack(
        '<4sI',
        b'data',
        byte_count
    )

    return header + pcm_data

# --- MAIN SERVICES ---

class LessonData(BaseModel):
    title: str
    page_start: int
    page_end: int
    description: str

class SlideData(BaseModel):
    title: str
    bullets: List[str]
    explanation: str



async def analyze_book_structure(file_base64: str) -> List[LessonData]:
    prompt = """
    You are an expert educational content analyzer.
    Analyze the first 50 pages of this PDF document (or the Table of Contents if available).
    Identify the main chapters or sections.
    Return a list of chapters with their titles, page_start, page_end, and a very brief 1-sentence description of what the chapter covers.
    Do not hallucinate chapters if they are not clear.
    """

    try:
        response = client.models.generate_content(
            model=MODEL_NAME,
            contents=[
                types.Content(
                    parts=[
                        types.Part.from_bytes(
                            data=base64.b64decode(file_base64),
                            mime_type="application/pdf"
                        ),
                        types.Part.from_text(text=prompt)
                    ]
                )
            ],
            config=types.GenerateContentConfig(
                response_mime_type="application/json",
                response_schema={
                    "type": "ARRAY",
                    "items": {
                        "type": "OBJECT",
                        "properties": {
                            "title": {"type": "STRING"},
                            "page_start": {"type": "INTEGER"},
                            "page_end": {"type": "INTEGER"},
                            "description": {"type": "STRING"}
                        },
                        "required": ["title", "page_start", "page_end", "description"]
                    }
                }
            )
        )
        
        # In Python, using response_mime_type="application/json" often parses automatically
        # depending on SDK version, but accessing .text and loading JSON is safest.
        if response.text:
            import json
            return json.loads(response.text)
        return []

    except Exception as error:
        print(f"Book analysis failed: {error}")
        raise Exception("Failed to analyze book structure.")

async def generate_chapter_slides(chapter_title: str, chapter_desc: str) -> List[SlideData]:
    prompt = f"""
    You are an expert presentation designer.
    Create detailed educational presentation slides specifically for the chapter titled: "{chapter_title}".
    Context for this chapter: "{chapter_desc}".
    """

    try:
        response = client.models.generate_content(
            model=MODEL_NAME,
            contents=[
                types.Content(
                    parts=[
                        types.Part.from_text(text=prompt)
                    ]
                )
            ],
            config=types.GenerateContentConfig(
                response_mime_type="application/json",
                response_schema={
                    "type": "ARRAY",
                    "items": {
                        "type": "OBJECT",
                        "properties": {
                            "title": {"type": "STRING"},
                            "bullets": {
                                "type": "ARRAY",
                                "items": {"type": "STRING"}
                            },
                            "explanation": {"type": "STRING"}
                        },
                        "required": ["title", "bullets", "explanation"]
                    }
                }
            )
        )

        if response.text:
            import json
            return json.loads(response.text)
        return []

    except Exception as error:
        print(f"Chapter slide generation failed: {error}")
        raise Exception("Failed to generate slides for chapter.")

def generate_report_slides(file_base64: str) -> List[Dict[str, Any]]:
    prompt = """
    You are an expert analyst.
    Summarize this entire PDF report into a comprehensive slide deck for a presentation.
    Cover the main introduction, key findings, methodology (if applicable), and conclusions.
    Create between 5 to 10 slides depending on the density of information.
    """

    try:
        response = client.models.generate_content(
            model=MODEL_NAME,
            contents=[
                types.Content(
                    parts=[
                        types.Part.from_bytes(
                            data=base64.b64decode(file_base64),
                            mime_type="application/pdf"
                        ),
                        types.Part.from_text(text=prompt)
                    ]
                )
            ],
            config=types.GenerateContentConfig(
                response_mime_type="application/json",
                response_schema={
                    "type": "ARRAY",
                    "items": {
                        "type": "OBJECT",
                        "properties": {
                            "title": {"type": "STRING"},
                            "bullets": {
                                "type": "ARRAY",
                                "items": {"type": "STRING"}
                            },
                            "explanation": {"type": "STRING"}
                        },
                        "required": ["title", "bullets", "explanation"]
                    }
                }
            )
        )

        if response.text:
            import json
            return json.loads(response.text)
        return []

    except Exception as error:
        print(f"Report generation failed: {error}")
        raise Exception("Failed to generate report slides.")

def generate_slide_audio(text: str) -> str:
    """
    Generate audio speech for a given text explanation.
    WRAPS raw PCM output in a WAV header so browsers can play it.
    Returns Base64 encoded WAV string.
    """
    try:
        response = client.models.generate_content(
            model=TTS_MODEL_NAME,
            contents=[
                types.Content(
                    parts=[types.Part.from_text(text=text)]
                )
            ],
            config=types.GenerateContentConfig(
                response_modalities=["AUDIO"],
                speech_config=types.SpeechConfig(
                    voice_config=types.VoiceConfig(
                        prebuilt_voice_config=types.PrebuiltVoiceConfig(
                            voice_name="Kore"
                        )
                    )
                )
            )
        )

        # Retrieve inline data
        # The Python SDK usually provides the raw bytes in inline_data.data
        # We need to find the part containing inline_data
        pcm_bytes = None
        if response.candidates and response.candidates[0].content.parts:
            for part in response.candidates[0].content.parts:
                if part.inline_data:
                    pcm_bytes = part.inline_data.data
                    break
        
        if not pcm_bytes:
            raise Exception("No audio data returned from model.")

        # Convert Raw PCM to WAV (add header)
        wav_bytes = add_wav_header(pcm_bytes)
        
        # Convert back to base64 string to match the TypeScript return type
        wav_base64 = base64.b64encode(wav_bytes).decode('utf-8')
        
        return wav_base64

    except Exception as error:
        print(f"Audio generation failed: {error}")
        raise Exception("Failed to generate audio.")
from typing import List
from google import genai
from pydantic import BaseModel
from app.config import settings
from langchain.chat_models import init_chat_model

client = genai.Client(api_key=settings.GEMINI_API_KEY)


MODEL_NAME = 'google_genai:gemini-2.0-flash'
TTS_MODEL_NAME = 'gemini-2.5-flash-preview-tts'

llm = init_chat_model(model=MODEL_NAME, api_key=settings.GEMINI_API_KEY)
tts = init_chat_model(model=TTS_MODEL_NAME, api_key=settings.GEMINI_API_KEY) 


# --- MAIN SERVICES ---

class LessonData(BaseModel):
    title: str
    page_start: int
    page_end: int

async def analyze_book_toc(toc: str) -> List[LessonData]:
    prompt = f"""
    You are an expert educational content analyzer.
    Analyze the Table of Contents.
    Identify the main chapters or sections.
    Return a list of chapters with their titles, page_start, page_end.
    Do not hallucinate chapters if they are not clear.

    Table Of Contents:
    {toc}
    """

    try:
        analyze_toc = llm.with_structured_output(List[LessonData])
        response = await analyze_toc.ainvoke(prompt) 
        return response
    except Exception as error:
        print(f"Book analysis failed: {error}")
        raise Exception("Failed to analyze book structure.")



class SlideData(BaseModel):
    title: str
    bullets: List[str]
    explanation: str
    
async def generate_chapter_slides(chapter_title: str, chapter_desc: str) -> List[SlideData]:
    prompt = f"""
    You are an expert presentation designer.
    Create detailed educational presentation slides specifically for the chapter titled: "{chapter_title}".
    Context for this chapter: "{chapter_desc}".
    """

    try:
        generate_slides = llm.with_structured_output(List[SlideData])
        response = await generate_slides.ainvoke(prompt)

        return response

    except Exception as error:
        print(f"Chapter slide generation failed: {error}")
        raise Exception("Failed to generate slides for chapter.")



async def generate_slide_audio(text: str) -> str:
    """
    Generate audio speech for a given text explanation.
    WRAPS raw PCM output in a WAV header so browsers can play it.
    Returns Base64 encoded WAV string.
    """
    try:
        response = tts.ainvoke(text)
        # Base64 encoded binary data of the audio
        wav_data = response.additional_kwargs.get("audio")

        return wav_data

    except Exception as error:
        print(f"Audio generation failed: {error}")
        raise Exception("Failed to generate audio.")
# app/api/v1/endpoints/lessons.py
from typing import List
from fastapi import APIRouter, Depends, HTTPException, status, BackgroundTasks
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select
from sqlalchemy.orm import selectinload
from typing import Optional
from app.db import get_db, User, Lesson, Slide
from app.schemas import LessonUpdate, LessonResponse, SlideResponse
from app.api.deps import get_current_active_user
from app.services import ai, storage
import uuid

router = APIRouter()

@router.patch("/{lesson_id}", response_model=LessonResponse)
async def update_lesson(
    lesson_id: int,
    lesson_update: LessonUpdate,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_active_user)
):
    """Update lesson - only owner"""
    # Get lesson with subject
    result = await db.execute(
        select(Lesson)
        .options(selectinload(Lesson.subject))
        .where(Lesson.id == lesson_id)
    )
    lesson = result.scalar_one_or_none()
    
    if not lesson:
        raise HTTPException(status_code=404, detail="Lesson not found")
    
    # Check ownership
    if current_user.id != lesson.subject.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
    # Update lesson
    update_data = lesson_update.dict(exclude_unset=True)
    for field, value in update_data.items():
        setattr(lesson, field, value)
    
    db.add(lesson)
    await db.commit()
    await db.refresh(lesson)
    
    return lesson

@router.delete("/{lesson_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_lesson(
    lesson_id: int,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_active_user)
):
    """Delete lesson - only owner"""
    # Get lesson with subject
    result = await db.execute(
        select(Lesson)
        .options(selectinload(Lesson.subject))
        .where(Lesson.id == lesson_id)
    )
    lesson = result.scalar_one_or_none()
    
    if not lesson:
        raise HTTPException(status_code=404, detail="Lesson not found")
    
    # Check ownership
    if current_user.id != lesson.subject.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
    # Delete lesson (cascade will delete slides)
    await db.delete(lesson)
    await db.commit()

@router.post("/{lesson_id}/slides/generate", status_code=status.HTTP_202_ACCEPTED)
async def generate_slides(
    lesson_id: int,
    background_tasks: BackgroundTasks,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_active_user)
):
    """Generate slides for a lesson (AI)"""
    # Get lesson with subject
    result = await db.execute(
        select(Lesson)
        .options(selectinload(Lesson.subject))
        .where(Lesson.id == lesson_id)
    )
    lesson = result.scalar_one_or_none()
    
    if not lesson:
        raise HTTPException(status_code=404, detail="Lesson not found")
    
    # Check ownership
    if current_user.id != lesson.subject.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
    # Start background task
    background_tasks.add_task(
        generate_slides_background,
        lesson_id,
        lesson.title
    )
    
    return {"message": "Slide generation started"}

async def generate_slides_background(lesson_id: int, lesson_title: str):
    """Background task to generate slides"""
    from app.db import AsyncSessionLocal
    
    async with AsyncSessionLocal() as db:
        try:
            # Get lesson text (simplified - in reality, extract from PDF pages)
            # For now, we'll use a placeholder
            lesson_text = f"Content for lesson: {lesson_title}"
            
            # Generate slides using AI
            slides_data = await ai.generate_chapter_slides(lesson_title, lesson_text)
            
            if not slides_data:
                return
            
            # Create slides
            for i, slide_data in enumerate(slides_data):
                # Generate voice for explanation
                voice_filename = f"{uuid.uuid4()}_{slide_data.title[:20].replace(' ', '_')}.mp3"
                wav_base64 = await ai.generate_slide_audio(
                    slide_data["explanation"],
                )
                
                is_upload = storage.upload_file(wav_base64, voice_filename)
                if not is_upload:
                    raise HTTPException(
                        status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                        detail="Failed to upload audio file to s3."
                    )

                # Create slide
                slide = Slide(
                    lesson_id=lesson_id,
                    title=slide_data["title"],
                    points=slide_data["points"],
                    explanation=slide_data["explanation"],
                    voice_url=voice_filename,
                    order_index=i
                )
                db.add(slide)
            
            await db.commit()
            
        except Exception as e:
            import traceback
            print(f"Error generating slides for lesson {lesson_id}: {e}")
            traceback.print_exc()

@router.get("/{lesson_id}/slides", response_model=List[SlideResponse])
async def get_lesson_slides(
    lesson_id: int,
    db: AsyncSession = Depends(get_db),
    current_user: Optional[User] = Depends(get_current_active_user)
):
    """Get all slides for a lesson"""
    # Get lesson with subject
    result = await db.execute(
        select(Lesson)
        .options(selectinload(Lesson.subject))
        .where(Lesson.id == lesson_id)
    )
    lesson = result.scalar_one_or_none()
    
    if not lesson:
        raise HTTPException(status_code=404, detail="Lesson not found")
    
    # Check permissions
    if not lesson.subject.is_public and (not current_user or current_user.id != lesson.subject.user_id):
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
    # Get slides
    result = await db.execute(
        select(Slide)
        .where(Slide.lesson_id == lesson_id)
        .order_by(Slide.order_index)
    )
    slides = result.scalars().all()
    
    return slides
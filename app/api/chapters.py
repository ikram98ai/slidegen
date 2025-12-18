# app/api/v1/endpoints/chapters.py
from typing import List
from fastapi import APIRouter, Depends, HTTPException, status, BackgroundTasks
import uuid
from app.models import User, Subject, Chapter, Slide
from app.schemas import ChapterCreate, ChapterUpdate, ChapterResponse, SlideResponse
from app.schemas import SlideUpdate
from app.services.auth import get_current_user
from app.services import bg_tasks
from app.services.storage import get_presigned_url

router = APIRouter()


@router.post("/", response_model=ChapterResponse)
async def create_chapter(chapter: ChapterCreate, current_user: User = Depends(get_current_user)):
    """Create chapter - only owner of the subject"""
    # Get subject
    try:
        subject = Subject.get(chapter.subject_id)
    except Subject.DoesNotExist:
        raise HTTPException(
            status_code=404, detail="Subject not found to create chapter for"
        )

    # Create chapter
    db_chapter = Chapter(
        id=str(uuid.uuid4()),
        user_id=current_user.id,
        subject_id=subject.id,
        title=chapter.title,
        page_start=chapter.page_start,
        page_end=chapter.page_end,
        order_index=chapter.order_index,
    )

    db_chapter.save()

    return db_chapter


@router.patch("/{subject_id}/{chapter_id}", response_model=ChapterResponse)
async def update_chapter(
    subject_id: str,
    chapter_id: str,
    chapter_update: ChapterUpdate,
    current_user: User = Depends(get_current_user),
):
    """Update chapter - only owner"""
    # Get subject for ownership check
    try:
        subject = Subject.get(subject_id)
    except Subject.DoesNotExist:
        raise HTTPException(status_code=404, detail="Subject not found")
    # Get chapter
    try:
        chapter = Chapter.get(subject_id, chapter_id)
    except Chapter.DoesNotExist:
        raise HTTPException(status_code=404, detail="Chapter not found")

    if current_user.id != subject.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")

    # Update chapter
    update_data = chapter_update.model_dump(exclude_unset=True)
    actions = []
    for field, value in update_data.items():
        setattr(chapter, field, value)
        actions.append(getattr(Chapter, field).set(value))

    if actions:
        chapter.update(actions=actions)

    return chapter


@router.delete("/{subject_id}/{chapter_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_chapter(
    subject_id:str, chapter_id: str, current_user: User = Depends(get_current_user)
):
    """Delete chapter - only owner"""
    # Get chapter
    try:
        chapter = Chapter.get(subject_id, chapter_id)
    except Chapter.DoesNotExist:
        raise HTTPException(status_code=404, detail="Chapter not found")


    # Check ownership
    if current_user.id != chapter.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")

    # Delete slides in the chapter
    slides = Slide.query(Slide.chapter_id)
    with Slide.batch_write() as batch:
        for slide in slides:
            batch.delete(slide)

    # Delete chapter
    chapter.delete()


@router.post("/{subject_id}/{chapter_id}/slides/generate", status_code=status.HTTP_202_ACCEPTED)
async def generate_slides(
    subject_id: str,
    chapter_id: str,
    background_tasks: BackgroundTasks,
    current_user: User = Depends(get_current_user),
):
    """Generate slides for a chapter (AI)"""

    # Get chapter
    try:
        chapter = Chapter.get(subject_id, chapter_id)
    except Chapter.DoesNotExist:
        raise HTTPException(status_code=404, detail="Chapter not found")


    # Check ownership
    if current_user.id != chapter.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")

    # Start background task
    background_tasks.add_task(bg_tasks.generate_slides_background, current_user.id, subject_id, chapter)

    return {"message": "Slide generation started"}


@router.get("/{chapter_id}/slides", response_model=List[SlideResponse])
async def get_chapter_slides(chapter_id: str):
    """Get all slides for a chapter"""

    # Get slides
    slides = Slide.query(chapter_id)
    slides = sorted(slides, key=lambda s: s.order_index)
    
    # Generate presigned urls
    for slide in slides:
        if slide.voice_url:
            slide.voice_url = get_presigned_url(slide.voice_url)


    return slides



@router.patch("/{chpater_id}/slides/{slide_id}", response_model=SlideResponse)
async def update_slide(
    chapter_id: str,
    slide_id: str,
    slide_update: SlideUpdate,
    current_user: User = Depends(get_current_user),
):
    """Update slide - only owner"""
    # Get slide
    try:
        slide = Slide.get(chapter_id, slide_id)
    except Slide.DoesNotExist:
        raise HTTPException(status_code=404, detail="Slide not found")

    # Check ownership
    if current_user.id != slide.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")

    # Update slide
    update_data = slide_update.model_dump(exclude_unset=True)
    actions = []
    for field, value in update_data.items():
        setattr(slide, field, value)
        actions.append(getattr(Slide, field).set(value))

    if actions:
        slide.update(actions=actions)

    return slide


@router.delete("/{chapter_id}/slides/{slide_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_slide(chapter_id:str, slide_id: str, current_user: User = Depends(get_current_user)):
    """Delete slide - only owner"""

    # Get slide
    try:
        slide = Slide.get(chapter_id, slide_id)
    except Slide.DoesNotExist:
        raise HTTPException(status_code=404, detail="Slide not found")

    if current_user.id != slide.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")

    # Delete slide
    slide.delete()


# app/api/v1/endpoints/chapters.py
from typing import List
from fastapi import APIRouter, Depends, HTTPException, status, BackgroundTasks
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select
from sqlalchemy.orm import selectinload
from app.db import get_db, User, Subject, Chapter, Slide
from app.schemas import ChapterCreate, ChapterUpdate, ChapterResponse, SlideResponse
from app.api.deps import get_current_user
from app.services import bg_tasks

router = APIRouter()


@router.post("/", response_model=ChapterResponse)
async def create_chapter(
    chapter: ChapterCreate,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user)
):
    """Create chapter - only owner of the subject"""
    # Get subject
    result = await db.execute(
        select(Subject).where(Subject.id == chapter.subject_id)
    )
    subject = result.scalar_one_or_none()
    
    if not subject:
        raise HTTPException(status_code=404, detail="Subject not found to create chapter for")
    
    # Check ownership
    if current_user.id != subject.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
    # Create chapter
    db_chapter = Chapter(subject_id=chapter.subject_id, 
                         title=chapter.title, 
                         page_start=chapter.page_start, 
                         page_end=chapter.page_end,
                         order_index=chapter.order_index)
    
    db.add(db_chapter)
    await db.commit()
    await db.refresh(db_chapter)
    
    return db_chapter

@router.patch("/{chapter_id}", response_model=ChapterResponse)
async def update_chapter(
    chapter_id: int,
    chapter_update: ChapterUpdate,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user)
):
    """Update chapter - only owner"""
    # Get chapter with subject
    result = await db.execute(
        select(Chapter)
        .options(selectinload(Chapter.subject))
        .where(Chapter.id == chapter_id)
    )
    chapter = result.scalar_one_or_none()
    
    if not chapter:
        raise HTTPException(status_code=404, detail="Chapter not found")
    
    # Check ownership
    if current_user.id != chapter.subject.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
    # Update chapter
    update_data = chapter_update.dict(exclude_unset=True)
    for field, value in update_data.items():
        setattr(chapter, field, value)
    
    db.add(chapter)
    await db.commit()
    await db.refresh(chapter)
    
    return chapter

@router.delete("/{chapter_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_chapter(
    chapter_id: int,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user)
):
    """Delete chapter - only owner"""
    # Get chapter with subject
    result = await db.execute(
        select(Chapter)
        .options(selectinload(Chapter.subject))
        .where(Chapter.id == chapter_id)
    )
    chapter = result.scalar_one_or_none()
    
    if not chapter:
        raise HTTPException(status_code=404, detail="Chapter not found")
    
    # Check ownership
    if current_user.id != chapter.subject.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
    # Delete chapter (cascade will delete slides)
    await db.delete(chapter)
    await db.commit()
    

@router.post("/{chapter_id}/slides/generate", status_code=status.HTTP_202_ACCEPTED)
async def generate_slides(
    chapter_id: int,
    background_tasks: BackgroundTasks,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user)
):
    """Generate slides for a chapter (AI)"""
    # Get chapter with subject
    result = await db.execute(
        select(Chapter)
        .options(selectinload(Chapter.subject))
        .where(Chapter.id == chapter_id)
    )
    chapter = result.scalar_one_or_none()
    
    if not chapter:
        raise HTTPException(status_code=404, detail="Chapter not found")
    
    # Check ownership
    if current_user.id != chapter.subject.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
    # Start background task
    background_tasks.add_task(bg_tasks.generate_slides_background, chapter)
    
    return {"message": "Slide generation started"}

@router.get("/{chapter_id}/slides", response_model=List[SlideResponse])
async def get_chapter_slides(
    chapter_id: int,
    db: AsyncSession = Depends(get_db),
):
    """Get all slides for a chapter"""
    # Get chapter with subject
    result = await db.execute(
        select(Chapter)
        .options(selectinload(Chapter.subject))
        .where(Chapter.id == chapter_id)
    )
    chapter = result.scalar_one_or_none()
    
    if not chapter:
        raise HTTPException(status_code=404, detail="Chapter not found")

    # Get slides
    result = await db.execute(
        select(Slide)
        .where(Slide.chapter_id == chapter_id)
        .order_by(Slide.order_index)
    )
    slides = result.scalars().all()
    
    return slides
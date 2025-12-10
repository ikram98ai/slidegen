# app/api/v1/endpoints/slides.py
from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select
from sqlalchemy.orm import selectinload

from app.db import get_db, User, Slide, Lesson
from app.schemas import SlideUpdate, SlideResponse
from app.api.deps import get_current_active_user

router = APIRouter()

@router.patch("/{slide_id}", response_model=SlideResponse)
async def update_slide(
    slide_id: int,
    slide_update: SlideUpdate,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_active_user)
):
    """Update slide - only owner"""
    # Get slide with lesson and subject
    result = await db.execute(
        select(Slide)
        .options(
            selectinload(Slide.lesson).selectinload(Lesson.subject)
        )
        .where(Slide.id == slide_id)
    )
    slide = result.scalar_one_or_none()
    
    if not slide:
        raise HTTPException(status_code=404, detail="Slide not found")
    
    # Check ownership
    if current_user.id != slide.lesson.subject.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
    # Update slide
    update_data = slide_update.dict(exclude_unset=True)
    for field, value in update_data.items():
        setattr(slide, field, value)
    
    db.add(slide)
    await db.commit()
    await db.refresh(slide)
    
    return slide

@router.delete("/{slide_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_slide(
    slide_id: int,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_active_user)
):
    """Delete slide - only owner"""
    # Get slide with lesson and subject
    result = await db.execute(
        select(Slide)
        .options(
            selectinload(Slide.lesson).selectinload(Lesson.subject)
        )
        .where(Slide.id == slide_id)
    )
    slide = result.scalar_one_or_none()
    
    if not slide:
        raise HTTPException(status_code=404, detail="Slide not found")
    
    # Check ownership
    if current_user.id != slide.lesson.subject.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
    # Delete slide
    await db.delete(slide)
    await db.commit()
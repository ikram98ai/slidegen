# app/api/subjects.py
from typing import List, Optional
from fastapi import APIRouter, Depends, HTTPException, status, Query, UploadFile, File, Form, BackgroundTasks
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select, update, union_all
import uuid
from pathlib import Path

from app.db import get_db, User, Subject, SubjectType, Lesson
from app.schemas import  SubjectUpdate, SubjectResponse, LessonResponse
from app.api.deps import get_current_active_user
from app.services import storage, bg_tasks

router = APIRouter()

@router.get("/", response_model=List[SubjectResponse])
async def list_subjects(
    skip: int = Query(0, ge=0),
    limit: int = Query(100, ge=1, le=100),
    db: AsyncSession = Depends(get_db),
    current_user: Optional[User] = Depends(get_current_active_user)
):
    """List all subjects (public or user's own)"""
    if current_user:
        # Get user's subjects and public subjects
        user_subjects_query = select(Subject).where(Subject.user_id == current_user.id)
        public_subjects_query = select(Subject).where(Subject.is_public == True)
        
        # Combine queries
        combined = union_all(user_subjects_query, public_subjects_query).alias()
        
        query = select(Subject).select_from(combined).offset(skip).limit(limit).order_by(Subject.created_at.desc())
    else:
        # Only public subjects for non-authenticated users
        query = select(Subject).where(Subject.is_public == True).offset(skip).limit(limit).order_by(Subject.created_at.desc())
    
    result = await db.execute(query)
    subjects = result.scalars().all()
    
    return subjects

ALLOWED_EXTENSIONS=['.pdf', '.txt', '.docs']
MAX_UPLOAD_SIZE = 10 * 1024 * 1024  # 10 MB
@router.post("/upload", response_model=SubjectResponse, status_code=status.HTTP_201_CREATED)
async def upload_subject(
    background_tasks: BackgroundTasks,
    title: str = Form(...),
    is_public: bool = Form(False),
    type: SubjectType = Form(...),
    file: UploadFile = File(...),
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_active_user)
):
    """Upload a new subject file"""
    # Validate file type
    file_ext = Path(file.filename).suffix.lower()
    if file_ext not in ALLOWED_EXTENSIONS:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"File type not allowed. Allowed types: {ALLOWED_EXTENSIONS}"
        )
    
    # Check file size
    file.file.seek(0, 2)  # Seek to end
    file_size = file.file.tell()
    file.file.seek(0)  # Reset to beginning
    if file_size > MAX_UPLOAD_SIZE:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"File too large. Maximum size is {MAX_UPLOAD_SIZE // 1024 // 1024}MB"
        )
    
    # Save file
    file_s3path = f"subjects/{current_user.id}/{uuid.uuid4()}_{file.filename}"
    is_upload = await storage.upload_file(file, file.filename)
    if not is_upload:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to upload {file_ext} file to s3."
        )
    # Create subject
    subject = Subject(
        user_id=current_user.id,
        title=title,
        file_path=file_s3path,
        is_public=is_public,
        type=type,
        processing_status="pending"
    )
    
    db.add(subject)
    await db.commit()
    await db.refresh(subject)
    
    # Process file in background
    background_tasks.add_task(bg_tasks.process_subject_background, subject.id, file_s3path)
    
    return subject

@router.get("/{subject_id}/lessons", response_model=List[LessonResponse])
async def get_subject_lessons(
    subject_id: int,
    db: AsyncSession = Depends(get_db),
    current_user: Optional[User] = Depends(get_current_active_user)
):
    """Get all lessons for a subject"""
    # Get subject
    result = await db.execute(select(Subject).where(Subject.id == subject_id))
    subject = result.scalar_one_or_none()
    
    if not subject:
        raise HTTPException(status_code=404, detail="Subject not found")
    
    # Check permissions
    if not subject.is_public and (not current_user or current_user.id != subject.user_id):
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
    # Get lessons
    result = await db.execute(
        select(Lesson)
        .where(Lesson.subject_id == subject_id)
        .order_by(Lesson.order_index)
    )
    lessons = result.scalars().all()
    
    return lessons

@router.patch("/{subject_id}", response_model=SubjectResponse)
async def update_subject(
    subject_id: int,
    subject_update: SubjectUpdate,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_active_user)
):
    """Update subject - only owner"""
    # Get subject
    result = await db.execute(select(Subject).where(Subject.id == subject_id))
    subject = result.scalar_one_or_none()
    
    if not subject:
        raise HTTPException(status_code=404, detail="Subject not found")
    
    # Check ownership
    if current_user.id != subject.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
    # Update subject
    update_data = subject_update.dict(exclude_unset=True)
    for field, value in update_data.items():
        setattr(subject, field, value)
    
    db.add(subject)
    await db.commit()
    await db.refresh(subject)
    
    return subject

@router.delete("/{subject_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_subject(
    subject_id: int,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_active_user)
):
    """Delete subject - only owner"""
    # Get subject
    result = await db.execute(select(Subject).where(Subject.id == subject_id))
    subject = result.scalar_one_or_none()
    
    if not subject:
        raise HTTPException(status_code=404, detail="Subject not found")
    
    # Check ownership
    if current_user.id != subject.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
 
    # Delete subject (cascade will delete lessons and slides)
    await db.delete(subject)
    await db.commit()
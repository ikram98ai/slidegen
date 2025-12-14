# app/api/subjects.py
from typing import List
from fastapi import APIRouter, Depends, HTTPException, status, Query, UploadFile, File, Form, BackgroundTasks
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.orm import selectinload
from sqlalchemy import select

import uuid
from pathlib import Path

from app.db import get_db, User, Subject, SubjectType, Chapter
from app.schemas import  SubjectUpdate, SubjectResponse, SubjectDetailResponse
from app.api.deps import get_current_user
from app.services import storage, bg_tasks

router = APIRouter()

@router.get("/", response_model=List[SubjectResponse])
async def list_subjects(
    skip: int = Query(0, ge=0),
    limit: int = Query(100, ge=1, le=100),
    db: AsyncSession = Depends(get_db),
):
    """List all subjects """
    # Only public subjects for non-authenticated users
    query = select(Subject).options(selectinload(Subject.owner)).where(Subject.is_public == True)\
                           .offset(skip).limit(limit).order_by(Subject.created_at.desc())
    
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
    current_user: User = Depends(get_current_user)
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
    file_bytes = await file.read()
    is_upload = storage.upload_file(file_bytes, file_s3path)
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

@router.get("/{subject_id}", response_model=SubjectResponse)
async def get_subject(
    subject_id: int,
    db: AsyncSession = Depends(get_db),
):
    """Get all chapters for a subject"""
    # Get subject
    result = await db.execute(select(Subject).where(Subject.id == subject_id))
    subject = result.scalar_one_or_none()
    
    if not subject:
        raise HTTPException(status_code=404, detail="Subject not found")
    
    return subject

@router.get("/{subject_id}/chapters", response_model=SubjectDetailResponse)
async def get_subject_chapters(
    subject_id: int,
    db: AsyncSession = Depends(get_db),
):
    """Get all chapters for a subject"""
    # Get subject
    result = await db.execute(select(Subject).where(Subject.id == subject_id))
    subject = result.scalar_one_or_none()
    
    if not subject:
        raise HTTPException(status_code=404, detail="Subject not found")
    

    # Get chapters
    result = await db.execute(
        select(Chapter)
        .where(Chapter.subject_id == subject_id)
        .order_by(Chapter.order_index)
    )
    chapters = result.scalars().all()

    response = SubjectDetailResponse(
                id= subject.id, 
                user_id= subject.user_id,
                title= subject.title,
                is_public= subject.is_public,
                type= subject.type,
                file_path= subject.file_path,
                processing_status= subject.processing_status,
                created_at= subject.created_at,
                chapters= chapters
            )
    return response

@router.patch("/{subject_id}", response_model=SubjectResponse)
async def update_subject(
    subject_id: int,
    subject_update: SubjectUpdate,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user)
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
    current_user: User = Depends(get_current_user)
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
    
 
    # Delete subject (cascade will delete chapters and slides)
    await db.delete(subject)
    await db.commit()
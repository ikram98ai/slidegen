# app/api/subjects.py
from typing import List
from fastapi import (
    APIRouter,
    BackgroundTasks,
    HTTPException,
    status,
    UploadFile,
    Depends,
    File,
    Form,
    Query,
)
import uuid
from pathlib import Path

from app.models import User, Subject, SubjectType, Chapter, Slide
from app.schemas import SubjectUpdate, SubjectResponse, SubjectDetailResponse
from app.services.auth import get_current_user, get_current_user_or_anonymous
from app.services import storage, bg_tasks

router = APIRouter()


@router.get("", response_model=List[SubjectResponse])
async def list_subjects(
    skip: int = Query(0, ge=0),
    limit: int = Query(100, ge=1, le=100),
    current_user: User = Depends(get_current_user_or_anonymous)
):
    """List all subjects"""
    # Only public subjects for non-authenticated users
    if current_user:
        subjects = Subject.scan(limit=100)
    else:
        subjects = Subject.scan(Subject.is_public == True, limit=100)
    subjects = sorted(subjects, key=lambda s: s.created_at, reverse=True)
    return subjects[skip : skip + limit]


ALLOWED_EXTENSIONS = [".pdf", ".txt", ".docs"]
MAX_UPLOAD_SIZE = 10 * 1024 * 1024  # 10 MB

@router.post( "/upload", response_model=SubjectResponse, status_code=status.HTTP_201_CREATED)
async def upload_subject(
    background_tasks: BackgroundTasks,
    title: str = Form(...),
    is_public: bool = Form(False),
    type: SubjectType = Form(...),
    file: UploadFile = File(...),
    current_user: User = Depends(get_current_user),
):
    """Upload a new subject file"""
    # Validate file type
    file_ext = Path(file.filename).suffix.lower()
    if file_ext not in ALLOWED_EXTENSIONS:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"File type not allowed. Allowed types: {ALLOWED_EXTENSIONS}",
        )

    # Check file size
    file.file.seek(0, 2)  # Seek to end
    file_size = file.file.tell()
    file.file.seek(0)  # Reset to beginning
    if file_size > MAX_UPLOAD_SIZE:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"File too large. Maximum size is {MAX_UPLOAD_SIZE // 1024 // 1024}MB",
        )

    # Save file
    file_s3path = f"subjects/{current_user.id}/{uuid.uuid4()}_{file.filename}"
    file_bytes = await file.read()
    is_upload = storage.upload_file(file_bytes, file_s3path)
    if not is_upload:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to upload {file_ext} file to s3.",
        )
    # Create subject
    subject = Subject(
        id=str(uuid.uuid4()),
        user_id=current_user.id,
        title=title,
        file_path=file_s3path,
        is_public=is_public,
        type=type,
        processing_status="pending",
    )

    subject.save()

    # Process file in background
    background_tasks.add_task(bg_tasks.process_subject_background, current_user.id, subject.id, file_s3path)

    return subject

@router.get("/{subject_id}", response_model=SubjectResponse)
async def get_subject(subject_id: str):
    """Get a subject"""
    # Get subject
    try:
        subject = Subject.get(subject_id)
    except Subject.DoesNotExist:
        raise HTTPException(status_code=404, detail="Subject not found")
    return subject


@router.get("/{subject_id}/chapters", response_model=SubjectDetailResponse)
async def get_subject_chapters(subject_id: str):
    """Get all chapters for a subject"""
    # Get subject
    try:
        subject = Subject.get(subject_id)
    except Subject.DoesNotExist:
        raise HTTPException(status_code=404, detail="Subject not found")

    # Get chapters
    chapters = Chapter.query(subject_id)
    chapters = sorted(chapters, key=lambda c: c.order_index)

    response = SubjectDetailResponse(
        id=subject.id,
        user_id=subject.user_id,
        title=subject.title,
        is_public=subject.is_public,
        type=subject.type,
        file_path=subject.file_path,
        processing_status=subject.processing_status,
        created_at=subject.created_at,
        chapters=chapters,
    )
    return response


@router.patch("/{subject_id}", response_model=SubjectResponse)
async def update_subject(
    subject_id: str,
    subject_update: SubjectUpdate,
    current_user: User = Depends(get_current_user),
):
    """Update subject - only owner"""
    # Get subject
    try:
        subject = Subject.get(subject_id)
    except Subject.DoesNotExist:
        raise HTTPException(status_code=404, detail="Subject not found")

    # Check ownership
    if current_user.id != subject.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")

    # Update subject
    update_data = subject_update.model_dump(exclude_unset=True)
    actions = []
    for field, value in update_data.items():
        setattr(subject, field, value)
        actions.append(getattr(Subject, field).set(value))

    if actions:
        subject.update(actions=actions)

    return subject


@router.delete("/{subject_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_subject(subject_id: str, current_user: User = Depends(get_current_user)):
    """Delete subject - only owner"""
    # Get subject
    try:
        subject = Subject.get(subject_id)
    except Subject.DoesNotExist:
        raise HTTPException(status_code=404, detail="Subject not found")

    # Check ownership
    if current_user.id != subject.user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")
    try:
        # Delete chapters and slides
        chapters = list(Chapter.query(subject_id))
        with Chapter.batch_write() as ch_batch:
            for chapter in chapters:
                slides = list(Slide.query(chapter.id))
                with Slide.batch_write() as batch:
                    for slide in slides:
                        batch.delete(slide)
                ch_batch.delete(chapter)
        subject.delete()
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"server error: {str(e)}")

    # Delete subjec

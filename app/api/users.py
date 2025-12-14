# app/api/users.py
from typing import Optional, List
from fastapi import APIRouter, Depends, HTTPException, status, Query, UploadFile, File, Form
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select
import uuid
import json
from app.services import storage
from app.db import get_db, Subject, User
from app.schemas import UserResponse, UserUpdate, SubjectResponse
from app.api.deps import get_current_user

router = APIRouter()

@router.get("/{user_id}", response_model=UserResponse)
async def get_user(
    user_id: int,
    db: AsyncSession = Depends(get_db),
):
    """Get user by ID"""
    result = await db.execute(select(User).where(User.id == user_id))
    user = result.scalar_one_or_none()
    
    if not user:
        raise HTTPException(status_code=404, detail="User not found")
    
    # Only show active users
    if not user.is_active:
        raise HTTPException(status_code=404, detail="User not found")
    
    return user

@router.get("/{user_id}/subjects", response_model=List[SubjectResponse])
async def get_user_subjects(
    user_id: int,
    skip: int = Query(0, ge=0),
    limit: int = Query(100, ge=1, le=100),
    db: AsyncSession = Depends(get_db),
    current_user: Optional[User] = Depends(get_current_user)
):
    """Get all subjects for a user"""
    # Check if user exists
    result = await db.execute(select(User).where(User.id == user_id))
    user = result.scalar_one_or_none()
    if not user:
        raise HTTPException(status_code=404, detail="User not found")
    
    # Check permissions (only owner or public subjects)
    if current_user and current_user.id == user_id:
        # Owner can see all their subjects
        query = select(Subject).where(Subject.user_id == user_id)
    else:
        # Others can only see public subjects
        query = select(Subject).where(
            Subject.user_id == user_id, Subject.is_public == True
        )
    
    query = query.offset(skip).limit(limit).order_by(Subject.created_at.desc())
    result = await db.execute(query)
    subjects = result.scalars().all()
    
    return subjects

@router.patch("/me", response_model=UserResponse)
async def update_user(
    user_update: str = Form(...),
    dp: Optional[UploadFile] = File(None),
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user)
):
    """Update user - only owner"""

    user_update_data = UserUpdate.model_validate(json.loads(user_update))
    

    if dp and not dp.filename.lower().endswith((".jpg", ".jpeg", ".png")):
        raise HTTPException(
            status_code=400,
            detail="Only JPG, JPEG, PNG files are allowed for avatar",
        )

    if dp and dp.size > 5 * 1024 * 1024:  # 5MB limit
        raise HTTPException(
            status_code=400, detail="Avatar file size too large (max 5MB)"
        )

    update_data = user_update_data.model_dump(exclude_unset=True)

    # Upload avatar to S3
    if dp:
        avatar_content = await dp.read()
        avatar_key = f"avatars/{current_user.id}_{uuid.uuid4()}.{dp.filename.split('.')[-1]}"
        is_upload = storage.upload_file(avatar_content, avatar_key)
        if is_upload:
            avatar_url = storage.get_public_url(avatar_key)
            update_data['dp'] = avatar_url

    # Update fields
    for field, value in update_data.items():
        setattr(current_user, field, value)
    
    db.add(current_user)
    await db.commit()
    await db.refresh(current_user)
    
    return current_user

@router.delete("/me", status_code=status.HTTP_204_NO_CONTENT)
async def delete_user(
    user_id: int,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_user)
):
    """Delete user - only owner"""
    # Verify ownership
    if current_user.id != user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
    result = await db.execute(select(User).where(User.id == user_id))
    user = result.scalar_one_or_none()
    if not user:
        raise HTTPException(status_code=404, detail="User not found")
    
    # Soft delete (deactivate)
    user.is_active = False
    db.add(user)
    await db.commit()
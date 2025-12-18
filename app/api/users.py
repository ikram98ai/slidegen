# app/api/users.py
from typing import Optional, List
from fastapi import (
    APIRouter,
    HTTPException,
    status,
    UploadFile,
    Depends,
    File,
    Form,
    Query,
)
import uuid
import json
from app.services import storage
from app.models import Subject, User
from app.schemas import UserResponse, UserUpdate, SubjectResponse
from app.services.auth import get_current_user, get_current_user_or_anonymous
from app.services.storage import get_presigned_url

router = APIRouter()



@router.get("/me", response_model=UserResponse)
async def get_myprofile(current_user: User = Depends(get_current_user)):
    if current_user.dp:
        current_user.dp = get_presigned_url(current_user.dp)
    return current_user


@router.get("/{user_id}", response_model=UserResponse)
async def get_user(user_id: str):
    """Get user by ID"""
    try:
        user = User.get(user_id)
    except User.DoesNotExist:
        raise HTTPException(status_code=404, detail="User not found")

    # Only show active users
    if not user.is_active:
        raise HTTPException(status_code=404, detail="User not found")

    if user.dp:
        user.dp = get_presigned_url(user.dp)
    return user


@router.get("/{user_id}/subjects", response_model=List[SubjectResponse])
async def get_user_subjects(
    user_id: str,
    skip: int = Query(0, ge=0),
    limit: int = Query(100, ge=1, le=100),
    current_user: User = Depends(get_current_user_or_anonymous)
):
    """Get all subjects for a user"""
    # Check if user exists
    try:
        user = User.get(user_id)
    except User.DoesNotExist:
        raise HTTPException(status_code=404, detail="User not found")
    
    if current_user and current_user.id == user_id:
        subjects = Subject.user_index.query(user.id)
    else:
        subjects = Subject.user_index.query(user.id, filter_condition=Subject.is_public==True)

    subjects = sorted(subjects, key=lambda s: s.created_at, reverse=True)
    return subjects[skip : skip + limit]



@router.patch("/me", response_model=UserResponse)
async def update_user(
    user_update: str = Form(...),
    dp: Optional[UploadFile] = File(None),
    current_user: User = Depends(get_current_user),
):
    """Update user - only owner"""

    user_update_data = UserUpdate.model_validate(json.loads(user_update))

    if dp and not dp.filename.lower().endswith((".jpg", ".jpeg", ".png")):
        raise HTTPException(
            status_code=400,
            detail="Only JPG, JPEG, PNG files are allowed for avatar",
        )

    if dp and dp.size > 5 * 1024 * 1024:  # 5MB limit
        raise HTTPException(status_code=400, detail="Avatar file size too large (max 5MB)")

    update_data = user_update_data.model_dump(exclude_unset=True)

    # Upload avatar to S3
    if dp:
        avatar_content = await dp.read()
        avatar_key = (
            f"avatars/{current_user.id}_{uuid.uuid4()}.{dp.filename.split('.')[-1]}"
        )
        is_upload = storage.upload_file(avatar_content, avatar_key)
        if is_upload:
            update_data["dp"] = avatar_key

    for field, value in update_data.items():
        setattr(current_user, field, value)

    current_user.save()
    if current_user.dp:
        current_user.dp = get_presigned_url(current_user.dp)
    return current_user


@router.delete("/me", status_code=status.HTTP_204_NO_CONTENT)
async def delete_user(current_user: User = Depends(get_current_user)):
    """Delete user - only owner"""

    # Soft delete (deactivate)
    current_user.update(actions=[User.is_active.set(False)])
    current_user.save()
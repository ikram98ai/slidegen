# app/api/users.py
from typing import Optional, List
from fastapi import APIRouter, Depends, HTTPException, status, Query
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select

from app.db import get_db
from app.db import Subject, User
from app.schemas import UserResponse, UserUpdate, SubjectResponse
from app.api.deps import get_current_active_user

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
    current_user: Optional[User] = Depends(get_current_active_user)
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

@router.patch("/{user_id}", response_model=UserResponse)
async def update_user(
    user_id: int,
    user_update: UserUpdate,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_active_user)
):
    """Update user - only owner"""
    # Verify ownership
    if current_user.id != user_id:
        raise HTTPException(status_code=403, detail="Not enough permissions")
    
    result = await db.execute(select(User).where(User.id == user_id))
    user = result.scalar_one_or_none()
    if not user:
        raise HTTPException(status_code=404, detail="User not found")
    
    # Update user
    update_data = user_update.dict(exclude_unset=True)
    # store the dp in s3 and then save the url in db
    # if "dp" in update_data and update_data["dp"]:
    #     # Convert base64 to bytes
    #     try:
    #         if ',' in update_data["dp"]:
    #             update_data["dp"] = base64.b64decode(update_data["dp"].split(',')[-1])
    #         else:
    #             update_data["dp"] = base64.b64decode(update_data["dp"])
    #     except:
    #         raise HTTPException(status_code=400, detail="Invalid base64 image")
    
    # Update fields
    for field, value in update_data.items():
        setattr(user, field, value)
    
    db.add(user)
    await db.commit()
    await db.refresh(user)
    
    return user

@router.delete("/{user_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_user(
    user_id: int,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(get_current_active_user)
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
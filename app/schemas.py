# app/schemas/user.py
from pydantic import BaseModel, EmailStr, field_validator, Field
from typing import Optional, List

from datetime import datetime
from enum import Enum


class UserBase(BaseModel):
    full_name: str
    email: EmailStr


class UserCreate(UserBase):
    password: str

    @field_validator("password")
    def validate_password(cls, v):
        if len(v) < 8:
            raise ValueError("Password must be at least 8 characters")
        return v


class UserUpdate(BaseModel):
    full_name: Optional[str] = None


class UserInDB(UserBase):
    id: str
    is_active: bool

    class Config:
        from_attributes = True


class UserResponse(UserInDB):
    dp: Optional[str] = None


class Token(BaseModel):
    access_token: str
    refresh_token: str
    token_type: str = "bearer"


class TokenData(UserBase):
    id: str


class SubjectType(str, Enum):
    BOOK = "book"
    REPORT = "report"


class SubjectBase(BaseModel):
    title: str
    is_public: bool = False
    type: SubjectType


class SubjectCreate(SubjectBase):
    user_id: str
    file_path: str


class SubjectUpdate(BaseModel):
    title: Optional[str] = None
    is_public: Optional[bool] = None


class SubjectInDB(SubjectBase):
    id: str
    user_id: str
    file_path: str
    processing_status: str
    created_at: datetime
    updated_at: Optional[datetime] = None

    class Config:
        from_attributes = True


class SubjectResponse(SubjectInDB):
    pass


class SubjectDetailResponse(SubjectInDB):
    chapters: Optional[List["ChapterResponse"]]


class ChapterBase(BaseModel):
    title: str = Field(..., min_length=1, max_length=200)
    page_start: int = Field(..., ge=0)
    page_end: int = Field(..., ge=1)
    order_index: int = Field(..., ge=0)


class ChapterCreate(ChapterBase):
    subject_id: str


class ChapterUpdate(BaseModel):
    title: Optional[str] = Field(None, min_length=1, max_length=200)
    page_start: Optional[int] = Field(None, ge=0)
    page_end: Optional[int] = Field(None, ge=1)
    order_index: Optional[int] = Field(None, ge=0)


class ChapterInDB(ChapterBase):
    id: str
    subject_id: str
    created_at: datetime
    updated_at: Optional[datetime] = None

    class Config:
        from_attributes = True


class ChapterResponse(ChapterInDB):
    pass


class ChapterWithSlides(ChapterResponse):
    slides: List["SlideResponse"] = []

    class Config:
        from_attributes = True


class ChapterWithSubject(ChapterResponse):
    subject_title: str
    subject_type: SubjectType
    subject_is_public: bool

    class Config:
        from_attributes = True


class SlideBase(BaseModel):
    title: str = Field(..., min_length=1, max_length=200)
    points: List[str] = Field(..., min_items=1)
    explanation: str = Field(..., min_length=10)
    order_index: int = Field(..., ge=0)


class SlideUpdate(BaseModel):
    title: Optional[str] = Field(None, min_length=1, max_length=200)
    points: Optional[List[str]] = Field(None, min_items=1)
    explanation: Optional[str] = Field(None, min_length=10)
    order_index: Optional[int] = Field(None, ge=0)
    voice_url: Optional[str] = None


class SlideResponse(SlideBase):
    id: str
    chapter_id: str
    voice_url: Optional[str]
    created_at: datetime
    updated_at: Optional[datetime] = None

    class Config:
        from_attributes = True

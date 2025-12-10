# app/schemas/user.py
from pydantic import BaseModel, EmailStr, field_validator, Field
from typing import Optional, List

from datetime import datetime
from enum import Enum
import base64

class UserBase(BaseModel):
    full_name: str
    email: EmailStr

class UserCreate(UserBase):
    password: str
    
    @field_validator('password')
    def validate_password(cls, v):
        if len(v) < 8:
            raise ValueError('Password must be at least 8 characters')
        return v

class UserUpdate(BaseModel):
    full_name: Optional[str] = None
    dp: Optional[str] = None  # base64 encoded image
    
    @field_validator('dp')
    def validate_dp(cls, v):
        if v:
            try:
                # Validate base64
                base64.b64decode(v.split(',')[-1])
            except Exception as e:
                raise ValueError('Invalid base64 image: ' + str(e))
        return v

class UserInDB(UserBase):
    id: int
    is_active: bool
    
    class Config:
        from_attributes = True

class UserResponse(UserInDB):
    pass

class Token(BaseModel):
    access_token: str
    refresh_token: str
    token_type: str = "bearer"

class TokenData(UserBase):
    id: int

class SubjectType(str, Enum):
    BOOK = "book"
    REPORT = "report"

class SubjectBase(BaseModel):
    title: str
    is_public: bool = False
    type: SubjectType

class SubjectCreate(SubjectBase):
    user_id: int
    file_path: str


class SubjectUpdate(BaseModel):
    title: Optional[str] = None
    is_public: Optional[bool] = None

class SubjectInDB(SubjectBase):
    id: int
    user_id: int
    file_path: str
    processing_status: str
    created_at: datetime
    updated_at: Optional[datetime] = None
    
    class Config:
        from_attributes = True

class SubjectResponse(SubjectInDB):
    pass


class LessonBase(BaseModel):
    title: str = Field(..., min_length=1, max_length=200)
    page_start: int = Field(..., ge=1)
    page_end: int = Field(..., ge=1)
    order_index: int = Field(..., ge=0)

class LessonCreate(LessonBase):
    subject_id: int

class LessonUpdate(BaseModel):
    title: Optional[str] = Field(None, min_length=1, max_length=200)
    page_start: Optional[int] = Field(None, ge=1)
    page_end: Optional[int] = Field(None, ge=1)
    order_index: Optional[int] = Field(None, ge=0)

class LessonInDB(LessonBase):
    id: int
    subject_id: int
    created_at: datetime
    updated_at: Optional[datetime] = None
    
    class Config:
        from_attributes = True

class LessonResponse(LessonInDB):
    pass

class LessonWithSlides(LessonResponse):
    slides: List['SlideResponse'] = []
    
    class Config:
        from_attributes = True

class LessonWithSubject(LessonResponse):
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
    
    @field_validator('points')
    def validate_points(cls, v):
        if v is not None and len(v) < 1:
            raise ValueError('Points must contain at least one item')
        return v

class SlideResponse(SlideBase):
    id: int
    lesson_id: int
    voice_url: Optional[str]
    created_at: datetime
    updated_at: Optional[datetime] = None
    
    class Config:
        from_attributes = True



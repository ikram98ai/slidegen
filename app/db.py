# app/models/base.py
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy import Column, Integer, DateTime, func, String, Boolean, Text
from sqlalchemy import ForeignKey, Enum as SQLEnum
from sqlalchemy.orm import relationship
from sqlalchemy.ext.asyncio import AsyncSession, create_async_engine, async_sessionmaker
from app.config import settings
import enum

Base = declarative_base()

engine = create_async_engine(
    settings.DATABASE_URL,
    echo=True,  # Set to False in production
    pool_size=20,
    max_overflow=40,
)

AsyncSessionLocal = async_sessionmaker(
    engine,
    class_=AsyncSession,
    expire_on_commit=False,
)

async def get_db():
    async with AsyncSessionLocal() as session:
        try:
            yield session
        finally:
            await session.close()


class BaseModel(Base):
    __abstract__ = True
    
    id = Column(Integer, primary_key=True, index=True)
    created_at = Column(DateTime(timezone=True), server_default=func.now())
    updated_at = Column(DateTime(timezone=True), onupdate=func.now())


class User(BaseModel):
    __tablename__ = "users" 
    full_name = Column(String, nullable=False)
    email = Column(String, unique=True, index=True, nullable=False)
    hashed_password = Column(String, nullable=False)
    dp = Column(String, nullable=True)  # Profile picture
    is_active = Column(Boolean, default=True)
    
    subjects = relationship("Subject", back_populates="owner", cascade="all, delete-orphan")


class SubjectType(str, enum.Enum):
    BOOK = "book"
    REPORT = "report"

class Subject(BaseModel):
    __tablename__ = "subjects"
    user_id = Column(Integer, ForeignKey("users.id", ondelete="CASCADE"), nullable=False)
    title = Column(String, nullable=False)
    file_path = Column(String, nullable=False)
    is_public = Column(Boolean, default=False)
    type = Column(SQLEnum(SubjectType), nullable=False)
    processing_status = Column(String, default="pending")  # pending, processing, completed, failed
    
    owner = relationship("User", back_populates="subjects")
    chapters = relationship("Chapter", back_populates="subject", cascade="all, delete-orphan")


class Chapter(BaseModel):
    __tablename__ = "chapters"
    subject_id = Column(Integer, ForeignKey("subjects.id", ondelete="CASCADE"), nullable=False)
    title = Column(String, nullable=False)
    page_start = Column(Integer, nullable=False)
    page_end = Column(Integer, nullable=False)
    order_index = Column(Integer, nullable=False)
    
    subject = relationship("Subject", back_populates="chapters")
    slides = relationship("Slide", back_populates="chapter", cascade="all, delete-orphan")


class Slide(BaseModel):
    __tablename__ = "slides"
    chapter_id = Column(Integer, ForeignKey("chapters.id", ondelete="CASCADE"), nullable=False)
    title = Column(String, nullable=False)
    points = Column(Text, nullable=False)
    explanation = Column(Text, nullable=False)
    voice_url = Column(String, nullable=True)
    order_index = Column(Integer, nullable=False)
    
    chapter = relationship("Chapter", back_populates="slides")
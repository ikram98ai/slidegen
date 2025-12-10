# app/core/config.py
from pydantic_settings import BaseSettings
from typing import Optional

class Settings(BaseSettings):

    # AWS
    AWS_ACCESS_KEY_ID: str  
    AWS_SECRET_ACCESS_KEY: str  
    AWS_REGION: str  = "ap-southeast-1"
    S3_BUCKET_NAME: str  

    # Security
    SECRET_KEY: str
    ALGORITHM: str = "HS256"
    ACCESS_TOKEN_EXPIRE_DAYS: int = 7
    REFRESH_TOKEN_EXPIRE_DAYS: int = 30
    
    # Database
    DATABASE_URL: str = "sqlite+aiosqlite:///./lumina.sqlite3"
    
    # AI Services
    GEMINI_API_KEY: Optional[str] 
    TEXT_MODEL: str = "gemini-2.5-flash"
    EMBEDDING_MODEL: str = "text-embedding-3-small"

    
    class Config:
        env_file = ".env"

settings = Settings()
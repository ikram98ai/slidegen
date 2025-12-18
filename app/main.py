# app/main.py
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from mangum import Mangum
from app.api import auth, users, subjects, chapters
from app.config import settings
from logging import getLogger

logger = getLogger(__name__)


app = FastAPI(title="lumina", openapi_url="/openapi.json")

# Set up CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"] if settings.DEBUG else [],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Include routers
app.include_router(auth.router, prefix="/api/auth", tags=["auth"])
app.include_router(users.router, prefix="/api/users", tags=["users"])
app.include_router(subjects.router, prefix="/api/subjects", tags=["subjects"])
app.include_router(chapters.router, prefix="/api/chapters", tags=["chapters"])


@app.get("/")
async def root():
    return {"message": "Lumina Education Platform API", "version": "1.0.0"}


@app.get("/health")
async def health_check():
    return {"status": "healthy"}


handler = Mangum(app)

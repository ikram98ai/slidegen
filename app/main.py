# app/main.py
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from contextlib import asynccontextmanager
from mangum import Mangum
from app.api import auth, users, subjects, chapters, slides
from app.db import engine, Base
from logging import getLogger
from app.config import settings
logger = getLogger(__name__)

@asynccontextmanager
async def lifespan(app: FastAPI):
    # Startup
    logger.info("Starting up...")
    # Create database tables
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)

    yield
    
    # Shutdown
    logger.info("Shutting down...")
    await engine.dispose()

app = FastAPI(
    title="lumina",
    openapi_url="/openapi.json",
    lifespan=lifespan
)

# Set up CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"] if settings.DEBUG else ["https://slides.khaneducation.ai"], 
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Include routers
app.include_router(auth.router, prefix="/api/auth", tags=["auth"])
app.include_router(users.router, prefix="/api/users", tags=["users"])
app.include_router(subjects.router, prefix="/api/subjects", tags=["subjects"])
app.include_router(chapters.router, prefix="/api/chapters", tags=["chapters"])
app.include_router(slides.router, prefix="/api/slides", tags=["slides"])

@app.get("/")
async def root():
    return {"message": "Lumina Education Platform API", "version": "1.0.0"}

@app.get("/health")
async def health_check():
    return {"status": "healthy"}

handler = Mangum(app)
from fastapi import HTTPException, status
from sqlalchemy import select, update
import fitz, uuid
from app.db import Subject, SubjectType, Lesson, Slide
from app.services import ai, storage
import numpy as np 

async def process_subject_background(subject_id: int, file_s3path:str ):
    """Background task to process uploaded subject"""
    from app.db import AsyncSessionLocal
    
    async with AsyncSessionLocal() as db:
        try:
            # Update status to processing
            await db.execute(
                update(Subject)
                .where(Subject.id == subject_id)
                .values(processing_status="processing")
            )
            await db.commit()
            
            # Extract lessons if it's a book
            result = await db.execute(select(Subject).where(Subject.id == subject_id))
            subject = result.scalar_one_or_none()


            # read the pdf file from s3 which can know the table of contents and page count.
            file = storage.download_file(file_s3path)
            doc = fitz.open(file)

            if subject and subject.type == SubjectType.BOOK:

                toc_list = doc.get_toc()    
                toc = "\n".join(toc_list)

                lessons_data = await ai.analyze_book_toc(toc)
                
                # Save lessons
                for i, lesson_data in enumerate(lessons_data):
                    lesson = Lesson(
                        subject_id=subject_id,
                        title=lesson_data.title,
                        page_start=lesson_data.page_start,
                        page_end=lesson_data.page_end,
                        order_index=i
                    )
                    db.add(lesson)
                await db.commit()
            else:
                # For reports, create a single lesson
                lesson = Lesson(
                    subject_id=subject_id,
                    title="Report Content",
                    page_start=1,
                    page_end=doc.page_count(),
                    order_index=0
                )
                db.add(lesson)
                await db.commit()
            
            # Update status to completed
            await db.execute(
                update(Subject)
                .where(Subject.id == subject_id)
                .values(processing_status="completed")
            )
            await db.commit()
            
        except Exception as e:
            # Update status to failed
            await db.execute(
                update(Subject)
                .where(Subject.id == subject_id)
                .values(processing_status="failed")
            )
            await db.commit()
            
            import traceback
            print(f"Error processing subject {subject_id}: {e}")
            traceback.print_exc()



async def generate_slides_background(lesson: Lesson):
    """Background task to generate slides"""
    from app.db import AsyncSessionLocal
    
    async with AsyncSessionLocal() as db:
        try:

            file = storage.download_file(lesson.subject.file_path)
            doc = fitz.open(file)
            pages = list(np.arange(lesson.page_start-1, lesson.page_end-1))
            doc.select(pages)
            chapter_text = "\n".join([page.get_text() for page in doc])
            
            slides_data = await ai.generate_chapter_slides(lesson.title, chapter_text)
            
            if not slides_data:
                return
            
            # Create slides
            for i, slide_data in enumerate(slides_data):
                # Generate voice for explanation
                voice_filename = f"{uuid.uuid4()}_{slide_data.title[:20].replace(' ', '_')}.mp3"
                wav_base64 = await ai.generate_slide_audio(slide_data.explanation)
                
                is_upload = storage.upload_file(wav_base64, voice_filename)
                if not is_upload:
                    raise HTTPException(
                        status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                        detail="Failed to upload audio file to s3."
                    )

                # Create slide
                slide = Slide(
                    lesson_id=lesson.id,
                    title=slide_data.title,
                    points=slide_data.bullets,
                    explanation=slide_data.explanation,
                    voice_url=voice_filename,
                    order_index=i
                )
                db.add(slide)
            
            await db.commit()
            
        except Exception as e:
            import traceback
            print(f"Error generating slides for lesson {lesson.id}: {e}")
            traceback.print_exc()

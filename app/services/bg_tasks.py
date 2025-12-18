import fitz
import uuid
import os
import base64
from app.models import Subject, SubjectType, Chapter, Slide
from app.services import ai, storage


async def process_subject_background(user_id: str, subject_id: str, file_s3path: str):
    """Background task to process uploaded subject"""
    file_path = None
    subject = Subject.get(subject_id)
    print(f"process subject background task started for subject {subject.title}:{subject.id}")
    try:
        # Update status to processing
        subject.update(actions=[Subject.processing_status.set("processing")])

        # read the pdf file from s3 which can know the table of contents and page count.
        file_path = storage.download_file(file_s3path)
        if not file_path:
            raise Exception("Failed to download file from S3")

        doc = fitz.open(file_path)

        if subject and subject.type == SubjectType.BOOK:
            toc_list = doc.get_toc()
            toc = ""
            if toc_list:
                for item in toc_list:
                    level, title, page_num = item[0], item[1], item[2]
                    # Use indentation based on the hierarchy level (lvl - 1)
                    indent = "  " * (level - 1)
                    toc += f"{indent}- {title} (Page {page_num})\\n"

                chapters_data = await ai.analyze_book_toc(toc)

                # Save chapters
                # with Chapter.batch_write() as batch:
                for i, chapter_data in enumerate(chapters_data):
                    chapter = Chapter(
                        id=str(uuid.uuid4()),
                        subject_id=subject_id,
                        title=chapter_data.title,
                        page_start=chapter_data.page_start,
                        page_end=chapter_data.page_end,
                        order_index=i,
                    )
                    chapter.save()
                    # batch.save(chapter)
            else:
                chapter = Chapter(
                    id=str(uuid.uuid4()),
                    user_id=user_id,
                    subject_id=subject_id,
                    title="Only chapter of the book, because there is no table of content",
                    page_start=0,
                    page_end=doc.page_count,
                    order_index=0,
                )
                chapter.save()

        else:
            # For reports, create a single chapter
            chapter = Chapter(
                id=str(uuid.uuid4()),
                user_id=user_id,
                subject_id=subject_id,
                title=subject.title + "'s Report",
                page_start=0,
                page_end=doc.page_count,
                order_index=0,
            )
            chapter.save()

        # Update status to completed
        subject.update(actions=[Subject.processing_status.set("completed")])

    except Exception as e:
        # Update status to failed
        subject.update(actions=[Subject.processing_status.set("failed")])

        import traceback

        print(f"Error processing subject {subject.title}:{subject.id}: {e}")
        traceback.print_exc()
    finally:
        if file_path and os.path.exists(file_path):
            os.remove(file_path)

    print(f"process subject background task completed for subject {subject.title}:{subject.id}.")


async def generate_slides_background(user_id: str, subject_id, chapter: Chapter):
    """Background task to generate slides"""

    print(f"generate slides background task started for chapter: {chapter.title}:{chapter.id}.")
    file_path = None
    try:
        subject = Subject.get(subject_id)
        file_path = storage.download_file(subject.file_path)
        if not file_path:
            raise Exception("Failed to download file from S3")

        doc = fitz.open(file_path)

        page_count = doc.page_count
        start_page = chapter.page_start
        if start_page > page_count:
            start_page = page_count - 1
        end_page = chapter.page_end
        if end_page > page_count:
            end_page = page_count

        for page_no in range(start_page, end_page):
            page_text = doc.get_page_text(page_no)
            slides_data = await ai.generate_slides(page_text)

            if not slides_data:
                return

            # # Create slides
            # with Slide.batch_write() as batch:
            for i, slide_data in enumerate(slides_data):
                # Generate voice for explanation
                voice_url = None
                try:
                    voice_filename = f"voices/{chapter.id}/{uuid.uuid4()}.mp3"
                    wav_base64 = await ai.generate_slide_audio(slide_data.explanation)
                    wav_bytes = base64.b64decode(wav_base64)
                    storage.upload_file(wav_bytes, voice_filename)
                    voice_url = storage.get_public_url(voice_filename)
                except Exception:
                    print(f"There is an error while generating voice for slide {slide_data.title} of chapter {chapter.id}")
                    voice_url  = None
                # Create slide
                slide = Slide(
                    id=str(uuid.uuid4()),
                    user_id=user_id,
                    chapter_id=chapter.id,
                    title=slide_data.title,
                    points=slide_data.bullets,
                    explanation=slide_data.explanation,
                    voice_url=voice_url,
                    order_index=i,
                )
                slide.save()
                    # batch.save(slide)

    except Exception as e:
        import traceback

        print(f"Error generating slides for chapter {chapter.title}:{chapter.id}: {e}")
        traceback.print_exc()
    finally:
        if file_path and os.path.exists(file_path):
            os.remove(file_path)
    print(f"generate slides background task completed for chapter: {chapter.title}:{chapter.id}.")

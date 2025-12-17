from pynamodb.models import Model
from pynamodb.attributes import (
    UnicodeAttribute,
    BooleanAttribute,
    NumberAttribute,
    ListAttribute,
    UTCDateTimeAttribute,
)
from pynamodb.indexes import (
    GlobalSecondaryIndex,
    AllProjection,
)

from app.config import settings
import uuid
from datetime import datetime
from typing import Optional
import enum


class UserEmailIndex(GlobalSecondaryIndex):
    class Meta:
        index_name = "email-index"
        projection = AllProjection()  # More efficient for lookups

    email = UnicodeAttribute(hash_key=True)

class User(Model):
    """
    A DynamoDB User
    """

    class Meta:
        table_name = "lumina_users"
        region = settings.AWS_REGION
        billing_mode = "PAY_PER_REQUEST"

    id = UnicodeAttribute(hash_key=True, default=lambda: str(uuid.uuid4()))
    full_name = UnicodeAttribute()
    email = UnicodeAttribute(null=False)
    hashed_password = UnicodeAttribute(null=False)
    dp = UnicodeAttribute(null=True)
    is_active = BooleanAttribute(default=True)
    created_at = UTCDateTimeAttribute(default=datetime.now)
    updated_at = UTCDateTimeAttribute(default=datetime.now)

    email_index = UserEmailIndex()

    def save(self, *args, **kwargs):
        self.updated_at = datetime.now()
        super().save(*args, **kwargs)

    @classmethod
    def get_by_email(cls, email: str) -> Optional["User"]:
        try:
            return next(cls.email_index.query(email))
        except StopIteration:
            return None


class SubjectType(str, enum.Enum):
    BOOK = "book"
    REPORT = "report"

class SubjectUserIndex(GlobalSecondaryIndex):
    class Meta:
        index_name = "user-index"
        projection = AllProjection()  # More efficient for lookups

    user_id = UnicodeAttribute(hash_key=True)

class Subject(Model):
    """
    A DynamoDB Subject
    """
    class Meta:
        table_name = "lumina_subjects"
        region = settings.AWS_REGION
        billing_mode = "PAY_PER_REQUEST"

    id = UnicodeAttribute(hash_key=True, default=lambda: str(uuid.uuid4()))
    user_id = UnicodeAttribute()
    title = UnicodeAttribute(null=False)
    file_path = UnicodeAttribute(null=False)
    is_public = BooleanAttribute(default=False)
    type = UnicodeAttribute(null=False)
    processing_status = UnicodeAttribute(default="pending")
    created_at = UTCDateTimeAttribute(default=datetime.now)
    updated_at = UTCDateTimeAttribute(default=datetime.now)

    user_index = SubjectUserIndex()

    def save(self, *args, **kwargs):
        self.updated_at = datetime.now()
        super().save(*args, **kwargs)
    

class Chapter(Model):
    """
    A DynamoDB Chapter
    """

    class Meta:
        table_name = "lumina_chapters"
        region = settings.AWS_REGION
        billing_mode = "PAY_PER_REQUEST"

    subject_id = UnicodeAttribute(hash_key=True)
    id = UnicodeAttribute(range_key=True, default=lambda: str(uuid.uuid4()))
    user_id = UnicodeAttribute()

    title = UnicodeAttribute(null=False)
    page_start = NumberAttribute(null=False)
    page_end = NumberAttribute(null=False)
    order_index = NumberAttribute(null=False)
    created_at = UTCDateTimeAttribute(default=datetime.now)
    updated_at = UTCDateTimeAttribute(default=datetime.now)

    def save(self, *args, **kwargs):
        self.updated_at = datetime.now()
        super().save(*args, **kwargs)


class Slide(Model):
    """
    A DynamoDB Slide
    """

    class Meta:
        table_name = "lumina_slides"
        region = settings.AWS_REGION
        billing_mode = "PAY_PER_REQUEST"

    chapter_id = UnicodeAttribute(hash_key=True)
    id = UnicodeAttribute(range_key=True, default=lambda: str(uuid.uuid4()))
    user_id = UnicodeAttribute()
    title = UnicodeAttribute(null=False)
    points = ListAttribute(of=UnicodeAttribute)
    explanation = UnicodeAttribute(null=False)
    voice_url = UnicodeAttribute(null=True)
    order_index = NumberAttribute(null=False)
    created_at = UTCDateTimeAttribute(default=datetime.now)
    updated_at = UTCDateTimeAttribute(default=datetime.now)

    def save(self, *args, **kwargs):
        self.updated_at = datetime.now()
        super().save(*args, **kwargs)



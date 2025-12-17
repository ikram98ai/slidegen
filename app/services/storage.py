import boto3
from botocore.exceptions import ClientError
from app.config import settings
import logging
import json

logger = logging.getLogger(__name__)

s3_client = boto3.client("s3")

def upload_file(file, object_name):
    """Upload a file to an S3 bucket"""

    try:
        # bytes/bytearray -> wrap in BytesIO
        if isinstance(file, (bytes, bytearray)):
            import io

            file = io.BytesIO(file)
            s3_client.upload_fileobj(file, settings.S3_BUCKET_NAME, object_name)
            return True

        # path string -> use upload_file (more efficient for large files)
        if isinstance(file, str):
            s3_client.upload_file(file, settings.S3_BUCKET_NAME, object_name)
            return True

        # file-like object
        s3_client.upload_fileobj(file, settings.S3_BUCKET_NAME, object_name)
        return True
    except ClientError as e:
        logger.error(e)
        return False


def download_file(object_name):
    """Download a file from an S3 bucket"""
    file_path = object_name.split("/")[-1]
    try:
        with open(file_path, "wb") as f:
            s3_client.download_fileobj(settings.S3_BUCKET_NAME, object_name, f)
    except ClientError as e:
        logger.error(e)
        return None
    return file_path


def get_presigned_url(object_name, expiration=3600):
    """Generate a presigned URL to share an S3 object"""
    try:
        response = s3_client.generate_presigned_url(
            "get_object",
            Params={"Bucket": settings.S3_BUCKET_NAME, "Key": object_name},
            ExpiresIn=expiration,
        )
    except ClientError as e:
        logger.error(e)
        return None
    return response


def get_public_url(object_name):
    """Generate a public URL for an S3 object."""
    return f"https://{settings.S3_BUCKET_NAME}.s3.{settings.AWS_REGION}.amazonaws.com/{object_name}"


def upload_text(content, object_name):
    """Upload text content to S3"""
    try:
        s3_client.put_object(
            Body=content, Bucket=settings.S3_BUCKET_NAME, Key=object_name
        )
    except ClientError as e:
        logger.error(e)
        return False
    return True


def get_text_from_s3(object_name):
    """Download text content from S3"""
    try:
        response = s3_client.get_object(Bucket=settings.S3_BUCKET_NAME, Key=object_name)
        return response["Body"].read().decode("utf-8")
    except ClientError as e:
        logger.error(e)
        return None


def delete_files(object_names: list[str]) -> bool:
    """Delete multiple files from the S3 bucket."""
    if not object_names:
        return True

    # Filter out any None or empty string keys
    keys_to_delete = [name for name in object_names if name]
    if not keys_to_delete:
        return True

    delete_payload = {"Objects": [{"Key": name} for name in keys_to_delete]}
    try:
        s3_client.delete_objects(Bucket=settings.S3_BUCKET_NAME, Delete=delete_payload)
        logger.info("Deleted %s objects from S3", len(keys_to_delete))
        return True
    except ClientError as e:
        logger.error("Failed to delete objects from S3: %s", e)
        return False

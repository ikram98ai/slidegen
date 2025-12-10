import boto3
from botocore.exceptions import ClientError
from app.config import settings
import logging

logger = logging.getLogger(__name__)

s3_client = boto3.client(
    "s3",
    aws_access_key_id=settings.AWS_ACCESS_KEY_ID,
    aws_secret_access_key=settings.AWS_SECRET_ACCESS_KEY,
    region_name=settings.AWS_REGION,
)
def ensure_bucket_exists() -> bool:
    """Ensure the S3 bucket exists; create it if missing."""
    bucket = settings.S3_BUCKET_NAME
    try:
        s3_client.head_bucket(Bucket=bucket)
        return True
    except ClientError:
        logger.info("Bucket %s not found, attempting to create it", bucket)
        try:
            if settings.AWS_REGION in (None, "", "us-east-1"):
                s3_client.create_bucket(Bucket=bucket)
            else:
                s3_client.create_bucket(
                    Bucket=bucket,
                    CreateBucketConfiguration={"LocationConstraint": settings.AWS_REGION},
                )
            logger.info("Created bucket %s", bucket)
            return True
        except ClientError as ce:
            logger.error("Failed to create bucket %s: %s", bucket, ce)
            return False


def upload_file(file_obj, object_name):
    """Upload a file to an S3 bucket"""
    if not ensure_bucket_exists():
        logger.error("Bucket does not exist and could not be created.")
        return False

    try:
        s3_client.upload_fileobj(file_obj, settings.S3_BUCKET_NAME, object_name)
    except ClientError as e:
        logger.error(e)
        return False
    return True

def download_file(object_name, file_obj):
    """Download a file from an S3 bucket"""
    try:
        file = s3_client.download_fileobj(settings.S3_BUCKET_NAME, object_name, file_obj)
    except ClientError as e:
        logger.error(e)
        return None
    return file


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
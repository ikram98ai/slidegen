import typer
from app.models import User, Subject, Chapter, Slide
import json
import boto3
from botocore.exceptions import ClientError
from app.config import settings


s3_client = boto3.client("s3")

app = typer.Typer()
db_app = typer.Typer()
s3_app = typer.Typer()

app.add_typer(db_app, name="db")
app.add_typer(s3_app, name="s3")


@db_app.command("create-tables")
def create_tables():
    """
    Create all DynamoDB tables.
    """
    for model in [User, Subject, Chapter, Slide]:
        if not model.exists():
            print(f"Creating table for model {model.Meta.table_name}...")
            model.create_table(read_capacity_units=1, write_capacity_units=1, wait=True)
            print(f"Table for model {model.Meta.table_name} created.")
        else:
            print(f"Table for model {model.Meta.table_name} already exists.")

@db_app.command("delete-tables")
def delete_tables():
    """
    Delete all DynamoDB tables.
    """
    for model in [User, Subject, Chapter, Slide]:
        if model.exists():
            print(f"Deleting table for model {model.Meta.table_name}...")
            model.delete_table(wait=True)
            print(f"Table for model {model.Meta.table_name} deleted.")
        else:
            print(f"Table for model {model.Meta.table_name} does not exist.")


@s3_app.command("create-bucket")
def create_bucket():
    """
    Create a public S3 bucket.
    """
    try:
        # Create the bucket
        if settings.AWS_REGION in (None, "", "us-east-1"):
            s3_client.create_bucket(Bucket=settings.S3_BUCKET_NAME)
        else:
            s3_client.create_bucket(
                Bucket=settings.S3_BUCKET_NAME,
                CreateBucketConfiguration={"LocationConstraint": settings.AWS_REGION},
            )
        print("Successfully created bucket", settings.S3_BUCKET_NAME)

        # Disable block public access
        s3_client.put_public_access_block(
            Bucket=settings.S3_BUCKET_NAME,
            PublicAccessBlockConfiguration={
                'BlockPublicAcls': False,
                'IgnorePublicAcls': False,
                'BlockPublicPolicy': False,
                'RestrictPublicBuckets': False
            }
        )
        print("Disabled public access block for bucket ", settings.S3_BUCKET_NAME)

        # Define the bucket policy
        bucket_policy = {
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Sid": "PublicReadGetObject",
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:GetObject",
                    "Resource": f"arn:aws:s3:::{settings.S3_BUCKET_NAME}/*"
                }
            ]
        }

        # Apply the bucket policy
        s3_client.put_bucket_policy(
            Bucket=settings.S3_BUCKET_NAME,
            Policy=json.dumps(bucket_policy)
        )
        print("Set public read policy for bucket ", settings.S3_BUCKET_NAME)

    except ClientError as ce:
        print(f"Failed to create or configure bucket {settings.S3_BUCKET_NAME}: {ce}")


@s3_app.command("delete-bucket")
def delete_bucket():
    """
    Delete a public S3 bucket. This will delete all objects and versions in the bucket.
    """
    try:
        # Before deleting the bucket, you must delete all objects and versions of objects
        print(f"Getting all objects and versions from bucket {settings.S3_BUCKET_NAME}...")
        paginator = s3_client.get_paginator('list_object_versions')
        page_iterator = paginator.paginate(Bucket=settings.S3_BUCKET_NAME)

        objects_to_delete = []
        for page in page_iterator:
            if 'Versions' in page:
                for obj in page['Versions']:
                    objects_to_delete.append({'Key': obj['Key'], 'VersionId': obj['VersionId']})
            if 'DeleteMarkers' in page:
                for marker in page['DeleteMarkers']:
                    objects_to_delete.append({'Key': marker['Key'], 'VersionId': marker['VersionId']})
        
        if objects_to_delete:
            print(f"Deleting {len(objects_to_delete)} objects from bucket {settings.S3_BUCKET_NAME}...")
            # Boto3's delete_objects can handle up to 1000 objects at a time
            for i in range(0, len(objects_to_delete), 1000):
                s3_client.delete_objects(
                    Bucket=settings.S3_BUCKET_NAME,
                    Delete={'Objects': objects_to_delete[i:i+1000]}
                )
        else:
            print("Bucket is already empty.")

        print(f"Deleting bucket {settings.S3_BUCKET_NAME}...")
        s3_client.delete_bucket(Bucket=settings.S3_BUCKET_NAME)
        print(f"Successfully deleted bucket {settings.S3_BUCKET_NAME}")
    except ClientError as ce:
        if ce.response['Error']['Code'] == 'NoSuchBucket':
            print(f"Bucket {settings.S3_BUCKET_NAME} does not exist.")
        else:
            print(f"Failed to delete bucket {settings.S3_BUCKET_NAME}: {ce}")


if __name__ == "__main__":
    app()


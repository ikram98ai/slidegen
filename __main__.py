import pulumi
import pulumi_aws as aws
import pulumi_docker as docker
import pulumi_command as command
import pulumi_synced_folder as synced_folder
import json, os, time
from dotenv import load_dotenv, find_dotenv

load_dotenv(find_dotenv())

DEBUG = os.getenv("DEBUG")
SECRET_KEY = os.getenv("SECRET_KEY")
ALGORITHM = os.getenv("ALGORITHM")
ACCESS_TOKEN_EXPIRE_DAYS = os.getenv("ACCESS_TOKEN_EXPIRE_DAYS")
REFRESH_TOKEN_EXPIRE_DAYS = os.getenv("REFRESH_TOKEN_EXPIRE_DAYS")
S3_BUCKET_NAME = os.getenv("S3_BUCKET_NAME")

GEMINI_API_KEY = os.getenv("GEMINI_API_KEY")
TEXT_MODEL = os.getenv("TEXT_MODEL")
EMBEDDING_MODEL = os.getenv("EMBEDDING_MODEL")

# ==================================================================================
# 1. ECR + LAMBDA FUNCTION 
# ==================================================================================

repo = aws.ecr.Repository("lumina-repo", force_delete=True)

# Build and Push image using Pulumi Docker provider
image = docker.Image(
    "lambda-image",
    image_name=repo.repository_url,
    build=docker.DockerBuildArgs(
        context="./",  # Must be root to access uv.lock & pyproject.toml
        dockerfile="./Dockerfile",  # Ensure Dockerfile is at the root
        platform="linux/amd64",
        args={"DOCKER_BUILDKIT": "1"},
    ),
    registry=docker.RegistryArgs(
        server=repo.repository_url,
        username=aws.ecr.get_authorization_token().user_name,
        password=aws.ecr.get_authorization_token().password,
    ),
)

lambda_role = aws.iam.Role(
    "lambda-role",
    assume_role_policy=json.dumps(
        {
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Action": "sts:AssumeRole",
                    "Principal": {"Service": "lambda.amazonaws.com"},
                    "Effect": "Allow",
                }
            ],
        }
    ),
)

# Attach VPC Access Policy so Lambda can reach RDS
aws.iam.RolePolicyAttachment(
    "lambda-vpc-access",
    role=lambda_role.name,
    policy_arn="arn:aws:iam::aws:policy/service-role/AWSLambdaVPCAccessExecutionRole",
)

# Custom policy to access S3 bucket
aws.iam.RolePolicy(
    "lambda-s3-policy",
    role=lambda_role.id,
    policy=lumina_files_bucket.arn.apply(
        lambda arn: json.dumps(
            {
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Effect": "Allow",
                        "Action": ["s3:PutObject", "s3:GetObject"],
                        "Resource": f"{arn}/*",
                    }
                ],
            }
        )
    ),
)

fn = aws.lambda_.Function(
    "lumina-lambda",
    package_type="Image",
    image_uri=image.repo_digest,
    role=lambda_role.arn,
    timeout=60,
    memory_size=1024,
    reserved_concurrent_executions=1,
    environment=aws.lambda_.FunctionEnvironmentArgs(
        variables={
            "S3_BUCKET_NAME": S3_BUCKET_NAME,
            "DEBUG": False,
            "SECRET_KEY": SECRET_KEY,
            "ALGORITHM": ALGORITHM,
            "ACCESS_TOKEN_EXPIRE_DAYS": ACCESS_TOKEN_EXPIRE_DAYS,
            "REFRESH_TOKEN_EXPIRE_DAYS": REFRESH_TOKEN_EXPIRE_DAYS,
            "GEMINI_API_KEY": GEMINI_API_KEY,
            "TEXT_MODEL": TEXT_MODEL,
            "EMBEDDING_MODEL": EMBEDDING_MODEL,
        }
    ),
)

# Create a publicly accessible Function URL.
# We use auth_type="NONE" because CloudFront will front this connection.
# Note: In a stricter environment, we might use IAM auth and sign requests with CloudFront.
func_url = aws.lambda_.FunctionUrl(
    "lumina-lambda-url",
    function_name=fn.name,
    authorization_type="NONE",
    cors=aws.lambda_.FunctionUrlCorsArgs(
        allow_origins=["https://slides.khaneducation.ai"],
        allow_methods=["*"],
        allow_headers=["*"],
        max_age=86400,
    ),
)

# ==================================================================================
# 2. (S3 + CloudFront + Route53)
# ==================================================================================
# S3 Bucket for React App
web_bucket = aws.s3.Bucket(
    "lumina-slides-web",
    bucket="slides.khaneducation.ai-bucket",  # Suffix added to ensure uniqueness
    force_destroy=True,
)

# Build command that runs on every time (using triggers)
frontend_build = command.local.Command(
    "frontend-build",
    create="cd web && npm install && npm run build",
    # Add triggers to force rebuild on every deployment
    triggers=[
        str(
            time.time()
        )  # Also triggers on every run (you can remove this if you only want source-based triggers)
    ],
    environment={"VITE_API_URL": func_url.function_url},
)

synced_web_folder = synced_folder.S3BucketFolder(
    "web-folder-sync",
    acl="private",
    bucket_name=web_bucket.bucket,
    path="./web/dist",
    managed_objects=True,
    opts=pulumi.ResourceOptions(depends_on=[frontend_build]),
)

# ACM Certificate for Custom Domain (MUST be in us-east-1)
cert = aws.acm.Certificate(
    "cert",
    domain_name="slides.khaneducation.ai",
    validation_method="DNS",
    tags={"Environment": "Production"},
)

# Route53 Validation Record
zone = aws.route53.get_zone(name="khaneducation.ai.")

# Create the DNS records required by ACM to prove ownership
cert_validation_dns = aws.route53.Record(
    "cert-validation-dns",
    zone_id=zone.id,
    name=cert.domain_validation_options[0].resource_record_name,
    type=cert.domain_validation_options[0].resource_record_type,
    records=[cert.domain_validation_options[0].resource_record_value],
    ttl=60,
)

# Wait for Certificate Validation
# This resource halts Pulumi until AWS confirms the cert is valid
cert_validation = aws.acm.CertificateValidation(
    "cert-validation",
    certificate_arn=cert.arn,
    validation_record_fqdns=[cert_validation_dns.fqdn],
)

# CloudFront Distribution (Updated)
oac = aws.cloudfront.OriginAccessControl(
    "web-oac",
    description="OAC for Khan Slides",
    origin_access_control_origin_type="s3",
    signing_behavior="always",
    signing_protocol="sigv4",
)

distribution = aws.cloudfront.Distribution(
    "web-distribution",
    enabled=True,
    is_ipv6_enabled=True,
    default_root_object="index.html",
    custom_error_responses=[
        aws.cloudfront.DistributionCustomErrorResponseArgs(
            error_code=404,
            response_code=200,
            response_page_path="/index.html",
        ),
        aws.cloudfront.DistributionCustomErrorResponseArgs(
            error_code=403,
            response_code=200,
            response_page_path="/index.html",
        ),
    ],
    origins=[
        aws.cloudfront.DistributionOriginArgs(
            domain_name=web_bucket.bucket_regional_domain_name,
            origin_id=web_bucket.id,
            origin_access_control_id=oac.id,
        )
    ],
    default_cache_behavior=aws.cloudfront.DistributionDefaultCacheBehaviorArgs(
        allowed_methods=["GET", "HEAD"],
        cached_methods=["GET", "HEAD"],
        target_origin_id=web_bucket.id,
        viewer_protocol_policy="redirect-to-https",
        forwarded_values=aws.cloudfront.DistributionDefaultCacheBehaviorForwardedValuesArgs(
            query_string=False,
            cookies=aws.cloudfront.DistributionDefaultCacheBehaviorForwardedValuesCookiesArgs(
                forward="none"
            ),
        ),
    ),
    aliases=["slides.khaneducation.ai"],
    viewer_certificate=aws.cloudfront.DistributionViewerCertificateArgs(
        acm_certificate_arn=cert_validation.certificate_arn,  # Use the VALIDATED cert ARN
        ssl_support_method="sni-only",
        minimum_protocol_version="TLSv1.2_2021",
    ),
    restrictions=aws.cloudfront.DistributionRestrictionsArgs(
        geo_restriction=aws.cloudfront.DistributionRestrictionsGeoRestrictionArgs(
            restriction_type="none"
        )
    ),
)

# Final DNS Record for the Alias
dns_record = aws.route53.Record(
    "slides-dns",
    zone_id=zone.id,
    name="slides.khaneducation.ai",
    type="A",
    aliases=[
        aws.route53.RecordAliasArgs(
            name=distribution.domain_name,
            zone_id=distribution.hosted_zone_id,
            evaluate_target_health=True,
        )
    ],
)

# Bucket Policy to allow CloudFront to read
bucket_policy = aws.s3.BucketPolicy(
    "web-bucket-policy",
    bucket=web_bucket.id,
    policy=pulumi.Output.all(web_bucket.arn, distribution.arn).apply(
        lambda args: json.dumps(
            {
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Sid": "AllowCloudFrontServicePrincipal",
                        "Effect": "Allow",
                        "Principal": {"Service": "cloudfront.amazonaws.com"},
                        "Action": "s3:GetObject",
                        "Resource": f"{args[0]}/*",
                        "Condition": {"StringEquals": {"AWS:SourceArn": args[1]}},
                    }
                ],
            }
        )
    ),
)


# ==================================================================================
# EXPORTS
# ==================================================================================
pulumi.export("lambda_url", func_url.function_url)
pulumi.export("website_url", pulumi.Output.concat("https://", dns_record.name))

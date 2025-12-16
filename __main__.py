import pulumi
import pulumi_aws as aws
import pulumi_docker as docker
import pulumi_command as command
import pulumi_synced_folder as synced_folder
import json, os
from dotenv import load_dotenv, find_dotenv

load_dotenv(find_dotenv())

DB_USERNAME = os.getenv("DB_USERNAME")
DB_PASSWORD = os.getenv("DB_PASSWORD")

DEBUG = os.getenv("DEBUG")
SECRET_KEY = os.getenv("SECRET_KEY")
ALGORITHM = os.getenv("ALGORITHM")
ACCESS_TOKEN_EXPIRE_DAYS = os.getenv("ACCESS_TOKEN_EXPIRE_DAYS")
REFRESH_TOKEN_EXPIRE_DAYS = os.getenv("REFRESH_TOKEN_EXPIRE_DAYS")

GEMINI_API_KEY = os.getenv("GEMINI_API_KEY")
TEXT_MODEL = os.getenv("TEXT_MODEL")
EMBEDDING_MODEL = os.getenv("EMBEDDING_MODEL")

# ==================================================================================
# 1. NETWORK SETUP (VPC)
# ==================================================================================
# Creating a dedicated VPC for Database and Lambda to ensure secure networking.
vpc = aws.ec2.Vpc("lumina-vpc",
    cidr_block="10.0.0.0/16",
    enable_dns_hostnames=True,
    enable_dns_support=True,
    tags={"Name": "lumina-vpc"}
)

# Public Subnets (for NAT Gateway)
public_subnet_a = aws.ec2.Subnet("public-subnet-a",
    vpc_id=vpc.id,
    cidr_block="10.0.10.0/24",
    availability_zone="us-east-1a",
    map_public_ip_on_launch=True,
    tags={"Name": "lumina-public-subnet-a"}
)

public_subnet_b = aws.ec2.Subnet("public-subnet-b",
    vpc_id=vpc.id,
    cidr_block="10.0.11.0/24",
    availability_zone="us-east-1b",
    map_public_ip_on_launch=True,
    tags={"Name": "lumina-public-subnet-b"}
)

# Private Subnets (for Lambda & RDS)
subnet_a = aws.ec2.Subnet("subnet-a",
    vpc_id=vpc.id,
    cidr_block="10.0.1.0/24",
    availability_zone="us-east-1a",
    tags={"Name": "lumina-subnet-a"}
)

subnet_b = aws.ec2.Subnet("subnet-b",
    vpc_id=vpc.id,
    cidr_block="10.0.2.0/24",
    availability_zone="us-east-1b",
    tags={"Name": "lumina-subnet-b"}
)

# Internet Gateway
igw = aws.ec2.InternetGateway("igw",
    vpc_id=vpc.id,
    tags={"Name": "lumina-igw"}
)

# Elastic IP for NAT Gateway
eip = aws.ec2.Eip("nat-eip",
    domain="vpc",
    tags={"Name": "lumina-nat-eip"}
)

# NAT Gateway (for Lambda to access external APIs like Gemini)
nat = aws.ec2.NatGateway("nat",
    subnet_id=public_subnet_a.id,
    allocation_id=eip.id,
    tags={"Name": "lumina-nat"}
)

# Public Route Table
public_rt = aws.ec2.RouteTable("public-rt",
    vpc_id=vpc.id,
    routes=[aws.ec2.RouteTableRouteArgs(
        cidr_block="0.0.0.0/0",
        gateway_id=igw.id
    )],
    tags={"Name": "lumina-public-rt"}
)

aws.ec2.RouteTableAssociation("public-rta-a",
    subnet_id=public_subnet_a.id,
    route_table_id=public_rt.id
)

aws.ec2.RouteTableAssociation("public-rta-b",
    subnet_id=public_subnet_b.id,
    route_table_id=public_rt.id
)

# Private Route Table (routes through NAT)
private_rt = aws.ec2.RouteTable("private-rt",
    vpc_id=vpc.id,
    routes=[aws.ec2.RouteTableRouteArgs(
        cidr_block="0.0.0.0/0",
        nat_gateway_id=nat.id
    )],
    tags={"Name": "lumina-private-rt"}
)

aws.ec2.RouteTableAssociation("private-rta-a",
    subnet_id=subnet_a.id,
    route_table_id=private_rt.id
)

aws.ec2.RouteTableAssociation("private-rta-b",
    subnet_id=subnet_b.id,
    route_table_id=private_rt.id
)

db_subnet_group = aws.rds.SubnetGroup("db-subnet-group",
    subnet_ids=[subnet_a.id, subnet_b.id],
    tags={"Name": "lumina-db-subnet-group"}
)

# Security Groups
# Lambda SG
lambda_sg = aws.ec2.SecurityGroup("lambda-sg",
    vpc_id=vpc.id,
    description="Allow outbound traffic for Lambda",
    egress=[aws.ec2.SecurityGroupEgressArgs(
        protocol="-1", from_port=0, to_port=0, cidr_blocks=["0.0.0.0/0"]
    )]
)

# Aurora SG (Allow traffic from Lambda on 5432)
db_sg = aws.ec2.SecurityGroup("db-sg",
    vpc_id=vpc.id,
    description="Allow PostgreSQL access from Lambda",
    ingress=[aws.ec2.SecurityGroupIngressArgs(
        protocol="tcp", 
        from_port=5432, 
        to_port=5432, 
        security_groups=[lambda_sg.id]
    )]
)

# ==================================================================================
# 2. AURORA SERVERLESS V2 (PostgreSQL)
# ==================================================================================
# Serverless V2 requires "provisioned" engine mode + scaling config
aurora = aws.rds.Cluster("lumina-aurora-cluster",
    engine="aurora-postgresql",
    engine_mode="provisioned", 
    engine_version="16.4",
    database_name="lumina_db",
    master_username=DB_USERNAME,
    master_password=DB_PASSWORD, # In prod, use Pulumi Config or Secrets Manager
    db_subnet_group_name=db_subnet_group.name,
    vpc_security_group_ids=[db_sg.id],
    skip_final_snapshot=True,
    serverlessv2_scaling_configuration=aws.rds.ClusterServerlessv2ScalingConfigurationArgs(
        min_capacity=0.5,
        max_capacity=1.0,
    )
)

aurora_instance = aws.rds.ClusterInstance("lumina-aurora-instance",
    cluster_identifier=aurora.id,
    instance_class="db.serverless",
    engine=aurora.engine,
    engine_version=aurora.engine_version
)

# ==================================================================================
# 3. STORAGE (S3 - lumina-files)
# ==================================================================================
lumina_files_bucket = aws.s3.Bucket("lumina-files",
    acl="private",
    versioning=aws.s3.BucketVersioningArgs(enabled=True)
)

# ==================================================================================
# 4. ECR & CONTAINER IMAGE
# ==================================================================================
repo = aws.ecr.Repository("lumina-repo", force_delete=True)

# Build and Push image using Pulumi Docker provider
image = docker.Image("lambda-image",
    image_name=repo.repository_url,
    build=docker.DockerBuildArgs(
        context="./",            # Must be root to access uv.lock & pyproject.toml
        dockerfile="./Dockerfile", # Ensure Dockerfile is at the root
        platform="linux/amd64",
        args={"DOCKER_BUILDKIT": "1"} 
    ),
    registry=docker.RegistryArgs(
        server=repo.repository_url,
        username=aws.ecr.get_authorization_token().user_name,
        password=aws.ecr.get_authorization_token().password,
    )
)

# ==================================================================================
# 5. LAMBDA FUNCTION
# ==================================================================================
lambda_role = aws.iam.Role("lambda-role",
    assume_role_policy=json.dumps({
        "Version": "2012-10-17",
        "Statement": [{
            "Action": "sts:AssumeRole",
            "Principal": {"Service": "lambda.amazonaws.com"},
            "Effect": "Allow"
        }]
    })
)

# Attach VPC Access Policy so Lambda can reach RDS
aws.iam.RolePolicyAttachment("lambda-vpc-access",
    role=lambda_role.name,
    policy_arn="arn:aws:iam::aws:policy/service-role/AWSLambdaVPCAccessExecutionRole"
)

# Custom policy to access S3 bucket
aws.iam.RolePolicy("lambda-s3-policy",
    role=lambda_role.id,
    policy=lumina_files_bucket.arn.apply(lambda arn: json.dumps({
        "Version": "2012-10-17",
        "Statement": [{
            "Effect": "Allow",
            "Action": ["s3:PutObject", "s3:GetObject"],
            "Resource": f"{arn}/*"
        }]
    }))
)

db_url = pulumi.Output.all(
    aurora.master_username, 
    aurora.master_password, 
    aurora.endpoint, 
    aurora.port, 
    aurora.database_name
).apply(lambda args: f"postgresql+asyncpg://{args[0]}:{args[1]}@{args[2]}:{args[3]}/{args[4]}")

fn = aws.lambda_.Function("lumina-lambda",
    package_type="Image",
    image_uri=image.repo_digest,
    role=lambda_role.arn,
    timeout=60,
    memory_size=1024, 
    reserved_concurrent_executions=1,
    vpc_config=aws.lambda_.FunctionVpcConfigArgs(
        security_group_ids=[lambda_sg.id],
        subnet_ids=[subnet_a.id, subnet_b.id]
    ),
    environment=aws.lambda_.FunctionEnvironmentArgs(
        variables={
            "S3_BUCKET_NAME": lumina_files_bucket.bucket,
            "DATABASE_URL": db_url,
            
            "DEBUG": False,
            "SECRET_KEY": SECRET_KEY,
            "ALGORITHM": ALGORITHM,
            "ACCESS_TOKEN_EXPIRE_DAYS": ACCESS_TOKEN_EXPIRE_DAYS,
            "REFRESH_TOKEN_EXPIRE_DAYS": REFRESH_TOKEN_EXPIRE_DAYS,

            "GEMINI_API_KEY": GEMINI_API_KEY,
            "TEXT_MODEL": TEXT_MODEL,
            "EMBEDDING_MODEL": EMBEDDING_MODEL,
          
        }
    )
)

# ==================================================================================
# 6. LAMBDA FUNCTION URL
# ==================================================================================
# Create a publicly accessible Function URL. 
# We use auth_type="NONE" because CloudFront will front this connection.
# Note: In a stricter environment, we might use IAM auth and sign requests with CloudFront.
func_url = aws.lambda_.FunctionUrl("lumina-lambda-url",
    function_name=fn.name,
    authorization_type="NONE",
    cors=aws.lambda_.FunctionUrlCorsArgs(
        allow_origins=["https://slides.khaneducation.ai"],
        allow_methods=["*"],
        allow_headers=["*"],
        max_age=86400
    )
)

# ==================================================================================
# 7. (S3 + CloudFront + Route53)
# ==================================================================================
# S3 Bucket for React App
web_bucket = aws.s3.Bucket("lumina-slides-web",
    bucket="slides.khaneducation.ai-bucket", # Suffix added to ensure uniqueness
    force_destroy=True
)

frontend_build = command.local.Command("frontend-build",
    create="cd web && npm run build",
    environment={
        "VITE_API_URL": pulumi.Output.concat(func_url.function_url,"api")
    }
)

synced_web_folder= synced_folder.S3BucketFolder("web-folder-sync",
    acl="private", 
    bucket_name=web_bucket.bucket,
    path="./web/dist",
    managed_objects=True,
    opts=pulumi.ResourceOptions(depends_on=[frontend_build])
)

# ACM Certificate for Custom Domain (MUST be in us-east-1)
cert = aws.acm.Certificate("cert",
    domain_name="slides.khaneducation.ai",
    validation_method="DNS",
    tags={"Environment": "Production"}
)

# Route53 Validation Record
zone = aws.route53.get_zone(name="khaneducation.ai.")

# Create the DNS records required by ACM to prove ownership
cert_validation_dns = aws.route53.Record("cert-validation-dns",
    zone_id=zone.id,
    name=cert.domain_validation_options[0].resource_record_name,
    type=cert.domain_validation_options[0].resource_record_type,
    records=[cert.domain_validation_options[0].resource_record_value],
    ttl=60
)

# Wait for Certificate Validation
# This resource halts Pulumi until AWS confirms the cert is valid
cert_validation = aws.acm.CertificateValidation("cert-validation",
    certificate_arn=cert.arn,
    validation_record_fqdns=[cert_validation_dns.fqdn]
)

# CloudFront Distribution (Updated)
oac = aws.cloudfront.OriginAccessControl("web-oac",
    description="OAC for Khan Slides",
    origin_access_control_origin_type="s3",
    signing_behavior="always",
    signing_protocol="sigv4"
)

distribution = aws.cloudfront.Distribution("web-distribution",
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
        )
    ],
    origins=[aws.cloudfront.DistributionOriginArgs(
        domain_name=web_bucket.bucket_regional_domain_name,
        origin_id=web_bucket.id,
        origin_access_control_id=oac.id
    )],
    default_cache_behavior=aws.cloudfront.DistributionDefaultCacheBehaviorArgs(
        allowed_methods=["GET", "HEAD"],
        cached_methods=["GET", "HEAD"],
        target_origin_id=web_bucket.id,
        viewer_protocol_policy="redirect-to-https",
        forwarded_values=aws.cloudfront.DistributionDefaultCacheBehaviorForwardedValuesArgs(
            query_string=False,
            cookies=aws.cloudfront.DistributionDefaultCacheBehaviorForwardedValuesCookiesArgs(forward="none")
        )
    ),
    aliases=["slides.khaneducation.ai"],
    viewer_certificate=aws.cloudfront.DistributionViewerCertificateArgs(
        acm_certificate_arn=cert_validation.certificate_arn, # Use the VALIDATED cert ARN
        ssl_support_method="sni-only",
        minimum_protocol_version="TLSv1.2_2021" 
    ),
    restrictions=aws.cloudfront.DistributionRestrictionsArgs(
        geo_restriction=aws.cloudfront.DistributionRestrictionsGeoRestrictionArgs(restriction_type="none")
    )
)

# Final DNS Record for the Alias
dns_record = aws.route53.Record("slides-dns",
    zone_id=zone.id,
    name="slides.khaneducation.ai",
    type="A",
    aliases=[aws.route53.RecordAliasArgs(
        name=distribution.domain_name,
        zone_id=distribution.hosted_zone_id,
        evaluate_target_health=True
    )]
)

# Bucket Policy to allow CloudFront to read
bucket_policy = aws.s3.BucketPolicy("web-bucket-policy",
    bucket=web_bucket.id,
    policy=pulumi.Output.all(web_bucket.arn, distribution.arn).apply(
        lambda args: json.dumps({
            "Version": "2012-10-17",
            "Statement": [{
                "Sid": "AllowCloudFrontServicePrincipal",
                "Effect": "Allow",
                "Principal": {"Service": "cloudfront.amazonaws.com"},
                "Action": "s3:GetObject",
                "Resource": f"{args[0]}/*",
                "Condition": {"StringEquals": {"AWS:SourceArn": args[1]}}
            }]
        })
    )
)


# ==================================================================================
# EXPORTS
# ==================================================================================
pulumi.export("lambda_url", func_url.function_url)
pulumi.export("aurora_endpoint", aurora.endpoint)
pulumi.export("website_url", pulumi.Output.concat("https://", dns_record.name))
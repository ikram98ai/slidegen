#!/usr/bin/env bash
#
# bootstrap.sh — one-command first-time provisioning for Slidegen on AWS.
#
# Idempotent: every step checks for existing resources first, so it is safe
# to re-run after a partial failure or to converge an existing setup.
#
# What it does, in order:
#   1. DynamoDB tables, files S3 bucket, SQS jobs queue (+ DLQ)  [manage CLI]
#   2. Shared IAM role for both Lambdas (DynamoDB + S3 + SQS + logs)
#   3. Builds and deploys the API and worker Lambda functions (arm64)
#   4. Sets runtime env vars, timeouts and memory on both functions
#   5. Connects the queue to the worker (event source mapping with
#      ReportBatchItemFailures)
#   6. Public function URL for the API (this becomes VITE_API_URL)
#   7. Optional: web hosting bucket (static website) if WEB_BUCKET_NAME is set
#
# Requirements: aws CLI v2 (authenticated), cargo-lambda, a filled-in .env
#   (S3_BUCKET_NAME, SECRET_KEY, GEMINI_API_KEY at minimum).
#
# Usage: make bootstrap   (or ./scripts/bootstrap.sh)

set -euo pipefail

cd "$(dirname "$0")/.."

# ──────────────────────────── configuration ────────────────────────────────

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
else
  echo "ERROR: .env not found. Copy .env.example to .env and fill it in." >&2
  exit 1
fi

: "${S3_BUCKET_NAME:?S3_BUCKET_NAME must be set in .env}"
: "${SECRET_KEY:?SECRET_KEY must be set in .env}"
: "${GEMINI_API_KEY:?GEMINI_API_KEY must be set in .env}"

AWS_REGION="${AWS_REGION:-us-east-1}"
TEXT_MODEL="${TEXT_MODEL:-gemini-2.5-flash}"
API_FUNCTION="${LAMBDA_FUNCTION_NAME:-slidegen-lambda}"
WORKER_FUNCTION="${WORKER_FUNCTION_NAME:-slidegen-worker}"
ROLE_NAME="${LAMBDA_ROLE_NAME:-slidegen-lambda-role}"
QUEUE_NAME="slidegen-jobs"   # must match manage.rs

export AWS_REGION
AWS="aws --region $AWS_REGION --no-cli-pager"

log()  { printf '\n\033[1;34m▸ %s\033[0m\n' "$*"; }
ok()   { printf '\033[1;32m✓ %s\033[0m\n' "$*"; }

# ───────────────────────────── preflight ───────────────────────────────────
# Fail fast with clear instructions before touching anything in AWS.

MISSING=0
if ! command -v aws >/dev/null 2>&1; then
  echo "ERROR: the AWS CLI is required. Install: https://docs.aws.amazon.com/cli/" >&2
  MISSING=1
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: Rust/cargo is required. Install: https://rustup.rs" >&2
  MISSING=1
fi
if ! cargo lambda --version >/dev/null 2>&1; then
  echo "ERROR: cargo-lambda is required to build/deploy the Lambda binaries." >&2
  echo "       Install with one of:" >&2
  echo "         brew install cargo-lambda" >&2
  echo "         pip3 install cargo-lambda" >&2
  MISSING=1
fi
if [[ "$MISSING" == "1" ]]; then
  exit 1
fi

if ! ACCOUNT_ID=$($AWS sts get-caller-identity --query Account --output text 2>/dev/null); then
  echo "ERROR: AWS CLI is not authenticated for region $AWS_REGION" >&2
  echo "       ('aws sts get-caller-identity' failed). Run 'aws configure' first." >&2
  exit 1
fi
ok "Preflight OK (account $ACCOUNT_ID, region $AWS_REGION)"

# ─────────────────────── 1. core infrastructure ────────────────────────────

log "Creating DynamoDB tables, files bucket, and jobs queue (idempotent)"
cargo run --quiet --bin manage -- db create-tables
cargo run --quiet --bin manage -- s3 create-bucket
cargo run --quiet --bin manage -- sqs create-queue

QUEUE_URL=$($AWS sqs get-queue-url --queue-name "$QUEUE_NAME" --query QueueUrl --output text)
QUEUE_ARN=$($AWS sqs get-queue-attributes --queue-url "$QUEUE_URL" \
  --attribute-names QueueArn --query 'Attributes.QueueArn' --output text)
ok "Queue ready: $QUEUE_URL"

if [[ -n "${EVENT_BUS_NAME:-}" ]]; then
  if $AWS events describe-event-bus --name "$EVENT_BUS_NAME" >/dev/null 2>&1; then
    ok "Event bus ready: $EVENT_BUS_NAME"
  else
    $AWS events create-event-bus --name "$EVENT_BUS_NAME" >/dev/null
    ok "Event bus created: $EVENT_BUS_NAME"
  fi
fi

# ───────────────────────────── 2. IAM role ─────────────────────────────────

log "Ensuring IAM role '$ROLE_NAME'"
if ! $AWS iam get-role --role-name "$ROLE_NAME" >/dev/null 2>&1; then
  $AWS iam create-role --role-name "$ROLE_NAME" \
    --assume-role-policy-document '{
      "Version": "2012-10-17",
      "Statement": [{
        "Effect": "Allow",
        "Principal": {"Service": "lambda.amazonaws.com"},
        "Action": "sts:AssumeRole"
      }]
    }' >/dev/null
  ok "Role created"
  echo "  Waiting 10s for IAM propagation..."
  sleep 10
else
  ok "Role already exists"
fi

$AWS iam attach-role-policy --role-name "$ROLE_NAME" \
  --policy-arn arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole

$AWS iam put-role-policy --role-name "$ROLE_NAME" \
  --policy-name slidegen-app-access \
  --policy-document "{
    \"Version\": \"2012-10-17\",
    \"Statement\": [
      {
        \"Effect\": \"Allow\",
        \"Action\": [\"dynamodb:GetItem\", \"dynamodb:PutItem\", \"dynamodb:DeleteItem\",
                     \"dynamodb:Query\", \"dynamodb:Scan\", \"dynamodb:BatchWriteItem\"],
        \"Resource\": [
          \"arn:aws:dynamodb:$AWS_REGION:$ACCOUNT_ID:table/slidegen_*\",
          \"arn:aws:dynamodb:$AWS_REGION:$ACCOUNT_ID:table/slidegen_*/index/*\"
        ]
      },
      {
        \"Effect\": \"Allow\",
        \"Action\": [\"s3:GetObject\", \"s3:PutObject\", \"s3:DeleteObject\"],
        \"Resource\": \"arn:aws:s3:::$S3_BUCKET_NAME/*\"
      },
      {
        \"Effect\": \"Allow\",
        \"Action\": [\"sqs:SendMessage\", \"sqs:ReceiveMessage\", \"sqs:DeleteMessage\",
                     \"sqs:GetQueueAttributes\", \"sqs:ChangeMessageVisibility\"],
        \"Resource\": \"$QUEUE_ARN\"
      },
      {
        \"Effect\": \"Allow\",
        \"Action\": [\"events:PutEvents\"],
        \"Resource\": \"*\"
      }
    ]
  }"
ROLE_ARN="arn:aws:iam::$ACCOUNT_ID:role/$ROLE_NAME"
ok "Role policies attached ($ROLE_ARN)"

# ──────────────────────── 3. build + deploy lambdas ────────────────────────

log "Building Lambda binaries (arm64)"
cargo lambda build --release --arm64 --bin slidegen --bin worker

# Waits for both fresh creates (Pending → Active) and code updates to settle,
# so the configuration calls below don't hit ResourceConflictException.
wait_ready() {
  $AWS lambda wait function-active --function-name "$1"
  $AWS lambda wait function-updated --function-name "$1"
}

log "Deploying API function '$API_FUNCTION'"
cargo lambda deploy --binary-name slidegen --region "$AWS_REGION" \
  --iam-role "$ROLE_ARN" "$API_FUNCTION"
wait_ready "$API_FUNCTION"

log "Deploying worker function '$WORKER_FUNCTION'"
cargo lambda deploy --binary-name worker --region "$AWS_REGION" \
  --iam-role "$ROLE_ARN" "$WORKER_FUNCTION"
wait_ready "$WORKER_FUNCTION"

# ─────────────────── 4. runtime configuration (env, limits) ────────────────
# Note: AWS_REGION / AWS credentials are reserved Lambda env keys — the
# runtime provides them, so they are deliberately not set here.

SERVICE_KEYS_SUFFIX=""
if [[ -n "${SERVICE_API_KEYS:-}" ]]; then
  SERVICE_KEYS_SUFFIX=$(printf ',"SERVICE_API_KEYS":"%s"' "$SERVICE_API_KEYS")
fi

EVENT_BUS_SUFFIX=""
if [[ -n "${EVENT_BUS_NAME:-}" ]]; then
  EVENT_BUS_SUFFIX=$(printf ',"EVENT_BUS_NAME":"%s"' "$EVENT_BUS_NAME")
fi

shared_env() {
  printf '{"S3_BUCKET_NAME":"%s","SECRET_KEY":"%s","GEMINI_API_KEY":"%s","TEXT_MODEL":"%s","DEBUG":"false"%s%s}' \
    "$S3_BUCKET_NAME" "$SECRET_KEY" "$GEMINI_API_KEY" "$TEXT_MODEL" "$SERVICE_KEYS_SUFFIX" "$1"
}

log "Configuring API function (env vars, 30s timeout, 512MB)"
$AWS lambda update-function-configuration \
  --function-name "$API_FUNCTION" \
  --timeout 30 --memory-size 512 \
  --environment "{\"Variables\":$(shared_env ",\"JOBS_QUEUE_URL\":\"$QUEUE_URL\"$EVENT_BUS_SUFFIX")}" >/dev/null
$AWS lambda wait function-updated --function-name "$API_FUNCTION"
ok "API configured"

log "Configuring worker function (env vars, 900s timeout, 1024MB)"
$AWS lambda update-function-configuration \
  --function-name "$WORKER_FUNCTION" \
  --timeout 900 --memory-size 1024 \
  --environment "{\"Variables\":$(shared_env ",\"JOBS_QUEUE_URL\":\"$QUEUE_URL\"$EVENT_BUS_SUFFIX")}" >/dev/null
$AWS lambda wait function-updated --function-name "$WORKER_FUNCTION"
ok "Worker configured"

# ─────────────────── 5. SQS → worker event source mapping ──────────────────

log "Connecting queue to worker"
EXISTING_ESM=$($AWS lambda list-event-source-mappings \
  --function-name "$WORKER_FUNCTION" --event-source-arn "$QUEUE_ARN" \
  --query 'EventSourceMappings[0].UUID' --output text)
if [[ "$EXISTING_ESM" == "None" || -z "$EXISTING_ESM" ]]; then
  $AWS lambda create-event-source-mapping \
    --function-name "$WORKER_FUNCTION" \
    --event-source-arn "$QUEUE_ARN" \
    --batch-size 1 \
    --function-response-types ReportBatchItemFailures >/dev/null
  ok "Event source mapping created"
else
  ok "Event source mapping already exists ($EXISTING_ESM)"
fi

# ────────────────────────── 6. API function URL ────────────────────────────

log "Ensuring public function URL for the API"
if ! API_URL=$($AWS lambda get-function-url-config \
  --function-name "$API_FUNCTION" --query FunctionUrl --output text 2>/dev/null); then
  API_URL=$($AWS lambda create-function-url-config \
    --function-name "$API_FUNCTION" --auth-type NONE \
    --query FunctionUrl --output text)
fi
# Allow public invocation of the URL (idempotent: duplicate statement id errors are fine)
$AWS lambda add-permission \
  --function-name "$API_FUNCTION" \
  --statement-id FunctionURLAllowPublicAccess \
  --action lambda:InvokeFunctionUrl \
  --principal '*' \
  --function-url-auth-type NONE >/dev/null 2>&1 || true
API_URL="${API_URL%/}"
ok "API URL: $API_URL"

# ──────────────────── 7. optional web hosting bucket ───────────────────────

if [[ -n "${WEB_BUCKET_NAME:-}" ]]; then
  log "Ensuring web hosting bucket '$WEB_BUCKET_NAME'"
  if ! $AWS s3api head-bucket --bucket "$WEB_BUCKET_NAME" 2>/dev/null; then
    if [[ "$AWS_REGION" == "us-east-1" ]]; then
      $AWS s3api create-bucket --bucket "$WEB_BUCKET_NAME" >/dev/null
    else
      $AWS s3api create-bucket --bucket "$WEB_BUCKET_NAME" \
        --create-bucket-configuration LocationConstraint="$AWS_REGION" >/dev/null
    fi
  fi
  $AWS s3api put-public-access-block --bucket "$WEB_BUCKET_NAME" \
    --public-access-block-configuration \
    BlockPublicAcls=false,IgnorePublicAcls=false,BlockPublicPolicy=false,RestrictPublicBuckets=false
  $AWS s3api put-bucket-policy --bucket "$WEB_BUCKET_NAME" --policy "{
    \"Version\": \"2012-10-17\",
    \"Statement\": [{
      \"Sid\": \"PublicReadGetObject\",
      \"Effect\": \"Allow\",
      \"Principal\": \"*\",
      \"Action\": \"s3:GetObject\",
      \"Resource\": \"arn:aws:s3:::$WEB_BUCKET_NAME/*\"
    }]
  }"
  $AWS s3 website "s3://$WEB_BUCKET_NAME" --index-document index.html --error-document index.html
  WEB_URL="http://$WEB_BUCKET_NAME.s3-website-$AWS_REGION.amazonaws.com"
  ok "Web bucket ready: $WEB_URL"
  echo "  (For HTTPS + caching, put CloudFront in front of this bucket and set"
  echo "   CLOUDFRONT_DISTRIBUTION_ID as a GitHub Actions variable.)"
fi

# ───────────────────────────────── summary ─────────────────────────────────

cat <<EOF

────────────────────────────────────────────────────────────────────────────
Bootstrap complete.

  API URL (use as VITE_API_URL):  $API_URL
  Jobs queue:                     $QUEUE_URL
  Health check:                   curl $API_URL/health

GitHub Actions setup (Settings → Secrets and variables → Actions):
  Secrets:    AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY
  Variables:  VITE_API_URL=$API_URL
              WEB_BUCKET_NAME=${WEB_BUCKET_NAME:-<your web bucket>}
              AWS_REGION=$AWS_REGION
              CLOUDFRONT_DISTRIBUTION_ID=<optional>

Deploy the frontend now with:  make deploy-web
────────────────────────────────────────────────────────────────────────────
EOF

#!/usr/bin/env bash
#
# teardown.sh — destroys everything bootstrap.sh created. THIS DELETES DATA.
#
# Removes, in order:
#   1. SQS → worker event source mapping
#   2. Both Lambda functions (function URL + permissions go with them)
#   3. IAM role (inline + managed policies detached first)
#   4. SQS jobs queue + dead-letter queue
#   5. DynamoDB tables (all data!)
#   6. Files S3 bucket including every object/version (all data!)
#   7. Web hosting bucket, if WEB_BUCKET_NAME is set
#
# Not touched: CloudFront distributions (delete manually if you created one).
#
# Usage: make teardown            (interactive confirmation)
#        FORCE=1 ./scripts/teardown.sh   (skip confirmation — CI/automation)

set -euo pipefail

cd "$(dirname "$0")/.."

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

AWS_REGION="${AWS_REGION:-us-east-1}"
API_FUNCTION="${LAMBDA_FUNCTION_NAME:-slidegen-lambda}"
WORKER_FUNCTION="${WORKER_FUNCTION_NAME:-slidegen-worker}"
ROLE_NAME="${LAMBDA_ROLE_NAME:-slidegen-lambda-role}"
QUEUE_NAME="slidegen-jobs"   # must match manage.rs

export AWS_REGION
AWS="aws --region $AWS_REGION --no-cli-pager"

log()  { printf '\n\033[1;34m▸ %s\033[0m\n' "$*"; }
ok()   { printf '\033[1;32m✓ %s\033[0m\n' "$*"; }
skip() { printf '\033[1;33m– %s\033[0m\n' "$*"; }

# ───────────────────────────── confirmation ────────────────────────────────

if [[ "${FORCE:-0}" != "1" ]]; then
  echo "This will PERMANENTLY DELETE the Slidegen stack in region $AWS_REGION:"
  echo "  Lambdas:   $API_FUNCTION, $WORKER_FUNCTION"
  echo "  IAM role:  $ROLE_NAME"
  echo "  Queues:    $QUEUE_NAME (+ DLQ)"
  echo "  DynamoDB:  slidegen_* tables  (ALL DATA)"
  echo "  S3:        ${S3_BUCKET_NAME:-<files bucket>}${WEB_BUCKET_NAME:+, $WEB_BUCKET_NAME}  (ALL DATA)"
  echo
  read -r -p "Type 'destroy' to confirm: " CONFIRM
  if [[ "$CONFIRM" != "destroy" ]]; then
    echo "Aborted."
    exit 1
  fi
fi

# ──────────────── 1. event source mappings (queue → worker) ────────────────

log "Removing event source mappings for '$WORKER_FUNCTION'"
ESM_UUIDS=$($AWS lambda list-event-source-mappings \
  --function-name "$WORKER_FUNCTION" \
  --query 'EventSourceMappings[].UUID' --output text 2>/dev/null || true)
if [[ -n "$ESM_UUIDS" && "$ESM_UUIDS" != "None" ]]; then
  for uuid in $ESM_UUIDS; do
    $AWS lambda delete-event-source-mapping --uuid "$uuid" >/dev/null
    ok "Deleted event source mapping $uuid"
  done
else
  skip "No event source mappings found"
fi

# ───────────────────────── 2. Lambda functions ─────────────────────────────

for fn in "$API_FUNCTION" "$WORKER_FUNCTION"; do
  log "Deleting Lambda function '$fn'"
  if $AWS lambda get-function --function-name "$fn" >/dev/null 2>&1; then
    $AWS lambda delete-function --function-name "$fn"
    ok "Deleted $fn"
  else
    skip "$fn does not exist"
  fi
done

# ─────────────────────────────── 3. IAM role ───────────────────────────────

log "Deleting IAM role '$ROLE_NAME'"
if $AWS iam get-role --role-name "$ROLE_NAME" >/dev/null 2>&1; then
  $AWS iam delete-role-policy --role-name "$ROLE_NAME" \
    --policy-name slidegen-app-access 2>/dev/null || true
  $AWS iam detach-role-policy --role-name "$ROLE_NAME" \
    --policy-arn arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole 2>/dev/null || true
  $AWS iam delete-role --role-name "$ROLE_NAME"
  ok "Deleted role"
else
  skip "Role does not exist"
fi

# ──────────────────────── 4–6. queue, tables, bucket ───────────────────────

log "Deleting SQS queues"
cargo run --quiet --bin manage -- sqs delete-queue

log "Deleting DynamoDB tables"
cargo run --quiet --bin manage -- db delete-tables

log "Deleting files bucket"
if [[ -n "${S3_BUCKET_NAME:-}" ]] && $AWS s3api head-bucket --bucket "$S3_BUCKET_NAME" 2>/dev/null; then
  cargo run --quiet --bin manage -- s3 delete-bucket
else
  skip "Files bucket does not exist"
fi

# ───────────────────────── 7. web hosting bucket ───────────────────────────

if [[ -n "${WEB_BUCKET_NAME:-}" ]]; then
  log "Deleting web bucket '$WEB_BUCKET_NAME'"
  if $AWS s3api head-bucket --bucket "$WEB_BUCKET_NAME" 2>/dev/null; then
    $AWS s3 rb "s3://$WEB_BUCKET_NAME" --force
    ok "Deleted web bucket"
  else
    skip "Web bucket does not exist"
  fi
fi

cat <<EOF

────────────────────────────────────────────────────────────────────────────
Teardown complete.

Not removed automatically:
  - CloudFront distribution (if you created one) — disable and delete it in
    the console or with: aws cloudfront delete-distribution
  - CloudWatch log groups: /aws/lambda/$API_FUNCTION and
    /aws/lambda/$WORKER_FUNCTION (kept so you can inspect old logs; delete
    with: aws logs delete-log-group --log-group-name <name>)
────────────────────────────────────────────────────────────────────────────
EOF

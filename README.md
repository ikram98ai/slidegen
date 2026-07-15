# Slidegen - Project Documentation

## Overview

Slidegen is an advanced educational platform designed to generate and manage slides from uploaded files. It leverages Rust for high-performance processing and Generative AI (Google Gemini) for intelligent content extraction and slide generation.

## Technology Stack

### Backend (`src/`)

- **Language:** Rust (2024 Edition)
- **Framework:** [Axum](https://github.com/tokio-rs/axum)
- **Runtime:**
  - **Local:** Tokio (TCP Listener)
  - **Cloud:** AWS Lambda ([lambda_http](https://github.com/awslabs/aws-lambda-rust-runtime))
- **Documentation:** [Utoipa](https://github.com/juhakivekas/utoipa) for OpenAPI 3.0 generation and Swagger UI.
- **Data Storage:**
  - **Database:** AWS DynamoDB (Managed via `manage` CLI).
  - **File Storage:** AWS S3 for subject files and generated assets.
- **AI Integration:** Google Gemini.

### Frontend (`web/`)

- **Framework:** React + Vite
- **Styling:** Vanilla CSS (Modern, premium aesthetics)
- **Hosting:** S3 Static Web Hosting + CloudFront CDN.

### Infrastructure & Deployment

- **Deployment Strategy:** Custom Bash scripts using AWS CLI and `cargo-lambda`.
- **Backend Context:** Zip-based Lambda deployment (no Docker required for Lambda).
- **Security:** JWT-based Authentication, IAM OAC for CloudFront-to-S3 security.

---

## Local Development

### Prerequisites

- **Rust:** Latest stable version.
- **Node.js & npm:** For frontend development.
- **AWS CLI:** Configured with credentials.
- **Cargo Lambda:** (Optional, for testing Lambda builds) `pip install cargo-lambda`.

### Environment Setup

1. Copy `.env.example` to `.env` in the root directory.
2. Fill in the required values:
   - `GEMINI_API_KEY`: Your Google Gemini API key.
   - `SECRET_KEY`: A secure random string for JWT (e.g., `openssl rand -base64 32`).
   - `AWS_REGION`: Your target AWS region.
   - `AWS_ACCOUNT_ID`: Your 12-digit AWS account ID.
   - `S3_BUCKET_NAME`: Bucket for file storage.
   - `WEB_BUCKET_NAME`: Bucket for frontend hosting.

### Running the App Locally

The project uses a `makefile` to streamline commands.

- **Start API (Local Dev):**

  ```bash
  make dev
  ```

  API is available at `http://localhost:8000`.
  Swagger UI is at `http://localhost:8000/swagger-ui`.

- **Start Frontend:**
  ```bash
  make dev-web
  ```
  Web UI is available at `http://localhost:5173`.

---

## Infrastructure Management

The `manage` CLI tool (built in Rust) for managing AWS resources.

### Database & Storage Setup

Before your first run, you must create the necessary DynamoDB tables and S3 buckets:

```bash
# Create all DynamoDB tables (Users, Subjects, Chapters, Slides)
make create-tables

# Create the primary S3 bucket for files
make create-bucket

# Create the SQS queue for background jobs (also creates a dead-letter queue)
make create-queue
```

### Resource Teardown

```bash
# Delete all tables
make delete-tables

# Delete S3 bucket and all its contents
make delete-bucket

# Delete SQS queues (jobs queue and DLQ)
make delete-queue
```

---

## Deployment

Deployment has been migrated from Pulumi to custom high-performance scripts.

### 1. Deploy Backend

Cross-compiles for Lambda and updates the function:

```bash
make deploy
```

### 2. Deploy Frontend

Builds the React app with the correct API URL and syncs to S3:

```bash
make deploy-web
```

### 3. CI/CD (GitHub Actions)

Two workflows live in `.github/workflows/`:

- **`ci.yml`** — runs on every pull request: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, plus frontend lint/typecheck/build.
- **`deploy.yml`** — runs on every push to `main` (or manually via the Actions tab). It re-runs the full CI gate, then builds the backend with `cargo lambda` (arm64), updates the Lambda function code, smoke-tests `/health`, builds the frontend with the production `VITE_API_URL`, syncs it to S3, and invalidates CloudFront.

Configure these in **Settings → Secrets and variables → Actions**:

| Kind | Name | Purpose |
| --- | --- | --- |
| Secret | `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` | Deploy credentials (Lambda update, web bucket write, CloudFront invalidation) |
| Variable | `VITE_API_URL` | Public API base URL (Lambda function URL); also used for the health smoke test |
| Variable | `WEB_BUCKET_NAME` | S3 bucket hosting the frontend |
| Variable | `AWS_REGION` | Optional, defaults to `us-east-1` |
| Variable | `LAMBDA_FUNCTION_NAME` | Optional, defaults to `slidegen-lambda` |
| Variable | `CLOUDFRONT_DISTRIBUTION_ID` | Optional; invalidation is skipped when unset |

The pipeline only updates code. One-time provisioning (Lambda creation, DynamoDB tables, S3 buckets, SQS queue, Lambda runtime env vars such as `S3_BUCKET_NAME`, `SECRET_KEY`, `GEMINI_API_KEY`, `JOBS_QUEUE_URL`) is done via the `makefile` targets.

---

## Background Jobs (SQS + Worker Lambda)

Long-running AI work (TOC analysis, slide + audio generation) cannot run in-process on Lambda — the execution environment freezes as soon as the HTTP response is sent. Instead:

- **Locally** (no `JOBS_QUEUE_URL` set): jobs run in-process via `tokio::spawn` — zero setup for development.
- **In production**: the API Lambda enqueues a JSON `Job` message to SQS; a second Lambda (`worker` binary) consumes the queue, executes the job, and reports per-message failures so SQS retries only what failed. After 3 failed attempts a job lands in the dead-letter queue.

All of this is wired automatically by `make bootstrap` (see below). To do it manually instead:

```bash
# 1. Create the jobs queue + DLQ (prints the queue URL and the
#    aws lambda create-event-source-mapping command to run)
make create-queue

# 2. Create/update the worker Lambda
make build && make deploy-worker

# 3. Set JOBS_QUEUE_URL=<queue url> on the API Lambda's environment,
#    and give it sqs:SendMessage; give the worker the same runtime env
#    (S3_BUCKET_NAME, SECRET_KEY, GEMINI_API_KEY, ...) plus DynamoDB/S3 access.
```

---

## One-Command Provisioning & Teardown

With a filled-in `.env` and an authenticated AWS CLI:

```bash
make bootstrap   # zero → fully wired stack (idempotent, safe to re-run)
make teardown    # destroys the whole stack (asks for confirmation; DELETES ALL DATA)
```

`make bootstrap` ([scripts/bootstrap.sh](scripts/bootstrap.sh)) creates, in order: DynamoDB tables, the files S3 bucket, the SQS jobs queue + DLQ, a shared IAM role (DynamoDB/S3/SQS/logs), builds and deploys both Lambda functions (arm64), sets their env vars/timeouts/memory, connects the queue to the worker with `ReportBatchItemFailures`, exposes a public function URL for the API, and (if `WEB_BUCKET_NAME` is set) prepares the web hosting bucket. It finishes by printing the API URL and the exact GitHub Actions secrets/variables to configure for CI/CD.

`make teardown` ([scripts/teardown.sh](scripts/teardown.sh)) removes all of the above in reverse order — event source mapping, both Lambdas, the IAM role, queues, tables, and buckets **including all data**. It requires typing `destroy` to confirm (`FORCE=1` skips the prompt for automation). CloudFront distributions and CloudWatch log groups are intentionally left for manual cleanup.

---

## Testing

Tests are split by Rust convention:

- **Unit tests** live next to the code in `#[cfg(test)]` modules (private helpers: JSON extraction, WAV encoding, UTF-8 truncation, content types).
- **Integration tests** live in [`tests/`](tests/): `ai_service_test.rs` (AI service against a mocked Gemini API via wiremock), `auth_service_test.rs` (JWT + password hashing), `job_format_test.rs` (SQS job wire-format contract).

Makefile shortcuts for development:

```bash
make test         # everything (unit + integration)
make test-unit    # unit tests only
make test-int     # integration tests only
make test-ai      # just the AI service suite
make test-one t=analyze_book_toc   # single test by name, with output
make test-watch   # re-run on file change (needs cargo-watch)
make check        # exactly what CI runs: fmt, clippy -D warnings, tests, web build
```

---

## Directory Structure

- `src/`: Rust Backend application code.
  - `api/`: Route handlers and nested routers.
  - `models/`: Data structures and `utoipa::ToSchema` definitions.
  - `services/`: Core logic (AI, Auth, Storage, Background Tasks).
  - `db/`: DynamoDB client and data access layers.
  - `lib.rs`: Library root (shared by all binaries and integration tests).
  - `main.rs`: API entry point (Hybrid TCP/Lambda server).
  - `worker.rs`: SQS background-jobs worker Lambda.
  - `manage.rs`: Infrastructure management CLI.
- `tests/`: Integration tests (AI service, auth, job wire format).
- `web/`: Frontend React application code.
- `makefile`: Command shorthands for development and deployment.
- `Dockerfile`: (Legacy) Used for optional containerized builds.

---

## OpenAPI & Documentation

Slidegen features automatic API documentation. When the backend is running, visit:

- **Interactive UI**: `/swagger-ui`
- **JSON Spec**: `/api-docs/openapi.json`

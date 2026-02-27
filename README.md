# Slidegen - Project Documentation

## Overview

Slidegen is an advanced educational platform designed to generate and manage educational content (slides and chapters) from uploaded files. It leverages Rust for high-performance processing and Generative AI (Google Gemini) for intelligent content extraction and slide generation.

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
```

### Resource Teardown

```bash
# Delete all tables
make delete-tables

# Delete S3 bucket and all its contents
make delete-bucket
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

---

## Directory Structure

- `src/`: Rust Backend application code.
  - `api/`: Route handlers and nested routers.
  - `models/`: Data structures and `utoipa::ToSchema` definitions.
  - `services/`: Core logic (AI, Auth, Storage, Background Tasks).
  - `db/`: DynamoDB client and data access layers.
  - `main.rs`: Application entry point (Hybrid TCP/Lambda server).
  - `manage.rs`: Infrastructure management CLI.
- `web/`: Frontend React application code.
- `makefile`: Command shorthands for development and deployment.
- `Dockerfile`: (Legacy) Used for optional containerized builds.

---

## OpenAPI & Documentation

Slidegen features automatic API documentation. When the backend is running, visit:

- **Interactive UI**: `/swagger-ui`
- **JSON Spec**: `/api-docs/openapi.json`

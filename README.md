# Lumina - Project Documentation

## Overview
Lumina (deployed as `slides.khaneducation.ai`) is an educational platform designed to generate and manage educational content (slides/chapters) from uploaded files. It leverages Generative AI (Google Gemini) for content processing.

## Architecture

### Backend (`app/`)
- **Framework:** FastAPI (Python).
- **Runtime:** AWS Lambda (containerized via Docker).
- **Key Dependencies:** `fastapi`, `uv` (package manager), `mangum` (Lambda adapter), `google-generativeai` (implied).
- **Data Storage:**
  - **Database:** AWS DynamoDB (Tables: `lumina*`).
  - **File Storage:** AWS S3 (`lumina-files` bucket).
- **AI Integration:** Google Gemini API.

### Frontend (`web/`)
- **Framework:** React (Vite).
- **Styling:** Tailwind CSS.
- **State Management:** Zustand (inferred from store structure).
- **Hosting:** AWS S3 bucket served via CloudFront CDN.

### Infrastructure (`Pulumi.yaml`, `__main__.py`)
- **IaC Tool:** Pulumi (Python).
- **Stack:**
  - **Compute:** AWS Lambda (Docker Image from ECR).
  - **Frontend:** S3 + CloudFront + Route53 (Custom Domain: `slides.khaneducation.ai`).
  - **Storage:** S3 (Files), DynamoDB (Data).
  - **Security:** IAM Roles/Policies for Lambda (S3/DynamoDB access).

## Local Development

### Prerequisites
- Python 3.x (with `uv` installed).
- Node.js & npm.
- Docker (optional, for container testing).
- AWS CLI (configured if interacting with real AWS resources).

### Environment Setup
1. Copy `.env.example` to `.env` in the root directory.
2. Fill in the required values:
   - `GEMINI_API_KEY`: Your Google Gemini API key.
   - `SECRET_KEY`: A secure random string for JWT.
   - AWS credentials (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`).

### Running the App
The project uses a `makefile` to streamline commands.

- **Run Backend & Frontend:**
  ```bash
  make dev
  ```
  This will start:
  - API at `http://localhost:8000` (via `uv run fastapi dev`)
  - Web UI at `http://localhost:5173` (via `npm run dev`)

- **Run Backend Only:**
  ```bash
  make dev-api
  ```

- **Run Frontend Only:**
  ```bash
  make dev-web
  ```

- **Linting & Formatting:**
  ```bash
  make lint    # Check for issues
  make fix     # Fix auto-fixable issues
  make format  # Format code
  ```

- **Docker Build (Local Lambda Test):**
  ```bash
  make build
  ```

## Deployment

Deployment is managed via Pulumi.

1. **Setup Pulumi:** Ensure you have the Pulumi CLI installed and configured.
2. **Deploy:**
   ```bash
   pulumi up
   ```
   This will provision/update all AWS resources defined in `__main__.py`.

## Directory Structure
- `app/`: Backend FastAPI application code.
  - `api/`: API route handlers.
  - `services/`: Business logic (Auth, AI, Storage).
- `web/`: Frontend React application code.
- `Pulumi.yaml` & `__main__.py`: Infrastructure as Code definitions.
- `Dockerfile`: Container definition for the Lambda function.

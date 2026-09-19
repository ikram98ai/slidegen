# Slidegen

Slidegen turns an uploaded book into **interactive chapter packages**: a typed SceneSpec compiled to self-contained HTML (kit + player + CSS), stored on S3, and played in a sandboxed iframe. Gemini never writes raw HTML. Quotes are verified against the page extract before compile.

The slidegen UI uses JWT. Sibling backends (khaneducation, knoio, …) use `X-Api-Key` and the `/api/v1` service API.

## Technology Stack

### Backend (`src/`)

- **Language:** Rust (2024 Edition)
- **Framework:** [Axum](https://github.com/tokio-rs/axum)
- **Runtime:**
  - **API:** Tokio locally; AWS Lambda (`lambda_http`) in production
  - **Worker:** same `worker` binary on Lambda (SQS event source) or Fargate (long-poll)
- **Documentation:** [Utoipa](https://github.com/juhakivekas/utoipa) OpenAPI 3 + Swagger UI
- **Storage:** DynamoDB (users, subjects, chapters, slides, jobs); S3 (source PDF, extract pages, chapter packages, audio)
- **Queue / events:** SQS work queue + optional EventBridge fan-out (`book.processed`, `chapter.ready`, `job.failed`)
- **AI:** Google Gemini (text, TTS, embeddings)
- **Retrieval (optional):** Qdrant; lexical fallback over the extract when `QDRANT_URL` is unset

### Frontend (`web/`)

- **Framework:** React + Vite
- **Reader:** sandboxed iframe of the compiled chapter (`allow-scripts`, `allow="autoplay"`)
- **Hosting:** S3 + CloudFront

### Infrastructure

- Custom Bash + AWS CLI + `cargo-lambda` (`make bootstrap`)
- Optional Fargate Spot worker: `Dockerfile` + `deploy/fargate-worker.json`

---

## Local Development

### Prerequisites

- Rust (stable), Node.js + pnpm, AWS CLI with credentials
- Optional: `cargo-lambda` for Lambda builds

### Environment Setup

1. Copy `.env.example` to `.env`.
2. Required:
   - `GEMINI_API_KEY`, `SECRET_KEY` (`openssl rand -base64 32`), `S3_BUCKET_NAME`
   - `AWS_REGION`, `AWS_ACCOUNT_ID`
3. Optional:
   - `JOBS_QUEUE_URL` — unset locally so jobs run in-process
   - `SERVICE_API_KEYS=khaneducation:sk_…,knoio:sk_…` — tenant keys for `/api/v1`
   - `EVENT_BUS_NAME` — outbound EventBridge
   - `AUTO_GENERATE_CHAPTERS` — default `true`; set `false` to skip sequential chapter jobs after TOC
   - `QDRANT_URL` / `QDRANT_API_KEY` / `QDRANT_COLLECTION` — vector search (else lexical Ask)
   - `EMBEDDING_MODEL` / `EMBEDDING_DIMENSIONS` — default `gemini-embedding-001` / `768`

### Running locally

```bash
make run          # API at http://localhost:8000  (Swagger: /swagger-ui)
cd web && pnpm dev
```

---

## Product pipeline

1. **Ingest** a PDF (UI upload or `POST /api/v1/books`).
2. **Extract** every page to S3 (`paragraph_id` like `p12-2`, printed-page offset).
3. **TOC** → chapter rows.
4. **GenerateChapter** (one at a time when `AUTO_GENERATE_CHAPTERS` is on): plan scenes from retrieved paragraphs, generate SceneSpec, verify citations, TTS for depth text, compile HTML, upload package.
5. **Index** paragraphs (Gemini embeddings → Qdrant when configured).
6. **Play** via a short-lived presigned embed URL in a cross-origin iframe.

Legacy **GenerateSlides** still exists for old queue messages and `POST /api/chapters/{subject}/{chapter}/slides/generate`. The UI and `POST …/generate` start **GenerateChapter**.

---

## Service API (`/api/v1`)

Authenticate with `X-Api-Key` (value from `SERVICE_API_KEYS`).

| Method | Path | Purpose |
| --- | --- | --- |
| POST | `/api/v1/books` | Multipart upload + process job |
| POST | `/api/v1/books/ingest` | Book already in S3 (`file_s3path`) |
| GET | `/api/v1/jobs/{id}` | Job progress |
| GET | `/api/v1/books/{id}` | Book |
| GET | `/api/v1/books/{id}/chapters` | TOC + per-chapter status |
| GET | `/api/v1/books/{id}/chapters/{cid}` | Chapter + optional `embed_url` |
| GET | `/api/v1/books/{id}/chapters/{cid}/embed` | Presigned iframe URL |
| POST | `/api/v1/books/{id}/chapters/{cid}/generate` | Enqueue GenerateChapter |
| POST | `/api/v1/books/{id}/search` | Cited passages (vector or lexical) |
| POST | `/api/v1/books/{id}/chapters/{cid}/ask` | Grounded answer + citations |
| GET | `/api/v1/books/{id}/chapters/{cid}/scenes/{sid}` | SceneSpec |
| GET | `/api/v1/books/{id}/paragraphs/{pid}` | Paragraph + page |

JWT users poll the same jobs at `GET /api/jobs/{id}` and generate via `POST /api/chapters/{subject}/{chapter}/generate`.

Inbound SQS (skip HTTP): `{"type":"ingest_book","tenant_id":"khan","title":"…","book_type":"book","file_s3path":"…"}`.

---

## MCP

`slidegen-mcp` is a stdio MCP server wrapping the v1 API.

```bash
SLIDEGEN_BASE_URL=http://127.0.0.1:8000 SLIDEGEN_API_KEY=sk_… cargo run --bin slidegen-mcp
```

Tools: `search_book`, `get_chapter`, `get_scene`, `get_citation`. Every result includes page / `paragraph_id`.

---

## Infrastructure Management

```bash
make create-tables   # Users, Subjects, Chapters, Slides, Jobs
make create-bucket
make create-queue    # jobs queue + DLQ
make delete-tables
make delete-bucket
make delete-queue
```

---

## Background jobs

Long-running work cannot stay in-process on the API Lambda (the runtime freezes after the HTTP response).

- **Local** (no `JOBS_QUEUE_URL`): in-process job channel.
- **Production API**: writes `slidegen_jobs`, enqueues SQS.
- **Worker Lambda:** SQS event source, `ReportBatchItemFailures`, 900s timeout. After 3 failures the message goes to the DLQ.
- **Worker Fargate:** same binary long-polls SQS when `AWS_LAMBDA_RUNTIME_API` is unset. `JOBS_QUEUE_URL` is required. `make docker-worker` builds the image; register `deploy/fargate-worker.json` (Spot, scale 0–N on queue depth).

Job types: `process_subject`, `generate_chapter`, `generate_slides` (legacy), `ingest_book`.

Poll `GET /api/jobs/{job_id}` (JWT or `X-Api-Key`). Optional EventBridge (`EVENT_BUS_NAME`) notifies siblings so they do not share the work queue.

```bash
make create-queue
make build && make deploy-worker
# Set JOBS_QUEUE_URL on API and worker; worker also needs DynamoDB/S3/SQS (and events:PutEvents if using EventBridge).
```

---

## Deployment

```bash
make deploy          # API Lambda
make deploy-worker   # worker Lambda
make deploy-web      # React → S3
make docker-worker   # worker container for Fargate
```

### CI/CD (GitHub Actions)

- **`ci.yml`** — PR: `cargo fmt --check`, `clippy -D warnings`, `cargo test`, frontend lint/typecheck/build.
- **`deploy.yml`** — push to `main`: CI, `cargo lambda` arm64, update API Lambda, smoke `/health`, sync web, CloudFront invalidate.

| Kind | Name | Purpose |
| --- | --- | --- |
| Secret | `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` | Deploy |
| Variable | `VITE_API_URL` | Public API URL + health check |
| Variable | `WEB_BUCKET_NAME` | Frontend bucket |
| Variable | `AWS_REGION` | Optional, default `us-east-1` |
| Variable | `LAMBDA_FUNCTION_NAME` | Optional, default `slidegen-lambda` |
| Variable | `CLOUDFRONT_DISTRIBUTION_ID` | Optional |

One-time provisioning (tables, buckets, queue, Lambda env: `S3_BUCKET_NAME`, `SECRET_KEY`, `GEMINI_API_KEY`, `JOBS_QUEUE_URL`, optional `SERVICE_API_KEYS` / `EVENT_BUS_NAME` / `QDRANT_*`) is `make bootstrap`, not CI.

```bash
make bootstrap   # idempotent full stack
make teardown    # type `destroy` (or FORCE=1); deletes data. CloudFront/logs left for manual cleanup.
```

---

## Testing

- **Unit:** `#[cfg(test)]` next to the code (citations, compiler, extract, retrieval, MCP protocol).
- **Integration:** [`tests/`](tests/) — Gemini mock (`ai_service_test`), JWT (`auth_service_test`), SQS job JSON (`job_format_test`).

```bash
make test
make test-unit
make test-int
make test-ai
make test-one t=analyze_book_toc
make check        # fmt, clippy -D warnings, tests, web build
```

---

## Directory Structure

- `src/api/` — JWT routes + `v1` service API
- `src/models/` — subjects, chapters, jobs, SceneSpec
- `src/services/` — AI, extract, citations, compiler, EventBridge, Qdrant, retrieval, SQS jobs
- `src/runtime/v1/` — kit.js, player.js, style.css (compiled into each package)
- `src/mcp.rs` + `src/bin/mcp.rs` — MCP stdio server
- `src/worker.rs` — Lambda event source **or** Fargate long-poll
- `src/manage.rs` — tables / bucket / queue CLI
- `deploy/fargate-worker.json` — ECS task definition
- `Dockerfile` — worker image (not the API)
- `web/` — React app
- `tests/` — integration tests

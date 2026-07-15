rust-version:
	@echo "Rust command-line utility versions:"
	rustc --version 			#rust compiler
	cargo --version 			#rust package manager
	rustfmt --version			#rust code formatter
	rustup --version			#rust toolchain manager
	clippy-driver --version		#rust linter

format:
	cargo fmt --quiet

lint:
	cargo clippy --quiet

#### Testing Section ####
## Run everything (unit + integration)
test:
	cargo test

## Unit tests only (inline #[cfg(test)] modules in src/)
test-unit:
	cargo test --lib --bins

## Integration tests only (tests/ folder)
test-int:
	cargo test --tests

## One integration suite, e.g. `make test-ai`
test-ai:
	cargo test --test ai_service_test

test-auth:
	cargo test --test auth_service_test

test-jobs:
	cargo test --test job_format_test

## Run a single test by name: `make test-one t=analyze_book_toc`
test-one:
	cargo test $(t) -- --nocapture

## Re-run tests on file change (requires: cargo install cargo-watch)
test-watch:
	cargo watch -x "test --lib --bins" -x "test --tests"

## Everything CI runs, locally — catch failures before pushing
check:
	cargo fmt --all --check
	cargo clippy --all-targets -- -D warnings
	cargo test
	cd web && pnpm build

install:
	cargo install --path .
	cd web && pnpm install

#### Cargo Lambda Section ####
## Watches for changes and rebuilds
watch:
	cargo lambda watch

invoke:
	cargo lambda invoke --data-ascii "{}"

### Build for AWS Lambda (use arm64 for AWS Graviton2)
build:
	cargo lambda build --release --arm64 --bin slidegen --bin worker

### Deploy the API Lambda
deploy:
	cargo lambda deploy --binary-name slidegen --region us-east-1 slidegen-lambda

### Deploy the SQS background-jobs worker Lambda
deploy-worker:
	cargo lambda deploy --binary-name worker --region us-east-1 slidegen-worker

### Invoke on AWS
aws-invoke:
	cargo lambda invoke --remote slidegen-lambda --data-ascii "{}"

run:
	cargo run --bin slidegen

release:
	cargo build --release

### Deploy React Frontend to S3 and CloudFront
deploy-web:
	cd web && npm run deploy

all: format lint test run

#### One-command provisioning ####
# First-time setup: tables, buckets, queue, IAM role, both Lambdas,
# event source mapping, function URL. Idempotent — safe to re-run.
bootstrap:
	./scripts/bootstrap.sh

# Tear EVERYTHING down (asks for confirmation; deletes all data).
teardown:
	./scripts/teardown.sh

# Create all DynamoDB tables (Users, Subjects, Chapters, Slides)
create-tables:
	cargo run --bin manage -- db create-tables

# Delete all tables
delete-tables:
	cargo run --bin manage -- db delete-tables

# Create the primary S3 bucket for files
create-bucket:
	cargo run --bin manage -- s3 create-bucket

# Delete S3 bucket and all its contents
delete-bucket:
	cargo run --bin manage -- s3 delete-bucket

# Create the SQS background-jobs queue (+ dead-letter queue)
create-queue:
	cargo run --bin manage -- sqs create-queue

# Delete the SQS background-jobs queue and its DLQ
delete-queue:
	cargo run --bin manage -- sqs delete-queue
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

test:
	cargo test --quiet

#### Cargo Lambda Section ####
## Watches for changes and rebuilds
watch:
	cargo lambda watch

invoke:
	cargo lambda invoke --data-ascii "{}"

### Build for AWS Lambda (use arm64 for AWS Graviton2)
build:
	cargo lambda build --release --arm64

deploy:
	cargo lambda deploy --region us-east-1

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
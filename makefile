include .env

sync:
	uv sync
lint:
	uv run ruff check

fix:
	uv run ruff check --fix

format:
	uv run ruff format

dev: 
	uv run fastapi dev app/main.py



build:
	@echo "Building Docker image..."
	docker build -t lumina-lambda .

	@echo "Running Docker container..."
	docker run -p 8000:8000 \
		-e GEMINI_API_KEY=${GEMINI_API_KEY} \
		-e SECRET_KEY=${SECRET_KEY} \
		-e DEBUG=True \
		lumina-lambda


set-secrets:
	@echo "Setting GitHub Actions secrets..."
	gh secret set AWS_REGION --body ${AWS_REGION}
	gh secret set AWS_ACCESS_KEY_ID --body ${AWS_ACCESS_KEY_ID}
	gh secret set AWS_SECRET_ACCESS_KEY --body ${AWS_SECRET_ACCESS_KEY}
	gh secret set GEMINI_API_KEY --body ${GEMINI_API_KEY}
	gh secret set SECRET_KEY --body ${SECRET_KEY}
fix:
	uv run ruff check --fix

format:
	uv run ruff format

dev:
	uv run fastapi dev app/main.py
# Worker image for ECS Fargate Spot. API stays on Lambda.
FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
# Frontend is not part of the worker binary.
RUN cargo build --release --bin worker

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/worker /usr/local/bin/worker
USER nobody
CMD ["worker"]

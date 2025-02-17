# syntax=docker/dockerfile:1

# Build stage ---------------

FROM rust:1.84.1 AS builder

WORKDIR /app
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release

# Runtime stage -------------

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/shortbot shortbot
COPY config config
COPY data data
ENTRYPOINT [ "./shortbot" ]
# Stage 1: Plan dependencies
FROM lukemathwalker/cargo-chef:latest-rust-nightly AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Build dependencies (cached layer)
FROM chef AS builder
ARG FEATURES=""
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this layer is cached!
RUN if [ -n "$FEATURES" ]; then \
      cargo +nightly chef cook --release --features "$FEATURES" --recipe-path recipe.json; \
    else \
      cargo +nightly chef cook --release --recipe-path recipe.json; \
    fi

# Build application
COPY . .
RUN if [ -n "$FEATURES" ]; then \
      cargo +nightly build --release --features "$FEATURES" --bin digging; \
    else \
      cargo +nightly build --release --bin digging; \
    fi

# Stage 3: Runtime
FROM debian:trixie-slim AS runtime
WORKDIR /app

# Install runtime dependencies (OpenSSL, CA certs)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      ca-certificates \
      libssl3 && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 appuser && \
    chown -R appuser:appuser /app
USER appuser

# Copy binary from builder
COPY --from=builder /app/target/release/digging /usr/local/bin/digging

# Default bind address (can be overridden via ENV)
ENV BIND_ADDRESS=0.0.0.0:3000
EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/digging"]

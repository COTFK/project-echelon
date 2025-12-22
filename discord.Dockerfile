FROM rust:trixie AS chef
WORKDIR /app

# Install cargo-chef
RUN cargo install cargo-chef

# Planner stage to analyze dependencies
FROM chef AS planner
COPY packages/echelon-discord .
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage to compile the application
FROM chef AS builder
ARG CARGO_BUILD_JOBS=8
ENV CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS

# Cache dependencies
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code last (most likely to change)
COPY packages/echelon-discord .

# Build the application
RUN cargo build --release

FROM debian:trixie-slim AS runtime

# Install CA certificates and procps (for healthcheck)
RUN apt-get update && apt-get install -y ca-certificates procps && rm -rf /var/lib/apt/lists/*

# Copy bot binary
COPY --from=builder /app/target/release/echelon-discord /usr/local/bin/echelon-discord

# Simple health check: verify process is running
HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
  CMD pgrep -f echelon-discord > /dev/null || exit 1

ENTRYPOINT [ "/usr/local/bin/echelon-discord" ]
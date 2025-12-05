FROM rust:trixie AS chef
WORKDIR /app

# Install cargo-chef
RUN cargo install cargo-chef

# Planner stage to analyze dependencies
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage to compile the application
FROM chef AS builder
ARG CARGO_BUILD_JOBS=8
ENV CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS

# Cache dependencies
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code last (most likely to change)
COPY . .

# Build the application
RUN cargo build --release --package echelon-server

FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates curl xvfb ffmpeg \
    && rm -rf /var/lib/apt/lists/*

ADD https://github.com/ProjectIgnis/edopro-assets/releases/download/41.0.2/ProjectIgnis-EDOPro-41.0.2-linux.tar.gz /usr/local/
RUN tar -xzf /usr/local/ProjectIgnis-EDOPro-41.0.2-linux.tar.gz -C /usr/local/ && rm /usr/local/ProjectIgnis-EDOPro-41.0.2-linux.tar.gz

COPY packages/echelon-server/deployment/EDOPro /usr/local/ProjectIgnis/
COPY packages/echelon-server/deployment/config/configs.json /usr/local/ProjectIgnis/config/configs.json
COPY packages/echelon-server/deployment/config/system.conf /usr/local/ProjectIgnis/config/system.conf
COPY packages/echelon-server/deployment/textures/bg.png /usr/local/ProjectIgnis/textures/bg.png
COPY packages/echelon-server/deployment/textures/field3.png /usr/local/ProjectIgnis/textures/field3.png
COPY packages/echelon-server/deployment/textures/field-transparent3.png /usr/local/ProjectIgnis/textures/field-transparent3.png
RUN chmod +x /usr/local/ProjectIgnis/EDOPro

# Copy server binary
COPY --from=builder /app/target/release/echelon-server /usr/local/bin/echelon-server

HEALTHCHECK --interval=30s --timeout=30s --retries=3 CMD curl -f http://127.0.0.1:3000/health || exit 1

ENV EDOPRO_PATH=/usr/local/ProjectIgnis/EDOPro

ENTRYPOINT [ "/usr/local/bin/echelon-server" ]
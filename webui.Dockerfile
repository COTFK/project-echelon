# Build stage - compile Dioxus web app
FROM rust:bookworm AS builder
WORKDIR /app

# Install Dioxus CLI and wasm target
RUN cargo install dioxus-cli && \
    rustup target add wasm32-unknown-unknown

# Copy workspace files
COPY packages/echelon-webui ./packages/echelon-webui

# API URL - defaults to production, can be overridden
ARG API_BASE_URL="https://echelon-server.arqalite.org"
ENV API_BASE_URL=$API_BASE_URL

# Build the web app
WORKDIR /app/packages/echelon-webui
RUN dx bundle --platform web --release --debug-symbols=false --out-dir bundle

# Runtime stage - serve with nginx
FROM nginx:alpine AS runtime

# Copy built assets to nginx
COPY --from=builder /app/packages/echelon-webui/bundle/public /usr/share/nginx/html

# Copy custom nginx config
COPY --from=builder /app/packages/echelon-webui/config/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]

# Build stage - compile Dioxus web app
FROM rust:bookworm AS builder
WORKDIR /app

# Install Dioxus CLI and wasm target
RUN cargo install dioxus-cli && \
    rustup target add wasm32-unknown-unknown

# Copy workspace files
COPY packages/echelon-webui ./packages/echelon-webui

# Build the web app
WORKDIR /app/packages/echelon-webui
RUN dx build --release

# Runtime stage - serve with nginx
FROM nginx:alpine AS runtime

# Copy built assets to nginx
COPY --from=builder /app/packages/echelon-webui/target/dx/echelon-webui/release/web/public /usr/share/nginx/html

# Copy custom nginx config for SPA routing
COPY <<EOF /etc/nginx/conf.d/default.conf
server {
    listen 80;
    listen [::]:80;
    server_name localhost;
    root /usr/share/nginx/html;
    index index.html;

    # Gzip compression
    gzip on;
    gzip_types text/plain text/css application/json application/javascript text/xml application/xml application/wasm;

    # Cache static assets
    location ~* \.(js|css|wasm|png|jpg|jpeg|gif|ico|svg)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    # SPA fallback - serve index.html for all routes
    location / {
        try_files \$uri \$uri/ /index.html;
    }
}
EOF

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]

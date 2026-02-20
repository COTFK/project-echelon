#!/bin/bash
set -euo pipefail

REGISTRY="git.arqalite.org/cotfk"
VERSION="${1:-}"

docker login git.arqalite.org

echo "🔨 Building Discord image..."
docker build -f discord.Dockerfile -t "${REGISTRY}/echelon-discord:latest" .

echo "🔨 Building Server image..."
docker build -f server.Dockerfile -t "${REGISTRY}/echelon-server:latest" .

echo "🔨 Building WebUI image..."
docker build -f webui.Dockerfile -t "${REGISTRY}/echelon-webui:latest" .

# Tag with version if provided
if [[ -n "$VERSION" ]]; then
    echo "🏷️  Tagging with version: $VERSION"
    docker tag "${REGISTRY}/echelon-discord:latest" "${REGISTRY}/echelon-discord:${VERSION}"
    docker tag "${REGISTRY}/echelon-server:latest" "${REGISTRY}/echelon-server:${VERSION}"
    docker tag "${REGISTRY}/echelon-webui:latest" "${REGISTRY}/echelon-webui:${VERSION}"
fi

echo "📤 Pushing Discord image..."
docker push "${REGISTRY}/echelon-discord:latest"

echo "📤 Pushing Server image..."
docker push "${REGISTRY}/echelon-server:latest"

echo "📤 Pushing WebUI image..."
docker push "${REGISTRY}/echelon-webui:latest"

# Push versioned tags if provided
if [[ -n "$VERSION" ]]; then
    echo "📤 Pushing versioned tags..."
    docker push "${REGISTRY}/echelon-discord:${VERSION}"
    docker push "${REGISTRY}/echelon-server:${VERSION}"
    docker push "${REGISTRY}/echelon-webui:${VERSION}"
fi

echo "✅ All images built and pushed successfully!"
echo "   Run ./deploy.sh ${VERSION:-latest} to deploy."

#!/bin/bash
set -euo pipefail

REGISTRY="git.arqalite.org/cotfk/project-echelon"
VERSION="${1:-}"

docker login git.arqalite.org

echo "🔨 Building Discord image..."
docker build -f discord.Dockerfile -t "${REGISTRY}:discord-latest" .

echo "🔨 Building Server image..."
docker build -f server.Dockerfile -t "${REGISTRY}:server-latest" .

echo "🔨 Building WebUI image..."
docker build -f webui.Dockerfile -t "${REGISTRY}:webui-latest" .

# Tag with version if provided
if [[ -n "$VERSION" ]]; then
    echo "🏷️  Tagging with version: $VERSION"
    docker tag "${REGISTRY}:discord-latest" "${REGISTRY}:discord-${VERSION}"
    docker tag "${REGISTRY}:server-latest" "${REGISTRY}:server-${VERSION}"
    docker tag "${REGISTRY}:webui-latest" "${REGISTRY}:webui-${VERSION}"
fi

echo "📤 Pushing Discord image..."
docker push "${REGISTRY}:discord-latest"

echo "📤 Pushing Server image..."
docker push "${REGISTRY}:server-latest"

echo "📤 Pushing WebUI image..."
docker push "${REGISTRY}:webui-latest"

# Push versioned tags if provided
if [[ -n "$VERSION" ]]; then
    echo "📤 Pushing versioned tags..."
    docker push "${REGISTRY}:discord-${VERSION}"
    docker push "${REGISTRY}:server-${VERSION}"
    docker push "${REGISTRY}:webui-${VERSION}"
fi

caprover deploy -i git.arqalite.org/cotfk/project-echelon:server-latest -n arqalite -a echelon-server
caprover deploy -i git.arqalite.org/cotfk/project-echelon:discord-latest -n arqalite -a echelon-discord
caprover deploy -i git.arqalite.org/cotfk/project-echelon:webui-latest -n arqalite -a echelon

echo "✅ All images built and pushed successfully!"

#!/bin/bash
set -euo pipefail

REGISTRY="git.arqalite.org/cotfk"
VERSION="${1:-latest}"

echo "🚀 Deploying version: $VERSION"

caprover deploy -i "${REGISTRY}/echelon-server:${VERSION}" -n arqalite -a echelon-server
caprover deploy -i "${REGISTRY}/echelon-discord:${VERSION}" -n arqalite -a echelon-discord
caprover deploy -i "${REGISTRY}/echelon-webui:${VERSION}" -n arqalite -a echelon

echo "✅ Deployed ${VERSION} successfully!"

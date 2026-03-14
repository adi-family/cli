#!/usr/bin/env bash
# Deploy app (frontend SPA) to GCP Cloud Run
# Usage: ./deploy.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Load config from .env (values can be overridden by environment)
if [[ -f "$SCRIPT_DIR/.env" ]]; then
  set -a
  # shellcheck source=.env
  source "$SCRIPT_DIR/.env"
  set +a
fi

IMAGE="${GCP_REGION}-docker.pkg.dev/${GCP_PROJECT}/${REPO_NAME}/app"
TAG="$(git -C "$PROJECT_ROOT" rev-parse --short HEAD)"

# Swap .dockerignore for app build (root one excludes apps/)
ORIG_DOCKERIGNORE="$PROJECT_ROOT/.dockerignore"
BACKUP_DOCKERIGNORE="$PROJECT_ROOT/.dockerignore.bak"
cp "$ORIG_DOCKERIGNORE" "$BACKUP_DOCKERIGNORE"
cp "$SCRIPT_DIR/.dockerignore" "$ORIG_DOCKERIGNORE"
trap 'mv "$BACKUP_DOCKERIGNORE" "$ORIG_DOCKERIGNORE"' EXIT

echo "==> Building app image (tag: $TAG, platform: linux/amd64)"
docker build \
  --platform linux/amd64 \
  -t "${IMAGE}:${TAG}" \
  -t "${IMAGE}:latest" \
  -f "$SCRIPT_DIR/Dockerfile" \
  "$PROJECT_ROOT"

echo "==> Pushing image"
docker push "${IMAGE}:${TAG}"
docker push "${IMAGE}:latest"

echo "==> Deploying to Cloud Run (${SERVICE_NAME})"
gcloud run deploy "$SERVICE_NAME" \
  --project="$GCP_PROJECT" \
  --image="${IMAGE}:${TAG}" \
  --region="$GCP_REGION" \
  --port=8080 \
  --memory=512Mi \
  --cpu=1 \
  --min-instances=0 \
  --max-instances=3 \
  --no-invoker-iam-check \
  --execution-environment=gen2

echo "==> Done"
gcloud run services describe "$SERVICE_NAME" \
  --project="$GCP_PROJECT" \
  --region="$GCP_REGION" \
  --format='value(status.url)'

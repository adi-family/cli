#!/usr/bin/env bash
# Deploy CLI registry to GCP Cloud Run
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

# Resolve op:// references via 1Password CLI
if [[ "${REGISTRY_AUTH_TOKEN:-}" == op://* ]]; then
  echo "==> Resolving registry token from 1Password"
  REGISTRY_AUTH_TOKEN="$(op read "$REGISTRY_AUTH_TOKEN")"
fi

IMAGE="${GCP_REGION}-docker.pkg.dev/${GCP_PROJECT}/${REPO_NAME}/cli-registry"
TAG="$(git -C "$PROJECT_ROOT" rev-parse --short HEAD)"

# Collect container env vars (non-empty SERVICE_ENV_* vars from .env)
SERVICE_ENVS="REGISTRY_DATA_DIR=${REGISTRY_DATA_DIR}"
[[ -n "${REGISTRY_AUTH_TOKEN:-}" ]] && SERVICE_ENVS+=",REGISTRY_AUTH_TOKEN=${REGISTRY_AUTH_TOKEN}"
[[ -n "${RUST_LOG:-}" ]] && SERVICE_ENVS+=",RUST_LOG=${RUST_LOG}"

echo "==> Building CLI registry image (tag: $TAG, platform: linux/amd64)"
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
  --allow-unauthenticated \
  --set-env-vars="$SERVICE_ENVS" \
  --add-volume=name=registry-data,type=cloud-storage,bucket="$GCS_BUCKET" \
  --add-volume-mount=volume=registry-data,mount-path=/data \
  --execution-environment=gen2

echo "==> Done"
gcloud run services describe "$SERVICE_NAME" \
  --project="$GCP_PROJECT" \
  --region="$GCP_REGION" \
  --format='value(status.url)'

#!/usr/bin/env bash
# Deploy llm-proxy to GCP Cloud Run
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
resolve_op() {
  local var_name="$1"
  local val="${!var_name:-}"
  if [[ "$val" == op://* ]]; then
    echo "==> Resolving $var_name from 1Password"
    eval "$var_name=\"$(op read "$val")\""
  fi
}

resolve_op DATABASE_URL
resolve_op JWT_SECRET
resolve_op ENCRYPTION_KEY
resolve_op AUTH_DOMAIN

IMAGE="${GCP_REGION}-docker.pkg.dev/${GCP_PROJECT}/${REPO_NAME}/llm-proxy"
TAG="$(git -C "$PROJECT_ROOT" rev-parse --short HEAD)"

# Collect container env vars
SERVICE_ENVS="DATABASE_URL=${DATABASE_URL}"
SERVICE_ENVS+=",JWT_SECRET=${JWT_SECRET}"
SERVICE_ENVS+=",ENCRYPTION_KEY=${ENCRYPTION_KEY}"
SERVICE_ENVS+=",HOST=0.0.0.0"
[[ -n "${AUTH_DOMAIN:-}" ]] && SERVICE_ENVS+=",AUTH_DOMAIN=${AUTH_DOMAIN}"
[[ -n "${ANALYTICS_URL:-}" ]] && SERVICE_ENVS+=",ANALYTICS_URL=${ANALYTICS_URL}"
[[ -n "${UPSTREAM_TIMEOUT_SECS:-}" ]] && SERVICE_ENVS+=",UPSTREAM_TIMEOUT_SECS=${UPSTREAM_TIMEOUT_SECS}"
[[ -n "${RUST_LOG:-}" ]] && SERVICE_ENVS+=",RUST_LOG=${RUST_LOG}"

echo "==> Running database migrations"
DATABASE_URL="${DATABASE_URL}" llm-proxy-migrate 2>/dev/null || \
  echo "   (skipped — llm-proxy-migrate not found locally, migrations will run in container)"

echo "==> Building llm-proxy image (tag: $TAG, platform: linux/amd64)"
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
  --set-env-vars="$SERVICE_ENVS" \
  --add-cloudsql-instances="$CLOUDSQL_INSTANCE" \
  --execution-environment=gen2

echo "==> Done"
gcloud run services describe "$SERVICE_NAME" \
  --project="$GCP_PROJECT" \
  --region="$GCP_REGION" \
  --format='value(status.url)'

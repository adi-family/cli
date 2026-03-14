#!/usr/bin/env bash
# Deploy auth service to GCP Cloud Run
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
resolve_op SMTP_HOST
resolve_op SMTP_PORT
resolve_op SMTP_USERNAME
resolve_op SMTP_PASSWORD
resolve_op SMTP_FROM_EMAIL

IMAGE="${GCP_REGION}-docker.pkg.dev/${GCP_PROJECT}/${REPO_NAME}/auth"
TAG="$(git -C "$PROJECT_ROOT" rev-parse --short HEAD)"

# Collect container env vars
SERVICE_ENVS="DATABASE_URL=${DATABASE_URL}"
SERVICE_ENVS+=",JWT_SECRET=${JWT_SECRET}"
[[ -n "${JWT_EXPIRY_HOURS:-}" ]] && SERVICE_ENVS+=",JWT_EXPIRY_HOURS=${JWT_EXPIRY_HOURS}"
[[ -n "${SMTP_HOST:-}" ]] && SERVICE_ENVS+=",SMTP_HOST=${SMTP_HOST}"
[[ -n "${SMTP_PORT:-}" ]] && SERVICE_ENVS+=",SMTP_PORT=${SMTP_PORT}"
[[ -n "${SMTP_USERNAME:-}" ]] && SERVICE_ENVS+=",SMTP_USERNAME=${SMTP_USERNAME}"
[[ -n "${SMTP_PASSWORD:-}" ]] && SERVICE_ENVS+=",SMTP_PASSWORD=${SMTP_PASSWORD}"
[[ -n "${SMTP_FROM_EMAIL:-}" ]] && SERVICE_ENVS+=",SMTP_FROM_EMAIL=${SMTP_FROM_EMAIL}"
[[ -n "${SMTP_FROM_NAME:-}" ]] && SERVICE_ENVS+=",SMTP_FROM_NAME=${SMTP_FROM_NAME}"
[[ -n "${SMTP_TLS:-}" ]] && SERVICE_ENVS+=",SMTP_TLS=${SMTP_TLS}"
[[ -n "${RUST_LOG:-}" ]] && SERVICE_ENVS+=",RUST_LOG=${RUST_LOG}"

echo "==> Running database migrations"
DATABASE_URL="${DATABASE_URL}" auth-migrate 2>/dev/null || \
  echo "   (skipped — auth-migrate not found locally, migrations will run in container)"

echo "==> Building auth image (tag: $TAG, platform: linux/amd64)"
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

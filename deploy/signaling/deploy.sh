#!/usr/bin/env bash
# Deploy signaling server to GCP Cloud Run
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

resolve_op HMAC_SALT
resolve_op AUTH_DOMAIN

IMAGE="${GCP_REGION}-docker.pkg.dev/${GCP_PROJECT}/${REPO_NAME}/signaling"
TAG="$(git -C "$PROJECT_ROOT" rev-parse --short HEAD)"

# Collect container env vars
SERVICE_ENVS="HMAC_SALT=${HMAC_SALT}"
[[ -n "${AUTH_DOMAIN:-}" ]] && SERVICE_ENVS+=",AUTH_DOMAIN=${AUTH_DOMAIN}"
[[ -n "${ALLOW_MANUAL_REGISTRATION:-}" ]] && SERVICE_ENVS+=",ALLOW_MANUAL_REGISTRATION=${ALLOW_MANUAL_REGISTRATION}"
[[ -n "${WEBRTC_ICE_SERVERS:-}" ]] && SERVICE_ENVS+=",WEBRTC_ICE_SERVERS=${WEBRTC_ICE_SERVERS}"
[[ -n "${WEBRTC_TURN_USERNAME:-}" ]] && SERVICE_ENVS+=",WEBRTC_TURN_USERNAME=${WEBRTC_TURN_USERNAME}"
[[ -n "${WEBRTC_TURN_CREDENTIAL:-}" ]] && SERVICE_ENVS+=",WEBRTC_TURN_CREDENTIAL=${WEBRTC_TURN_CREDENTIAL}"
[[ -n "${RUST_LOG:-}" ]] && SERVICE_ENVS+=",RUST_LOG=${RUST_LOG}"

echo "==> Building signaling image (tag: $TAG, platform: linux/amd64)"
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
  --execution-environment=gen2

echo "==> Done"
gcloud run services describe "$SERVICE_NAME" \
  --project="$GCP_PROJECT" \
  --region="$GCP_REGION" \
  --format='value(status.url)'

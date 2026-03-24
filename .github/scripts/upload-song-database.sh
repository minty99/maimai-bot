#!/usr/bin/env bash

set -euo pipefail

: "${SONG_DATA_PATH:?SONG_DATA_PATH is required}"
: "${R2_BUCKET_NAME:?R2_BUCKET_NAME is required}"
: "${R2_ENDPOINT:?R2_ENDPOINT is required}"
: "${AWS_ACCESS_KEY_ID:?AWS_ACCESS_KEY_ID is required}"
: "${AWS_SECRET_ACCESS_KEY:?AWS_SECRET_ACCESS_KEY is required}"

if [[ ! -f "${SONG_DATA_PATH}/data.json" ]]; then
  echo "missing ${SONG_DATA_PATH}/data.json" >&2
  exit 1
fi

if [[ ! -d "${SONG_DATA_PATH}/cover" ]]; then
  echo "missing ${SONG_DATA_PATH}/cover" >&2
  exit 1
fi

aws s3 sync \
  "${SONG_DATA_PATH}/cover/" \
  "s3://${R2_BUCKET_NAME}/cover/" \
  --endpoint-url "${R2_ENDPOINT}" \
  --delete \
  --cache-control "public, max-age=31536000, immutable"

aws s3 cp \
  "${SONG_DATA_PATH}/data.json" \
  "s3://${R2_BUCKET_NAME}/data.json" \
  --endpoint-url "${R2_ENDPOINT}" \
  --cache-control "public, max-age=0, must-revalidate" \
  --content-type "application/json"

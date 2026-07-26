#!/usr/bin/env bash
set -euo pipefail

if (( $# == 0 )); then
  echo "Usage: $0 IMAGE [IMAGE ...]" >&2
  exit 2
fi

for image in "$@"; do
  if docker image inspect "$image" >/dev/null 2>&1; then
    echo "Docker image is already available: $image"
    continue
  fi

  for attempt in 1 2 3; do
    if docker pull "$image"; then
      break
    fi

    if (( attempt == 3 )); then
      echo "Unable to pull $image after $attempt attempts." >&2
      exit 1
    fi

    delay=$((attempt * 10))
    echo "Pulling $image failed; retrying in ${delay}s." >&2
    sleep "$delay"
  done
done

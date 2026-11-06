#!/usr/bin/env bash
# Preserve the failing command's result and output before the job-wide deadline.
set -euo pipefail

if (( $# < 3 )) || [[ ! "$1" =~ ^[a-zA-Z0-9_-]+$ ]] || [[ ! "$2" =~ ^[1-9][0-9]*$ ]]; then
  echo "Usage: $0 NAME TIMEOUT_SECONDS COMMAND [ARG ...]" >&2
  exit 2
fi
name=$1
budget=$2
shift 2
directory=${CITADEL_CI_LOG_DIRECTORY:-${RUNNER_TEMP:?RUNNER_TEMP must be set}/ci-tests}
mkdir -p "$directory"
echo "::group::$name (limit ${budget}s)"
{
  printf 'Command:'
  printf ' %q' "$@"
  printf '\n'
} | tee "$directory/$name.log"
started=$SECONDS
set +e
timeout --kill-after=15s "${budget}s" "$@" 2>&1 | tee -a "$directory/$name.log"
statuses=("${PIPESTATUS[@]}")
set -e
status=${statuses[0]}
if (( status == 0 )); then status=${statuses[1]}; fi
elapsed=$((SECONDS - started))
printf '%s\texit=%s\tseconds=%s\n' "$name" "$status" "$elapsed" >> "$directory/results.tsv"
echo "::endgroup::"
if (( status != 0 )); then
  echo "::error::$name failed with exit $status after ${elapsed}s (limit ${budget}s); see ci-tests artifact."
fi
if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
  printf -- '- `%s`: exit **%s**, %ss (limit %ss).\n' "$name" "$status" "$elapsed" "$budget" >> "$GITHUB_STEP_SUMMARY"
fi
exit "$status"

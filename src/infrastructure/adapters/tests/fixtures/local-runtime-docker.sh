#!/bin/sh
set -eu

# Keep $0 pointing at the fixture's docker symlink so each test owns its files.
. "$(dirname "$0")/script"

#!/usr/bin/env bash
# Push main to the optional "testing" remote (e.g. a second GitHub account or mirror).
# Does not push tags (no release).
#
# Before running:
#   1. Add the remote once:  git remote add testing git@github.com:OWNER/REPO.git
#   2. Authenticate as that account (SSH or HTTPS + credential).
#
# Usage:
#   ./scripts/push-testing-remote.sh
#   TESTING_REMOTE=myfork ./scripts/push-testing-remote.sh

set -euo pipefail

REMOTE="${TESTING_REMOTE:-testing}"
BRANCH="${TESTING_BRANCH:-main}"

if ! git remote get-url "$REMOTE" &>/dev/null; then
  echo "Remote '$REMOTE' is not configured. Example:" >&2
  echo "  git remote add testing git@github.com:OWNER/winusb-switcher-tauri-lite1.git" >&2
  exit 1
fi

echo "Remote: $REMOTE ($(git remote get-url "$REMOTE"))"
echo "Branch: $BRANCH"
echo "Pushing (no tags)..."

git push -u "$REMOTE" "$BRANCH"

echo "Done."

#!/usr/bin/env bash
# claim-branch.sh — claim/release a branch for agent isolation
# Usage:
#   ./scripts/claim-branch.sh <branch>            # claim
#   ./scripts/claim-branch.sh --release <branch>  # release
#   AGENT=a1 ./scripts/claim-branch.sh <branch>  # explicit agent id
set -euo pipefail
cd "$(dirname "$0")/.."

CLAIM_DIR=".opencode/claims"

sanitize() {
  # branch name -> filename: replace / with __
  echo "$1" | sed 's|/|__|g'
}

if [[ "${1:-}" == "--release" ]]; then
  BRANCH="${2:-}"
  if [[ -z "$BRANCH" ]]; then echo "Usage: $0 --release <branch>"; exit 1; fi
  FILE="$CLAIM_DIR/$(sanitize "$BRANCH")"
  if [[ -f "$FILE" ]]; then rm -f "$FILE"; echo "Released claim $BRANCH"; else echo "No claim for $BRANCH"; fi
  exit 0
fi

if [[ "${1:-}" == "--list" ]]; then
  ls -l "$CLAIM_DIR" 2>/dev/null || echo "No claims"
  cat "$CLAIM_DIR"/* 2>/dev/null || true
  exit 0
fi

BRANCH="${1:-}"
if [[ -z "$BRANCH" ]]; then
  echo "Usage: $0 <branch>  (e.g. a1/feat/kilo-health)"
  echo "       $0 --release <branch>"
  echo "       $0 --list"
  exit 1
fi

AGENT="${AGENT:-$(whoami)-$$}"
mkdir -p "$CLAIM_DIR"
FILE="$CLAIM_DIR/$(sanitize "$BRANCH")"

if [[ -f "$FILE" ]]; then
  echo "!! Branch $BRANCH already claimed:"
  cat "$FILE"
  echo "Pick a different <agent>/<type>/<kebab> name or --release if stale."
  exit 1
fi

# also check git branch exists (taken even without claim file)
if git show-ref --verify --quiet "refs/heads/$BRANCH" 2>/dev/null || git show-ref --verify --quiet "refs/remotes/origin/$BRANCH" 2>/dev/null; then
  echo "!! Branch $BRANCH already exists in git (local or origin). Pick another name."
  git branch -a | grep -F "$BRANCH" | head -n 5 || true
  # still allow claim if force? for now block unless --force
  if [[ "${2:-}" != "--force" ]]; then exit 1; fi
fi

cat > "$FILE" <<EOF
agent=$AGENT
pid=$$
branch=$BRANCH
timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)
worktree=$(pwd)
EOF

echo "Claimed $BRANCH for $AGENT -> $FILE"
# ensure gitignored
if ! grep -q "^\.opencode/claims/" .gitignore 2>/dev/null; then
  echo "Note: .opencode/claims/ should be gitignored (already in .gitignore)"
fi

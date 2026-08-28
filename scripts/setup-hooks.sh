#!/usr/bin/env bash
# setup-hooks.sh — install .githooks into .git/hooks
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ ! -d ".githooks" ]]; then
  echo "No .githooks directory found"
  exit 1
fi

mkdir -p .git/hooks
for hook in .githooks/*; do
  name=$(basename "$hook")
  target=".git/hooks/$name"
  cp "$hook" "$target"
  chmod +x "$target"
  echo "Installed $name -> $target"
done

# Ensure claims dir exists and is gitignored
mkdir -p .opencode/claims
if ! grep -q "^\.opencode/claims/" .gitignore 2>/dev/null; then
  echo ".opencode/claims/" >> .gitignore
  echo "Added .opencode/claims/ to .gitignore"
fi
# Also ignore web/dist stale check artifact not needed

echo "Hooks installed. Run './scripts/setup-hooks.sh' again after pulling new hooks."

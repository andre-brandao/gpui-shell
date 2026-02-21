#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"

if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>"
  echo "Example: $0 0.0.1"
  exit 1
fi

# Ensure version starts with 'v'
if [[ ! "$VERSION" =~ ^v ]]; then
  VERSION="v$VERSION"
fi

# Extract version number without 'v'
VERSION_NUM="${VERSION#v}"

echo "Bumping version to $VERSION..."

# Update version in Cargo.toml
sed -i "s/^version = \".*\"/version = \"$VERSION_NUM\"/" crates/app/Cargo.toml

# Update Cargo.lock
cargo update -p gpuishell

# Commit changes
git add crates/app/Cargo.toml Cargo.lock
git commit -m "chore: bump version to $VERSION"

# Create and push tag
git tag "$VERSION"

echo "Version bumped to $VERSION"
echo "To push: git push && git push origin $VERSION"

#!/usr/bin/env bash
set -euo pipefail

# Check if gum is installed
if ! command -v gum &> /dev/null; then
  echo "Error: gum is not installed. Please install it first."
  echo "See: https://github.com/charmbracelet/gum"
  exit 1
fi

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep '^version = ' crates/app/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

if [ -z "$CURRENT_VERSION" ]; then
  echo "Error: Could not extract current version from crates/app/Cargo.toml"
  exit 1
fi

echo "Current version: v$CURRENT_VERSION"
echo

# Parse current version (expecting semver: major.minor.patch)
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# Calculate next versions
NEXT_MAJOR="$((MAJOR + 1)).0.0"
NEXT_MINOR="$MAJOR.$((MINOR + 1)).0"
NEXT_PATCH="$MAJOR.$MINOR.$((PATCH + 1))"

# Let user choose version bump type
BUMP_TYPE=$(gum choose --header "Select version bump type:" "major (v$NEXT_MAJOR)" "minor (v$NEXT_MINOR)" "patch (v$NEXT_PATCH)" "custom")

case "$BUMP_TYPE" in
  major*)
    NEW_VERSION="$NEXT_MAJOR"
    ;;
  minor*)
    NEW_VERSION="$NEXT_MINOR"
    ;;
  patch*)
    NEW_VERSION="$NEXT_PATCH"
    ;;
  custom)
    NEW_VERSION=$(gum input --placeholder "Enter custom version (e.g., 1.2.3)")
    if [ -z "$NEW_VERSION" ]; then
      echo "Error: No version provided"
      exit 1
    fi
    # Strip 'v' prefix if provided
    NEW_VERSION="${NEW_VERSION#v}"
    ;;
esac

# Ensure version format is valid
if [[ ! "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: Invalid version format. Expected: major.minor.patch (e.g., 1.2.3)"
  exit 1
fi

VERSION_TAG="v$NEW_VERSION"

echo
gum style --border normal --padding "1 2" --border-foreground 212 \
  "Version bump: v$CURRENT_VERSION → $VERSION_TAG"
echo

# Confirm before proceeding
if ! gum confirm "Proceed with version bump and release?"; then
  echo "Release cancelled."
  exit 0
fi

echo
gum spin --spinner dot --title "Updating version in Cargo.toml..." -- sleep 0.5

# Update version in Cargo.toml
sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" crates/app/Cargo.toml

gum spin --spinner dot --title "Updating Cargo.lock..." -- cargo update -p gpuishell

# Stage changes
git add crates/app/Cargo.toml Cargo.lock

# Create commit
COMMIT_MSG="chore: bump version to $VERSION_TAG"
git commit -m "$COMMIT_MSG"

# Create tag
git tag "$VERSION_TAG"

echo
gum style --foreground 212 "✓ Version bumped to $VERSION_TAG"
gum style --foreground 212 "✓ Commit created: $COMMIT_MSG"
gum style --foreground 212 "✓ Tag created: $VERSION_TAG"
echo

# Confirm before pushing
if gum confirm "Push commit and tag to remote?"; then
  gum spin --spinner dot --title "Pushing to remote..." -- git push
  gum spin --spinner dot --title "Pushing tag..." -- git push origin "$VERSION_TAG"
  echo
  gum style --foreground 212 --bold "🚀 Release $VERSION_TAG pushed successfully!"
else
  echo
  gum style --foreground 220 "⚠ Changes committed locally but not pushed."
  gum style --foreground 220 "To push manually, run:"
  echo "  git push && git push origin $VERSION_TAG"
fi

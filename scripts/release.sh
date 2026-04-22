#!/usr/bin/env bash
set -euo pipefail

# Check required tools
for tool in gum gh jq; do
  if ! command -v "$tool" &> /dev/null; then
    echo "Error: $tool is not installed. Please install it first."
    exit 1
  fi
done

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
PREV_TAG="v$CURRENT_VERSION"
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
REPO_SLUG=$(gh repo view --json nameWithOwner -q .nameWithOwner)

# Build release notes from PRs merged into the current branch since the last tag
NOTES_FILE=$(mktemp -t release-notes.XXXXXX.md)
trap 'rm -f "$NOTES_FILE"' EXIT

gum spin --spinner dot --title "Gathering PRs merged since $PREV_TAG..." -- sleep 0.3

# Build the set of commit SHAs introduced since the previous tag. Ancestry is
# more reliable than merge dates (which can disagree with committer dates after
# rebases/cherry-picks).
if git rev-parse -q --verify "$PREV_TAG" >/dev/null; then
  RANGE_SHAS=$(git log --format=%H "$PREV_TAG..HEAD")
else
  RANGE_SHAS=$(git log --format=%H HEAD)
fi

PR_JSON=$(gh pr list \
  --base "$CURRENT_BRANCH" \
  --state merged \
  --limit 200 \
  --json number,title,author,mergedAt,url,mergeCommit)

# Keep PRs whose mergeCommit is in the range (i.e. not already shipped)
PR_LIST=$(echo "$PR_JSON" | jq --arg shas "$RANGE_SHAS" '
  ($shas | split("\n") | map(select(length > 0))) as $set
  | [.[] | select(.mergeCommit != null and (.mergeCommit.oid as $oid | $set | index($oid)))]
  | sort_by(.mergedAt)
')

PR_COUNT=$(echo "$PR_LIST" | jq 'length')

COMPARE_URL="https://github.com/$REPO_SLUG/compare/$PREV_TAG...$VERSION_TAG"

{
  echo "## What's Changed"
  echo
  if [ "$PR_COUNT" -eq 0 ]; then
    echo "_No pull requests merged since $PREV_TAG._"
  else
    echo "$PR_LIST" | jq -r '.[] | "- \(.title) by @\(.author.login) in [#\(.number)](\(.url))"'
  fi
  echo
  echo "**Full Changelog**: $COMPARE_URL"
} > "$NOTES_FILE"

echo
gum style --border normal --padding "1 2" --border-foreground 212 \
  "Version bump: v$CURRENT_VERSION → $VERSION_TAG" \
  "PRs merged into '$CURRENT_BRANCH' since $PREV_TAG: $PR_COUNT"
echo
gum style --foreground 212 --bold "Release notes preview:"
gum style --border rounded --padding "1 2" --border-foreground 240 "$(cat "$NOTES_FILE")"
echo

# Let user edit notes if desired
if gum confirm "Edit release notes before publishing?" --default=false; then
  EDITOR=${EDITOR:-vi} gum write --value "$(cat "$NOTES_FILE")" --width 100 --height 20 \
    --header "Edit release notes (Ctrl+D to save)" > "$NOTES_FILE.new"
  mv "$NOTES_FILE.new" "$NOTES_FILE"
  echo
  gum style --foreground 212 --bold "Updated notes:"
  gum style --border rounded --padding "1 2" --border-foreground 240 "$(cat "$NOTES_FILE")"
  echo
fi

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
if gum confirm "Push commit + tag and create GitHub release?"; then
  gum spin --spinner dot --title "Pushing commit..." -- git push
  gum spin --spinner dot --title "Pushing tag..." -- git push origin "$VERSION_TAG"
  gum spin --spinner dot --title "Creating GitHub release..." -- \
    gh release create "$VERSION_TAG" \
      --title "$VERSION_TAG" \
      --notes-file "$NOTES_FILE" \
      --target "$CURRENT_BRANCH"
  echo
  gum style --foreground 212 --bold "🚀 Release $VERSION_TAG published!"
  gum style --foreground 212 "   https://github.com/$REPO_SLUG/releases/tag/$VERSION_TAG"
  gum style --foreground 240 "   Build artifacts will be attached by the workflow once it completes."
else
  echo
  gum style --foreground 220 "⚠ Changes committed locally but not pushed."
  gum style --foreground 220 "To push and release manually, run:"
  echo "  git push && git push origin $VERSION_TAG"
  echo "  gh release create $VERSION_TAG --title $VERSION_TAG --notes-file $NOTES_FILE --target $CURRENT_BRANCH"
  # Preserve notes file if user wants to use it later
  KEPT_NOTES="${TMPDIR:-/tmp}/release-notes-$VERSION_TAG.md"
  cp "$NOTES_FILE" "$KEPT_NOTES"
  echo "  (notes saved to $KEPT_NOTES)"
fi

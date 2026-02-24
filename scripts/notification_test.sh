#!/usr/bin/env bash
set -euo pipefail

SLEEP_SECONDS=1.5
QUICK=0
IMAGE_PATH=""

usage() {
  cat <<'EOF'
Send a sequence of test notifications to org.freedesktop.Notifications.
Tests icon display, image handling, text wrapping, and layout consistency.

Usage:
  scripts/notification_test.sh [--quick] [--sleep SECONDS] [--image PATH]

Options:
  --quick           Minimal set of tests (faster)
  --sleep SECONDS   Delay between notifications (default: 1.5)
  --image PATH      Image path/URL for image-hint test
  -h, --help        Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --quick)
      QUICK=1
      shift
      ;;
    --sleep)
      SLEEP_SECONDS="${2:-}"
      shift 2
      ;;
    --image)
      IMAGE_PATH="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if ! command -v notify-send >/dev/null 2>&1; then
  echo "notify-send not found. Install libnotify (or use nix develop)." >&2
  exit 1
fi

sleep_between() {
  sleep "$SLEEP_SECONDS"
}

resolve_default_image() {
  if [[ -n "$IMAGE_PATH" ]]; then
    echo "$IMAGE_PATH"
    return
  fi

  # Try to find actual image files
  local candidates=(
    "$HOME/Pictures"/*.{png,jpg,jpeg}
    "/usr/share/icons/hicolor/128x128/apps/firefox.png"
    "/usr/share/icons/hicolor/128x128/apps/org.gnome.Nautilus.png"
    "/usr/share/pixmaps/firefox.png"
    "/usr/share/pixmaps/fedora-logo-icon.png"
  )

  local path
  for path in "${candidates[@]}"; do
    if [[ -f "$path" ]]; then
      echo "$path"
      return
    fi
  done
}

echo "=== Testing Notification Layout & Formatting ==="
echo ""

echo "[1/12] Basic notification (icon fallback test)"
notify-send "GPUiShell Test" "Basic notification with text fallback icon"
sleep_between

echo "[2/12] Named icon (should show 28px icon in 64px container)"
notify-send -i "org.gnome.Nautilus" "Files" "Testing named icon display"
sleep_between

echo "[3/12] Urgency: Low (should have muted color bar)"
notify-send -u low "Urgency low" "Background sync complete"
sleep_between

echo "[4/12] Urgency: Normal (should have accent color bar)"
notify-send -u normal "Urgency normal" "New message arrived"
sleep_between

echo "[5/12] Urgency: Critical (should have red/error color bar)"
notify-send -u critical "Urgency critical" "Battery critically low"
sleep_between

if [[ "$QUICK" -eq 0 ]]; then
  echo "[6/12] Long text wrapping test"
  notify-send "Very Long App Name That Should Truncate" \
    "This is a very long notification body designed to test text wrapping and layout behavior. The text should wrap naturally without breaking the card layout or causing overflow issues. This helps verify that the 64x64px icon area maintains consistent spacing."
  sleep_between

  echo "[7/12] Image only (64x64px image, no icon)"
  icon_path="$(resolve_default_image || true)"
  if [[ -n "${icon_path:-}" ]]; then
    notify-send -h "string:image-path:$icon_path" "Image only test" "Should show 64x64px image preview"
  else
    echo "No image found; skipping image-only test."
  fi
  sleep_between

  echo "[8/12] Icon + Image (64x64px image with 20px icon badge)"
  if [[ -n "${icon_path:-}" ]]; then
    notify-send -i "brave-browser" -h "string:image-path:$icon_path" "Icon + Image" "Should show image with small icon badge overlay"
  else
    echo "No image found; skipping icon+image test."
  fi
  sleep_between

  echo "[9/12] Direct icon path (absolute filesystem path)"
  if [[ -n "${icon_path:-}" ]]; then
    notify-send -a "IconPathApp" -i "$icon_path" "Direct icon path" "Using: ${icon_path##*/}"
  else
    echo "No icon file found; skipping."
  fi
  sleep_between

  echo "[10/12] URL-encoded path test (spaces in filename)"
  notify-send -i "file:///home/user/My%20Pictures/test%20image.png" "URL encoding" "Should decode %20 to spaces"
  sleep_between

  echo "[11/12] Actions buttons test"
  notify-send -a "ActionApp" -i "firefox" \
    -A "reply=Reply" -A "mark-read=Mark as Read" -A "open=Open" \
    "Action test" "Click an action button below"
  sleep_between

  echo "[12/12] Replace/update notification"
  replace_id="$(notify-send -p -a "UpdateApp" "Build status" "Starting build..." || true)"
  if [[ -n "${replace_id:-}" ]]; then
    sleep_between
    notify-send -r "$replace_id" -a "UpdateApp" "Build status" "Build at 42%"
    sleep_between
    notify-send -r "$replace_id" -a "UpdateApp" "Build status" "Build finished successfully ✓"
  else
    echo "notify-send -p not supported; skipping replace test."
  fi
fi

echo ""
echo "=== Test Complete ==="
echo "Check notification center to verify:"
echo "  • Consistent 64x64px icon/image area across all notifications"
echo "  • Text wrapping without overflow"
echo "  • Proper icon display (no missing icons)"
echo "  • Image previews showing correctly"

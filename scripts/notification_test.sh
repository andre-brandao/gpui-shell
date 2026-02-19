#!/usr/bin/env bash
set -euo pipefail

SLEEP_SECONDS=1.0
QUICK=0
IMAGE_PATH=""

usage() {
  cat <<'EOF'
Send a sequence of test notifications to org.freedesktop.Notifications.

Usage:
  scripts/notification_test.sh [--quick] [--sleep SECONDS] [--image PATH]

Options:
  --quick           Minimal set of tests (faster)
  --sleep SECONDS   Delay between notifications (default: 1.0)
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

  local candidates=(
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

echo "[1/7] basic notification"
notify-send "GPUiShell Test" "Basic notification"
sleep_between

echo "[2/7] replace/update notification (timer reset case)"
replace_id="$(notify-send -p -a "UpdateApp" "Build status" "Starting build..." || true)"
if [[ -n "${replace_id:-}" ]]; then
  sleep_between
  notify-send -r "$replace_id" -a "UpdateApp" "Build status" "Build at 42%"
  sleep_between
  notify-send -r "$replace_id" -a "UpdateApp" "Build status" "Build finished successfully"
else
  echo "notify-send -p did not return an id; skipping replace test details."
fi
sleep_between

echo "[3/7] urgency levels"
notify-send -u low "Urgency low" "Background sync complete"
sleep_between
notify-send -u normal "Urgency normal" "New message arrived"
sleep_between
notify-send -u critical "Urgency critical" "Battery critically low"
sleep_between

if [[ "$QUICK" -eq 0 ]]; then
  echo "[4/7] long body/description"
  notify-send "Long description" \
    "This is a long notification body to test text wrapping and card expansion behavior in both popup and notification center views."
  sleep_between

  echo "[5/7] timeout variants"
  notify-send -t 1500 "Short timeout" "Should disappear quickly from popup only"
  sleep_between
  notify-send -t 8000 "Long timeout" "Should stay longer in popup"
  sleep_between

  echo "[6/10] app icon path (if available)"
  icon_path="$(resolve_default_image || true)"
  if [[ -n "${icon_path:-}" ]]; then
    notify-send -a "IconPathApp" -i "$icon_path" "Icon path test" "Using icon: $icon_path"
  else
    echo "No default icon file found; skipping icon-path test."
  fi
  sleep_between

  echo "[7/10] image hint (if available)"
  if [[ -n "${icon_path:-}" ]]; then
    notify-send -h "string:image-path:$icon_path" "Image hint test" "Image should render in popup card"
  else
    echo "No image source found; skipping image-hint test."
  fi
  sleep_between

  echo "[8/10] named app icon (XDG lookup)"
  notify-send -a "Firefox" -i "firefox" "Named icon test" "Should show Firefox icon, not 'fi' text"
  sleep_between
  notify-send -a "Files" -i "org.gnome.Nautilus" "Named icon test 2" "Should show Nautilus icon via XDG lookup"
  sleep_between

  echo "[9/10] desktop-entry hint fallback"
  notify-send -h "string:desktop-entry:firefox" "Desktop entry hint" "Icon should resolve via desktop-entry hint"
  sleep_between

  echo "[10/10] actions"
  notify-send -a "ActionApp" -i "firefox" \
    -A "reply=Reply" -A "mark-read=Mark as Read" -A "open=Open" \
    "Action test" "Click an action button below" &
fi

echo "Done. Open notification center to verify history retention and dismiss behavior."

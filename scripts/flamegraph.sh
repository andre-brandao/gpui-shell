#!/usr/bin/env bash
set -euo pipefail

# CPU flamegraph, via perf + cargo-flamegraph. Heap profiling lives in
# scripts/heaptrack.sh.

cd "$(dirname "$0")/.."
# shellcheck source=lib/profiling.sh
source scripts/lib/profiling.sh

usage() {
  cat << 'EOF'
Record a CPU flamegraph of the shell. Ctrl-C stops the recording and renders.

  scripts/flamegraph.sh                     pick a target interactively
  scripts/flamegraph.sh --launch            build the profiling binary and profile it
  scripts/flamegraph.sh --pid $(pidof gpuishell)
  scripts/flamegraph.sh --launch -- --input ";s"

Options:
  --launch         profile a freshly launched shell
  --pid PID        attach to a running process
  --freq HZ        sampling frequency (default 997)
  --out FILE       output svg (default target/flamegraph/<timestamp>-cpu.svg)
  --no-inline      skip inline frame resolution: much faster, less detail
  --               everything after this is passed to gpuishell
EOF
}

TARGET=""
PID=""
FREQ="${FREQ:-997}"
OUT=""
EXTRA=()
APP_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --launch)
      TARGET="launch"
      shift
      ;;
    --pid)
      TARGET="pid"
      PID="$2"
      shift 2
      ;;
    --freq)
      FREQ="$2"
      shift 2
      ;;
    --out)
      OUT="$2"
      shift 2
      ;;
    --no-inline)
      EXTRA+=(--no-inline)
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    --)
      shift
      APP_ARGS=("$@")
      break
      ;;
    *) die "unknown option $1 (see --help)" ;;
  esac
done

require cargo flamegraph perf

# No target given: ask when there is a choice to make, otherwise just launch.
if [ -z "$TARGET" ]; then
  RUNNING=$(shell_pid)
  if [ -n "$RUNNING" ] && have_gum; then
    CHOICE=$(gum choose --header "gpuishell is running (pid $RUNNING). Profile:" \
      "the running shell" "a fresh one") || die "cancelled"
    case "$CHOICE" in
      "the running shell")
        TARGET="pid"
        PID="$RUNNING"
        ;;
      *) TARGET="launch" ;;
    esac
  elif [ -n "$RUNNING" ]; then
    TARGET="pid"
    PID="$RUNNING"
  else
    TARGET="launch"
  fi
fi

# perf needs paranoid <= 1 to sample an unprivileged process.
PARANOID=$(cat /proc/sys/kernel/perf_event_paranoid 2> /dev/null || echo 2)
if [ "$PARANOID" -gt 1 ] && [ "$EUID" -ne 0 ]; then
  SYSCTL="sudo sysctl -w kernel.perf_event_paranoid=1"
  if have_gum && gum confirm "perf needs kernel.perf_event_paranoid <= 1 (it is $PARANOID). Run '$SYSCTL'?"; then
    $SYSCTL > /dev/null
  else
    die "kernel.perf_event_paranoid is $PARANOID; run '$SYSCTL' (resets on reboot), or use scripts/heaptrack.sh which needs no privileges"
  fi
fi

OUT="${OUT:-$OUT_DIR/$(stamp)-cpu.svg}"
mkdir -p "$(dirname "$OUT")"

if [ "$TARGET" = "pid" ]; then
  echo "Profiling pid $PID at ${FREQ}Hz - Ctrl-C to stop and render."
  flamegraph --pid "$PID" -F "$FREQ" -o "$OUT" "${EXTRA[@]}"
else
  ensure_free_to_launch
  echo "Sampling at ${FREQ}Hz - Ctrl-C to stop and render."
  CMD=(cargo flamegraph --profile "$PROFILE" --bin gpuishell -F "$FREQ" -o "$OUT" "${EXTRA[@]}")
  if [ ${#APP_ARGS[@]} -gt 0 ]; then
    CMD+=(-- "${APP_ARGS[@]}")
  fi
  "${CMD[@]}"
fi

echo
echo "Wrote $OUT (open in a browser to zoom and search; raw samples in ./perf.data)"

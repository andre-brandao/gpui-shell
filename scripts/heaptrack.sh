#!/usr/bin/env bash
set -euo pipefail

# Heap profiling, via heaptrack. Needs no perf privileges. CPU flamegraphs live
# in scripts/flamegraph.sh.

cd "$(dirname "$0")/.."
# shellcheck source=lib/profiling.sh
source scripts/lib/profiling.sh

usage() {
  cat << 'EOF'
Record heap allocations of the shell and render a flamegraph. Ctrl-C stops the
recording.

  scripts/heaptrack.sh                      pick what to do interactively
  scripts/heaptrack.sh --record             build the profiling binary and record it
  scripts/heaptrack.sh --pid $(pidof gpuishell)
  scripts/heaptrack.sh --analyze FILE.zst --cost leaked
  scripts/heaptrack.sh --record -- --input ";s"

Options:
  --record         record a freshly launched shell
  --pid PID        inject into a running shell (heaptrack calls this unstable:
                   it can crash the target, especially on detach)
  --analyze FILE   render an existing recording; FILE defaults to the newest one
  --cost COST      peak (default) | leaked | allocations | temporary
  --gui            open heaptrack_gui instead of rendering an svg
  --out FILE       output svg (default target/flamegraph/<timestamp>-<cost>.svg)
  --               everything after this is passed to gpuishell
EOF
}

MODE=""
PID=""
DATA=""
COST="${COST:-}"
OUT=""
GUI=0
APP_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --record)
      MODE="record"
      shift
      ;;
    --pid)
      MODE="attach"
      PID="$2"
      shift 2
      ;;
    --analyze)
      MODE="analyze"
      # The file is optional: `--analyze --cost leaked` takes the newest one.
      if [[ ${2-} && ${2-} != -* ]]; then
        DATA="$2"
        shift
      fi
      shift
      ;;
    --cost)
      COST="$2"
      shift 2
      ;;
    --gui)
      GUI=1
      shift
      ;;
    --out)
      OUT="$2"
      shift 2
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

require cargo heaptrack heaptrack_print
if [ "$GUI" = 1 ]; then
  require heaptrack_gui
else
  require inferno-flamegraph
fi

latest_recording() { ls -t "$OUT_DIR"/*-heaptrack.zst 2> /dev/null | head -1; }

if [ -z "$MODE" ]; then
  RUNNING=$(shell_pid)
  have_gum || die "pass --record, --pid PID or --analyze (see --help)"
  CHOICE=$(gum choose --header "Heap profile:" \
    "record a fresh shell" \
    "inject into the running shell${RUNNING:+ (pid $RUNNING)}" \
    "analyze the last recording") || die "cancelled"
  case "$CHOICE" in
    "record a fresh shell") MODE="record" ;;
    inject*)
      [ -n "$RUNNING" ] || die "no gpuishell process to inject into"
      MODE="attach"
      PID="$RUNNING"
      ;;
    *) MODE="analyze" ;;
  esac
fi

case "$MODE" in
  record)
    ensure_free_to_launch
    build_profiling
    PREFIX="$OUT_DIR/$(stamp)-heaptrack"
    mkdir -p "$OUT_DIR"
    echo "Recording allocations - Ctrl-C to stop."
    # --record-only, or heaptrack opens heaptrack_gui on exit and blocks here.
    heaptrack --record-only -o "$PREFIX" "$BIN" "${APP_ARGS[@]}"
    DATA=$(ls -t "$PREFIX"* | head -1)
    ;;
  attach)
    PREFIX="$OUT_DIR/$(stamp)-heaptrack"
    mkdir -p "$OUT_DIR"
    echo "Injecting into pid $PID - Ctrl-C to stop (this can crash the shell)."
    heaptrack --record-only -o "$PREFIX" -p "$PID"
    DATA=$(ls -t "$PREFIX"* | head -1)
    ;;
  analyze)
    [ -n "$DATA" ] || DATA=$(latest_recording)
    [ -n "$DATA" ] || die "no recording found under $OUT_DIR"
    ;;
esac

[ -f "$DATA" ] || die "no heaptrack data file at ${DATA:-$OUT_DIR}"
echo "Recording: $DATA"

if [ "$GUI" = 1 ]; then
  exec heaptrack_gui "$DATA"
fi

if [ -z "$COST" ]; then
  if have_gum; then
    COST=$(gum choose --header "Cost to plot:" peak leaked allocations temporary) || die "cancelled"
  else
    COST="peak"
  fi
fi

case "$COST" in
  peak | leaked) COUNTNAME="bytes" ;;
  allocations | temporary) COUNTNAME="$COST" ;;
  *) die "unknown cost type '$COST' (peak | leaked | allocations | temporary)" ;;
esac

OUT="${OUT:-${DATA%.zst}-$COST.svg}"
FOLDED="${DATA%.zst}-$COST.folded"

echo "Resolving symbols for '$COST' (this takes a while on a big recording)..."
heaptrack_print --flamegraph-cost-type "$COST" --print-flamegraph "$FOLDED" "$DATA" > /dev/null
inferno-flamegraph --colors mem --countname "$COUNTNAME" \
  --title "gpuishell heap: $COST" "$FOLDED" > "$OUT"

echo
echo "Wrote $OUT"
echo "Full report:  heaptrack_print $DATA | less"
echo "Interactive:  scripts/heaptrack.sh --analyze $DATA --gui"

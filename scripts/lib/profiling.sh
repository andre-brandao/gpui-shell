# Shared helpers for scripts/flamegraph.sh (CPU) and scripts/heaptrack.sh (heap).
# Sourced, not executed. Callers cd to the repo root first.
# shellcheck shell=bash

PROFILE="${PROFILE:-profiling}"
OUT_DIR="target/flamegraph"

die() {
  echo "Error: $*" >&2
  exit 1
}

require() {
  for tool in "$@"; do
    command -v "$tool" &> /dev/null || die "$tool not found - enter the dev shell: nix develop"
  done
}

have_gum() { command -v gum &> /dev/null; }

# pid of the running shell, empty if there is none
shell_pid() { pgrep -x gpuishell | head -1; }

stamp() { date +%Y%m%d-%H%M%S; }

build_profiling() {
  echo "Building the $PROFILE profile (release codegen + debug symbols, first build is slow)..."
  cargo build --profile "$PROFILE" --bin gpuishell
  BIN="target/$PROFILE/gpuishell"
  [ -x "$BIN" ] || die "no binary at $BIN"
}

# The shell is single-instance: a second process signals the first and exits, so
# launching one under a profiler while another runs records nothing.
ensure_free_to_launch() {
  local pid
  pid=$(shell_pid)
  if [ -z "$pid" ]; then
    return 0
  fi

  if have_gum && gum confirm "gpuishell is running (pid $pid). Stop it and profile a fresh one?"; then
    kill "$pid"
    # Give it a moment to drop its socket.
    for _ in {1..20}; do
      [ -z "$(shell_pid)" ] && return 0
      sleep 0.2
    done
    die "pid $pid did not exit"
  fi

  die "gpuishell is already running (pid $pid) - attach with --pid $pid, or stop it first"
}

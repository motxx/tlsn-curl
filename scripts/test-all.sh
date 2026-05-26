#!/usr/bin/env bash
# Unified quality harness for tlsn-curl.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

MODE="${1:---local}"
FAILED=0
DOCKER_STARTED=0

if [ -t 1 ]; then
  GREEN='\033[0;32m'; RED='\033[0;31m'; BOLD='\033[1m'; NC='\033[0m'
else
  GREEN=''; RED=''; BOLD=''; NC=''
fi

step() { printf '\n%s=== %s ===%s\n' "$BOLD" "$1" "$NC"; }
pass() { printf '  %sPASS%s %s\n' "$GREEN" "$NC" "$1"; }
fail() { printf '  %sFAIL%s %s\n' "$RED" "$NC" "$1"; FAILED=1; }

run_check() {
  local name="$1"; shift
  if "$@" 2>&1; then
    pass "$name"
  else
    fail "$name"
  fi
}

cleanup() {
  if [ "$DOCKER_STARTED" = "1" ]; then
    step "Docker teardown"
    docker compose down -v --remove-orphans --timeout 10 2>/dev/null || true
  fi
}

run_local() {
  step "Local quality gate"
  run_check "local path lint" ./scripts/check-no-local-paths.sh
  run_check "README command lint" ./scripts/check-docs.sh
  run_check "rust fmt root" cargo fmt --manifest-path Cargo.toml -- --check
  run_check "rust fmt server" cargo fmt --manifest-path crates/tlsn-server/Cargo.toml -- --check
  run_check "cargo test root" cargo test --manifest-path Cargo.toml
  run_check "cargo check server" cargo check --manifest-path crates/tlsn-server/Cargo.toml
  run_check "cli binary smoke" ./scripts/smoke-built-cli.sh
}

run_docker() {
  step "Docker quality gate"
  export TLSN_CURL_DOCKER_ISOLATION="${TLSN_CURL_DOCKER_ISOLATION:-worktree}"
  # shellcheck source=/dev/null
  source "$SCRIPT_DIR/docker-compose-env.sh"

  if [ ! -f docker-compose.yml ] && [ ! -f compose.yml ]; then
    echo "Docker Compose stack is not implemented yet; see docs/issues/pending/0005-add-local-docker-compose-stack.md"
    return 0
  fi

  run_check "tlsn build" cargo build --release --manifest-path Cargo.toml
  if [ "$FAILED" != "0" ]; then
    return 0
  fi

  DOCKER_STARTED=1
  docker compose up -d
  if ./scripts/e2e-tlsn.sh --use-existing-compose; then
    pass "compose TLSN e2e"
  else
    mkdir -p ci-artifacts
    docker compose ps > ci-artifacts/docker-compose-ps.txt 2>&1 || true
    docker compose logs > ci-artifacts/docker-compose.log 2>&1 || true
    cat ci-artifacts/docker-compose.log || true
    fail "compose TLSN e2e"
  fi
}

case "$MODE" in
  --local)
    run_local
    ;;
  --docker)
    trap cleanup EXIT
    run_docker
    ;;
  --ci|full|--full)
    trap cleanup EXIT
    run_local
    if [ "$FAILED" = "0" ]; then
      run_docker
    fi
    ;;
  *)
    echo "Usage: $0 [--local|--docker|--ci|--full]" >&2
    exit 2
    ;;
esac

echo ""
if [ "$FAILED" = "0" ]; then
  printf '%s%s%s\n' "$GREEN" "All requested checks passed." "$NC"
  exit 0
fi

printf '%s%s%s\n' "$RED" "Some checks failed." "$NC"
exit 1

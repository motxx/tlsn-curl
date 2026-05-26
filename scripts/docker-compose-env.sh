#!/usr/bin/env bash
# Shared Docker Compose environment for local TLSNotary harnesses.
#
# Source this before invoking docker compose. By default it keeps plain
# `docker compose up` predictable. Set TLSN_CURL_DOCKER_ISOLATION=worktree
# in test runners to derive an isolated project name and host port block from
# the current worktree path.

if [ -n "${TLSN_CURL_DOCKER_COMPOSE_ENV_LOADED:-}" ]; then
  return 0 2>/dev/null || exit 0
fi
TLSN_CURL_DOCKER_COMPOSE_ENV_LOADED=1

tlsn_curl_compose_sanitize() {
  printf '%s' "$1" \
    | tr '[:upper:]' '[:lower:]' \
    | sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//'
}

if [ "${TLSN_CURL_DOCKER_ISOLATION:-}" = "worktree" ]; then
  TLSN_CURL_WORKTREE_ROOT="${TLSN_CURL_WORKTREE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
  TLSN_CURL_WORKTREE_NAME="$(tlsn_curl_compose_sanitize "$(basename "$TLSN_CURL_WORKTREE_ROOT")")"
  TLSN_CURL_WORKTREE_NAME="${TLSN_CURL_WORKTREE_NAME:-worktree}"
  TLSN_CURL_WORKTREE_NAME="${TLSN_CURL_WORKTREE_NAME:0:24}"

  TLSN_CURL_WORKTREE_HASH="$(printf '%s' "$TLSN_CURL_WORKTREE_ROOT" | cksum | awk '{print $1}')"
  TLSN_CURL_WORKTREE_HASH_SHORT="$((TLSN_CURL_WORKTREE_HASH % 100000))"
  TLSN_CURL_WORKTREE_PORT_OFFSET="$((TLSN_CURL_WORKTREE_HASH % 1000))"

  : "${TLSN_CURL_COMPOSE_PROJECT_NAME:=${COMPOSE_PROJECT_NAME:-tlsn-curl-${TLSN_CURL_WORKTREE_NAME}-${TLSN_CURL_WORKTREE_HASH_SHORT}}}"
  : "${TLSN_CURL_TLSN_TCP_PORT:=$((22000 + TLSN_CURL_WORKTREE_PORT_OFFSET))}"
  : "${TLSN_CURL_TLSN_WS_PORT:=$((23000 + TLSN_CURL_WORKTREE_PORT_OFFSET))}"
  : "${TLSN_CURL_HTTPS_TARGET_PORT:=$((24000 + TLSN_CURL_WORKTREE_PORT_OFFSET))}"
else
  : "${TLSN_CURL_COMPOSE_PROJECT_NAME:=${COMPOSE_PROJECT_NAME:-tlsn-curl}}"
  : "${TLSN_CURL_TLSN_TCP_PORT:=7046}"
  : "${TLSN_CURL_TLSN_WS_PORT:=7047}"
  : "${TLSN_CURL_HTTPS_TARGET_PORT:=9443}"
fi

export TLSN_CURL_COMPOSE_PROJECT_NAME
export COMPOSE_PROJECT_NAME="$TLSN_CURL_COMPOSE_PROJECT_NAME"
export TLSN_CURL_TLSN_TCP_PORT
export TLSN_CURL_TLSN_WS_PORT
export TLSN_CURL_HTTPS_TARGET_PORT

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  printf 'COMPOSE_PROJECT_NAME=%s\n' "$COMPOSE_PROJECT_NAME"
  printf 'TLSN_CURL_DOCKER_ISOLATION=%s\n' "${TLSN_CURL_DOCKER_ISOLATION:-shared}"
  printf 'TLSN_CURL_TLSN_TCP_PORT=%s\n' "$TLSN_CURL_TLSN_TCP_PORT"
  printf 'TLSN_CURL_TLSN_WS_PORT=%s\n' "$TLSN_CURL_TLSN_WS_PORT"
  printf 'TLSN_CURL_HTTPS_TARGET_PORT=%s\n' "$TLSN_CURL_HTTPS_TARGET_PORT"
fi

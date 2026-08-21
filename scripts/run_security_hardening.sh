#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASE_VM="${NEMESIS_TART_BASE_VM:-anubis-xcode}"
GUEST_NAME="${NEMESIS_TART_GUEST_NAME:-nemesis-security-${USER:-operator}-$$}"
SSH_KEY="${NEMESIS_TART_SSH_KEY:-$HOME/.ssh/tart_anubis}"
VM_LOG="${TMPDIR:-/tmp}/${GUEST_NAME}.log"
VM_PID=""

vm_exists() {
  local name
  while IFS= read -r name; do
    if [[ "$name" == "$GUEST_NAME" ]]; then
      return 0
    fi
  done < <(tart list --source local --quiet)
  return 1
}

cleanup() {
  local status=$?
  trap - EXIT INT TERM
  set +e
  if vm_exists; then
    tart stop "$GUEST_NAME" >/dev/null 2>&1
  fi
  if [[ -n "$VM_PID" ]]; then
    wait "$VM_PID" >/dev/null 2>&1
  fi
  if vm_exists; then
    tart delete "$GUEST_NAME" >/dev/null 2>&1
  fi
  if vm_exists; then
    printf '%s\n' "FATAL_SECURITY_GUEST_ORPHANED name=$GUEST_NAME log=$VM_LOG" >&2
    if [[ "$status" -eq 0 ]]; then
      status=1
    fi
  else
    rm -f "$VM_LOG"
  fi
  exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

if [[ ! "$GUEST_NAME" =~ ^[A-Za-z0-9._-]+$ ]]; then
  printf '%s\n' "REFUSED_SECURITY_HARDENING_INVALID_GUEST_NAME name=$GUEST_NAME" >&2
  exit 2
fi
if [[ "$(uname -s)" != "Darwin" ]]; then
  printf '%s\n' 'REFUSED_SECURITY_HARDENING_REQUIRES_MACOS_VZ_HOST' >&2
  exit 2
fi
for executable in tart ssh rsync; do
  if ! command -v "$executable" >/dev/null 2>&1; then
    printf '%s\n' "REFUSED_SECURITY_HARDENING_MISSING_TOOL tool=$executable" >&2
    exit 2
  fi
done
if [[ ! -r "$SSH_KEY" ]]; then
  printf '%s\n' "REFUSED_SECURITY_HARDENING_MISSING_SSH_KEY path=$SSH_KEY" >&2
  exit 2
fi
if vm_exists; then
  printf '%s\n' "REFUSED_SECURITY_HARDENING_GUEST_EXISTS name=$GUEST_NAME" >&2
  exit 2
fi

cd "$ROOT"
tart clone "$BASE_VM" "$GUEST_NAME"
tart run "$GUEST_NAME" --no-graphics --no-audio --no-clipboard >"$VM_LOG" 2>&1 &
VM_PID=$!
GUEST_IP="$(tart ip "$GUEST_NAME" --wait 60)"
SSH=(ssh -i "$SSH_KEY" -o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null)
printf -v RSYNC_SSH 'ssh -i %q -o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null' "$SSH_KEY"

SSH_READY=false
for _ in {1..30}; do
  if "${SSH[@]}" "admin@$GUEST_IP" "printf '%s\\n' READY" >/dev/null 2>&1; then
    SSH_READY=true
    break
  fi
  sleep 1
done
if [[ "$SSH_READY" != "true" ]]; then
  printf '%s\n' "REFUSED_SECURITY_HARDENING_GUEST_SSH_UNAVAILABLE name=$GUEST_NAME" >&2
  exit 2
fi
rsync -a --delete \
  --exclude .git \
  --exclude .worktrees \
  --exclude build \
  --exclude runtime/target \
  --exclude desktop/node_modules \
  --exclude desktop/dist \
  --exclude desktop/src-tauri/target \
  --exclude dist \
  -e "$RSYNC_SSH" \
  ./ "admin@$GUEST_IP:~/nemesis-security/"

"${SSH[@]}" "admin@$GUEST_IP" \
  'cd ~/nemesis-security && NEMESIS_VZ_GUEST=1 ./scripts/security_hardening_guest.sh'
printf '%s\n' "PASS_PHASE16_VZ_SECURITY_HARDENING_ORCHESTRATOR base=$BASE_VM"

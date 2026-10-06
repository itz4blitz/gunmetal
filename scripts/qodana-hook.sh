#!/usr/bin/env bash
# Stop hook for Claude Code, Cursor, Codex, and Grok. When this turn changed
# Rust, Cargo, Clippy, or Qodana files, Qodana has to pass before the agent
# ends.
#
# Claude Code and Codex print the reason on stderr and exit 2. Cursor prints
# {followup_message} and exits 0. Grok prints {"decision":"block","reason"}
# and exits 0. Grok also loads the Claude and Cursor hook files. The lock
# makes a second fire wait and reuse this result instead of starting another
# scan.
set -euo pipefail

input="$(cat || true)"
event="$(printf '%s' "${input}" | jq -r '.hook_event_name // .hookEventName // empty' 2>/dev/null || true)"
status="$(printf '%s' "${input}" | jq -r '.status // empty' 2>/dev/null || true)"
stop_reason="$(printf '%s' "${input}" | jq -r '.reason // empty' 2>/dev/null || true)"

if [[ "${status}" == "aborted" || "${status}" == "error" ]]; then
  exit 0
fi

# A session-end Stop is observe-only. Gate a turn that is actually ending.
if [[ -n "${GROK_HOOK_EVENT:-}" && -n "${stop_reason}" && "${stop_reason}" != "end_turn" ]]; then
  exit 0
fi

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${root}"

changed="$(git status --porcelain -uall)"
if ! printf '%s\n' "${changed}" | awk '{ print $NF }' | grep -Eq '(^|/)([^/]+\.rs|Cargo\.toml|Cargo\.lock|clippy\.toml|qodana\.yaml|qodana\.sarif\.json|rust-toolchain\.toml)$'; then
  exit 0
fi

key="$(printf '%s' "${root}" | sha256sum | awk '{ print $1 }')"
lock_path="${TMPDIR:-/tmp}/qodana-hook-${key}.lock"
state_path="${TMPDIR:-/tmp}/qodana-hook-${key}.result"
exec 9>"${lock_path}"
waited=0
if ! flock -n 9; then
  waited=1
  flock 9
fi

report() {
  local code="$1"
  local log="$2"
  local tail_text="$3"
  if [[ "${code}" -eq 0 ]]; then
    exit 0
  fi
  local reason
  reason="$(printf 'Qodana for Rust failed (exit %s). Fix the new problems and finish again.\nLog: %s\n\n%s\n' "${code}" "${log}" "${tail_text}")"
  if [[ -n "${GROK_HOOK_EVENT:-}" ]]; then
    jq -n --arg reason "${reason}" '{decision: "block", reason: $reason}'
    exit 0
  fi
  case "${event}" in
    stop | subagentStop)
      jq -n --arg message "${reason}" '{followup_message: $message}'
      exit 0
      ;;
    *)
      printf '%s\n' "${reason}" >&2
      exit 2
      ;;
  esac
}

if [[ "${waited}" -eq 1 && -f "${state_path}" ]]; then
  if code="$(jq -er '.code' "${state_path}" 2>/dev/null)"; then
    log="$(jq -r '.log // empty' "${state_path}")"
    tail_text="$(jq -r '.tail // empty' "${state_path}")"
    report "${code}" "${log}" "${tail_text}"
  fi
fi

log="$(mktemp)"
set +e
"${root}/scripts/qodana.sh" >"${log}" 2>&1
code=$?
set -e
tail_text=""
if [[ "${code}" -ne 0 ]]; then
  tail_text="$(tail -n 50 "${log}" || true)"
fi
jq -n --argjson code "${code}" --arg log "${log}" --arg tail "${tail_text}" \
  '{code: $code, log: $log, tail: $tail}' >"${state_path}"
if [[ "${code}" -eq 0 ]]; then
  rm -f "${log}"
fi
report "${code}" "${log}" "${tail_text}"

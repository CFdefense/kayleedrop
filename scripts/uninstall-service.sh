#!/usr/bin/env bash
#
# Remove kayleedrop scheduling and binary installed by install-service.sh.
#
# - **macOS (user)**: no sudo — unloads Login LaunchAgent, removes plist, removes binary.
# - **macOS (daemon)**: sudo — unloads LaunchDaemon, removes plist, binary, /etc env, optional data root.
# - **Linux**: sudo — disables timer, removes systemd units, binary, optional data root.
#
# By default **`INSTALL_ROOT` is kept** so secrets (`~…/KayleeDrop/.env`) and `data/` survive.
#
# Usage:
#
#     bash scripts/uninstall-service.sh
#     bash scripts/uninstall-service.sh --purge       # also rm -rf INSTALL_ROOT (secrets + payloads)
#
# Mirrors **`BIN_NAME`**, **`LAUNCH_LABEL`**, **`INSTALL_ROOT`**, etc. — set the same overrides you used when installing.

set -euo pipefail

BIN_NAME="${BIN_NAME:-kayleedrop}"
LAUNCH_LABEL="${LAUNCH_LABEL:-io.github.cfdefense.kayleedrop}"

TIMER_UNIT="${TIMER_UNIT:-${BIN_NAME}-daily.timer}"
SERVICE_UNIT="${SERVICE_UNIT:-${BIN_NAME}-daily.service}"

OS="$(uname -s)"
SERVICE_USER=""
INSTALL_ROOT=""
BIN_DEST=""
ENV_FILE=""
PURGE=0

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

usage() {
  cat <<EOF
Usage: uninstall-service.sh [--purge]

  --purge   Remove INSTALL_ROOT (Application Support KayleeDrop dir, /opt/kaylee…, decrypted data, .env).

Environment (match install overrides):
  BIN_NAME  LAUNCH_LABEL  INSTALL_ROOT  BIN_DEST  ENV_FILE
  TIMER_UNIT  SERVICE_UNIT  (Linux)
EOF
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
  --purge) PURGE=1 ;;
  -h | --help) usage; exit 0 ;;
  *) die "unknown option: $1 (try --help)" ;;
  esac
  shift
done

detect_paths_linux() {
  [[ "${EUID:-0}" -eq 0 ]] || die "Linux: rerun with sudo"

  INSTALL_ROOT="${INSTALL_ROOT:-/opt/${BIN_NAME}}"
  BIN_DEST="${BIN_DEST:-/usr/local/bin/${BIN_NAME}}"
  ENV_FILE="${ENV_FILE:-/etc/${BIN_NAME}.env}"
  SERVICE_USER="${SERVICE_USER:-${SUDO_USER:-root}}"
}

detect_paths_darwin_root() {
  [[ "${EUID:-0}" -eq 0 ]] || die "macOS daemon uninstall: run with sudo"

  INSTALL_ROOT="${INSTALL_ROOT:-/opt/${BIN_NAME}}"
  BIN_DEST="${BIN_DEST:-/usr/local/bin/${BIN_NAME}}"
  ENV_FILE="${ENV_FILE:-/etc/${BIN_NAME}.env}"
  SERVICE_USER="${SERVICE_USER:-${SUDO_USER:-$(logname 2>/dev/null || true)}}"
}

detect_paths_darwin_user() {
  [[ "${EUID:-0}" -ne 0 ]] || die "macOS Login agent uninstall runs as your user, not sudo (use sudo only for daemon installs)"

  INSTALL_ROOT="${INSTALL_ROOT:-${HOME}/Library/Application Support/KayleeDrop}"
  BIN_DEST="${BIN_DEST:-${INSTALL_ROOT}/bin/${BIN_NAME}}"
  ENV_FILE="${ENV_FILE:-${INSTALL_ROOT}/.env}"
}

uninstall_linux() {
  detect_paths_linux

  if systemctl list-unit-files --type=timer "${TIMER_UNIT}" &>/dev/null || systemctl cat "${TIMER_UNIT}" &>/dev/null; then
    systemctl disable --now "${TIMER_UNIT}" 2>/dev/null || true
  fi
  rm -f "/etc/systemd/system/${TIMER_UNIT}" "/etc/systemd/system/${SERVICE_UNIT}"
  systemctl daemon-reload 2>/dev/null || true

  [[ ! -f "${BIN_DEST}" ]] || rm -f "${BIN_DEST}"
  [[ ! -f "${ENV_FILE}" ]] || rm -f "${ENV_FILE}"

  if [[ "${PURGE}" -eq 1 ]]; then
    rm -rf "${INSTALL_ROOT}"
    printf '%s removed INSTALL_ROOT %s\n' "$(basename "$0")" "${INSTALL_ROOT}" >&2
  else
    printf '%s Linux: unloaded %s — left data at %s (use --purge to delete)\n' "$(basename "$0")" "${TIMER_UNIT}" "${INSTALL_ROOT}" >&2
  fi
}

uninstall_darwin_daemon() {
  detect_paths_darwin_root
  local plist="/Library/LaunchDaemons/${LAUNCH_LABEL}.plist"

  launchctl bootout system "${plist}" 2>/dev/null || true
  [[ ! -f "${plist}" ]] || rm -f "${plist}"

  [[ ! -f "${BIN_DEST}" ]] || rm -f "${BIN_DEST}"
  [[ ! -f "${ENV_FILE}" ]] || rm -f "${ENV_FILE}"

  if [[ "${PURGE}" -eq 1 ]]; then
    rm -rf "${INSTALL_ROOT}"
    printf '%s removed INSTALL_ROOT %s\n' "$(basename "$0")" "${INSTALL_ROOT}" >&2
  else
    printf '%s daemon: unloaded %s — left data at %s\n' "$(basename "$0")" "${LAUNCH_LABEL}" "${INSTALL_ROOT}" >&2
  fi
}

uninstall_darwin_agent() {
  detect_paths_darwin_user
  local plist_dest="${HOME}/Library/LaunchAgents/${LAUNCH_LABEL}.plist"
  local gui="gui/$(id -u)"

  launchctl bootout "${gui}" "${plist_dest}" 2>/dev/null || true
  [[ ! -f "${plist_dest}" ]] || rm -f "${plist_dest}"

  [[ ! -f "${BIN_DEST}" ]] || rm -f "${BIN_DEST}"

  rm -f /tmp/kayleedrop.out /tmp/kayleedrop.err 2>/dev/null || true

  legacy_env="${INSTALL_ROOT}/env"
  if [[ "${PURGE}" -eq 1 ]]; then
    rm -rf "${INSTALL_ROOT}"
    printf '%s removed INSTALL_ROOT %s\n' "$(basename "$0")" "${INSTALL_ROOT}" >&2
  else
    printf '%s Login agent: unloaded %s — left %s (and %s if present); use --purge to delete INSTALL_ROOT\n' \
      "$(basename "$0")" "${LAUNCH_LABEL}" "${INSTALL_ROOT}" "${ENV_FILE}" >&2
    [[ ! -f "${legacy_env}" ]] ||
      printf '  legacy secrets file still present: %s\n' "${legacy_env}" >&2
  fi
}

case "${OS}" in
Darwin)
  if [[ "${EUID:-0}" -eq 0 ]]; then
    uninstall_darwin_daemon
  else
    uninstall_darwin_agent
  fi
  ;;
Linux) uninstall_linux ;;
*) die "unsupported OS: ${OS}" ;;
esac

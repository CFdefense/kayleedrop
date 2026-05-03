#!/usr/bin/env bash
#
# Install kayleedrop with **no compilers, Cargo, Rust, or Git**.
#
# - **Linux** (systemd): root + `curl`/`wget`, `tar`, `systemctl`.
# - **macOS (Apple Silicon laptops)**: `curl`, `tar`, `launchctl` — run **without sudo** for a Login
#   LaunchAgent in your Aqua session (`INSTALL_ROOT` defaults under ~/Library/Application Support).
# - Release assets include `kayleedrop-linux-*.tar.gz` and **`kayleedrop-darwin-arm64.tar.gz`**
#   — see `.github/workflows/release-binaries.yml`.
#
# ### One-line
#
#     curl -fsSL https://raw.githubusercontent.com/CFdefense/kayleedrop/main/scripts/install-service.sh | bash -s --
#
# Linux (system timers need root): `… | sudo bash -s --`
#
# ### Overrides
#
#     RELEASE_TAG=v0.1.0 REPO=my/kayleedrop BINARY_URL=https://example.com/kd.tgz bash -s --
#
set -euo pipefail

REPO="${REPO:-CFdefense/kayleedrop}"
RELEASE_TAG="${RELEASE_TAG:-latest}"
BIN_NAME="${BIN_NAME:-kayleedrop}"
BINARY_URL="${BINARY_URL:-}"

LAUNCH_LABEL="${LAUNCH_LABEL:-io.github.cfdefense.kayleedrop}"

# systemd (Linux only)
ON_CALENDAR="${ON_CALENDAR:-daily}"
TIMER_UNIT="${TIMER_UNIT:-${BIN_NAME}-daily.timer}"
SERVICE_UNIT="${SERVICE_UNIT:-${BIN_NAME}-daily.service}"
RANDOM_DELAY="${RANDOM_DELAY:-1h}"

# launchd approximate daily wake (ignored on Linux): hour / minute local time
LAUNCHD_HOUR="${LAUNCHD_HOUR:-9}"
LAUNCHD_MINUTE="${LAUNCHD_MINUTE:-0}"

OS="$(uname -s)"
SERVICE_USER=""
INSTALL_ROOT=""
BIN_DEST=""
ENV_FILE=""

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

http_get_to() {
  local url="$1" dest="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "${url}" -o "${dest}"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "${dest}" "${url}"
  else
    die "need curl or wget to download binaries"
  fi
}

asset_name_for_machine() {
  local machine
  machine="$(uname -m)"
  case "${OS}" in
  Linux)
    case "${machine}" in
    x86_64 | amd64) printf '%s' "kayleedrop-linux-amd64.tar.gz" ;;
    aarch64 | arm64) printf '%s' "kayleedrop-linux-arm64.tar.gz" ;;
    *)
      printf 'error: CPU %s — set BINARY_URL=… for a gz tarball containing %s.\n' "${machine}" "${BIN_NAME}" >&2
      exit 1
      ;;
    esac
    ;;
  Darwin)
    case "${machine}" in
    arm64) printf '%s' "kayleedrop-darwin-arm64.tar.gz" ;;
    x86_64)
      die "Intel macOS binaries are not built by default — set BINARY_URL=… (or install from source)"
      ;;
    *)
      printf 'error: CPU %s — set BINARY_URL=…. \n' "${machine}" >&2
      exit 1
      ;;
    esac
    ;;
  *) die "unsupported OS: ${OS}" ;;
  esac
}

detect_paths_linux() {
  [[ "${EUID:-0}" -eq 0 ]] || die "Linux: rerun with sudo (systemd installs under /etc and /usr/local)"

  INSTALL_ROOT="${INSTALL_ROOT:-/opt/${BIN_NAME}}"
  BIN_DEST="${BIN_DEST:-/usr/local/bin/${BIN_NAME}}"
  ENV_FILE="${ENV_FILE:-/etc/${BIN_NAME}.env}"
  SERVICE_USER="${SERVICE_USER:-${SUDO_USER:-root}}"
}

detect_paths_darwin_root() {
  [[ "${EUID:-0}" -eq 0 ]] || die "internal: Darwin root path requires root"

  INSTALL_ROOT="${INSTALL_ROOT:-/opt/${BIN_NAME}}"
  BIN_DEST="${BIN_DEST:-/usr/local/bin/${BIN_NAME}}"
  ENV_FILE="${ENV_FILE:-/etc/${BIN_NAME}.env}"
  SERVICE_USER="${SERVICE_USER:-${SUDO_USER:-$(logname)}}"
  [[ -n "${SERVICE_USER}" && "${SERVICE_USER}" != root ]] ||
    die "macOS daemon install: sudo from a login user so SUDO_USER is set"
}

detect_paths_darwin_user() {
  [[ "${EUID:-0}" -ne 0 ]] || die "internal"

  INSTALL_ROOT="${INSTALL_ROOT:-${HOME}/Library/Application Support/KayleeDrop}"
  BIN_DEST="${BIN_DEST:-${INSTALL_ROOT}/bin/${BIN_NAME}}"
  ENV_FILE="${ENV_FILE:-${INSTALL_ROOT}/.env}"
}

install_binary_portable() {
  local src="$1" dest="$2"
  mkdir -p "$(dirname "${dest}")"
  install -m 0755 "${src}" "${dest}"
}

write_linux_env_placeholder() {
  if [[ ! -f "${ENV_FILE}" ]]; then
    (
      umask 0177
      cat >"${ENV_FILE}" <<'EOFENV'
# chmod 600. Required:
# PASSWORD=

# Uncomment if iced needs display from systemd/Wayland:
# DISPLAY=:0
EOFENV
    )
    chown "${SERVICE_USER}:${SERVICE_USER}" "${ENV_FILE}"
    chmod 0600 "${ENV_FILE}"
  fi
}

write_darwin_env_placeholder() {
  local legacy_env=""
  [[ "${ENV_FILE}" == "${INSTALL_ROOT}/.env" ]] && legacy_env="${INSTALL_ROOT}/env"
  [[ -n "${legacy_env}" && -f "${legacy_env}" && ! -f "${ENV_FILE}" ]] &&
    cp -p "${legacy_env}" "${ENV_FILE}"

  if [[ ! -f "${ENV_FILE}" ]]; then
    (
      umask 0177
      cat >"${ENV_FILE}" <<'EOFENV'
# chmod 600. Put your decryption secret on the line below (no #).
# Leading # means "comment" — bash/LaunchAgent will NOT export PASSWORD.
PASSWORD=

# Launchd wrappers use `set -a` while sourcing this file so variables reach the binary.

# Older installs used `env` in this directory; launchd loads `.env` first, then falls back.

# Rarely needed for a Login LaunchAgent (inherits your GUI session):
# DISPLAY=
EOFENV
    )
    chmod 0600 "${ENV_FILE}"
    if [[ "${EUID:-0}" -eq 0 ]]; then chown "${SERVICE_USER}:staff" "${ENV_FILE}"; fi
  fi
}

finish_linux() {
  cat >"/etc/systemd/system/${SERVICE_UNIT}" <<EOFSERVICE
[Unit]
Description=${BIN_NAME}: daily decrypt / GUI bootstrap
Documentation=https://github.com/${REPO}
After=network-online.target

[Service]
Type=oneshot
User=${SERVICE_USER}
Group=${SERVICE_USER}
WorkingDirectory=${INSTALL_ROOT}
EnvironmentFile=-${ENV_FILE}
ExecStart=${BIN_DEST}
EOFSERVICE

  cat >"/etc/systemd/system/${TIMER_UNIT}" <<EOFTIMER
[Unit]
Description=Run ${SERVICE_UNIT} once per (${ON_CALENDAR})

[Timer]
OnCalendar=${ON_CALENDAR}
RandomizedDelaySec=${RANDOM_DELAY}
Persistent=true
Unit=${SERVICE_UNIT}

[Install]
WantedBy=timers.target
EOFTIMER

  systemctl daemon-reload
  systemctl enable --now "${TIMER_UNIT}"

  printf $'\nLinux: installed %s (cwd %s). Edit %s (PASSWORD).\nTimers: systemctl list-timers | grep %s\n' \
    "${BIN_DEST}" "${INSTALL_ROOT}" "${ENV_FILE}" "${BIN_NAME}"
}

# shellcheck disable=SC2016
darwin_plist_stdout() {
  printf '%s' "${INSTALL_ROOT}/logs/stdout.log"
}

darwin_mkdir_logs() {
  mkdir -p "${INSTALL_ROOT}/logs"
  if [[ "${EUID:-0}" -eq 0 ]]; then
    chown -R "${SERVICE_USER}:staff" "${INSTALL_ROOT}/logs"
  fi
}

finish_darwin_daemon() {
  local plist=/Library/LaunchDaemons/"${LAUNCH_LABEL}.plist"
  mkdir -p "${INSTALL_ROOT}/data/source" "${INSTALL_ROOT}/data/destination"
  chown -R "${SERVICE_USER}:staff" "${INSTALL_ROOT}"

  darwin_mkdir_logs

  cat >"${plist}" <<EOFPLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${LAUNCH_LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/bash</string>
    <string>-lc</string>
      <string>set -euo pipefail
if [[ ! -f &quot;${ENV_FILE}&quot; ]]; then
  printf &apos;%s: env file missing: %s\n&apos; &quot;${LAUNCH_LABEL}&quot; &quot;${ENV_FILE}&quot; &gt;&amp;2
elif [[ ! -r &quot;${ENV_FILE}&quot; ]]; then
  printf &apos;%s: env file exists but is not readable: %s\n&apos; &quot;${LAUNCH_LABEL}&quot; &quot;${ENV_FILE}&quot; &gt;&amp;2
fi
if [[ -r &quot;${ENV_FILE}&quot; ]]; then
  set -a
  source &quot;${ENV_FILE}&quot;
  set +a
fi
cd &quot;${INSTALL_ROOT}&quot;
exec &quot;${BIN_DEST}&quot;</string>
  </array>
  <key>UserName</key>
  <string>${SERVICE_USER}</string>
  <key>WorkingDirectory</key>
  <string>${INSTALL_ROOT}</string>
  <key>StandardOutPath</key>
  <string>$(darwin_plist_stdout)</string>
  <key>StandardErrorPath</key>
  <string>${INSTALL_ROOT}/logs/stderr.log</string>
  <key>RunAtLoad</key>
  <false/>
  <key>StartCalendarInterval</key>
  <dict>
    <key>Hour</key><integer>${LAUNCHD_HOUR}</integer>
    <key>Minute</key><integer>${LAUNCHD_MINUTE}</integer>
  </dict>
</dict>
</plist>
EOFPLIST

  chmod 0644 "${plist}"
  launchctl bootout system "${plist}" 2>/dev/null || true
  launchctl bootstrap system "${plist}"

  printf $'\nmacOS (daemon): installed %s, plist %s, secrets %s (chmod 600; PASSWORD uncommented)\n daily ~%02d:%02d local; stdout/stderr: %s and %s\ninspect: sudo launchctl print system/%s\nrun now: sudo launchctl kickstart -k system/%s\n' \
    "${BIN_DEST}" "${plist}" "${ENV_FILE}" "${LAUNCHD_HOUR}" "${LAUNCHD_MINUTE}" \
    "$(darwin_plist_stdout)" "${INSTALL_ROOT}/logs/stderr.log" \
    "${LAUNCH_LABEL}" "${LAUNCH_LABEL}"
}

finish_darwin_agent() {
  local plist_dest="${HOME}/Library/LaunchAgents/${LAUNCH_LABEL}.plist"

  mkdir -p "${INSTALL_ROOT}/data/source" "${INSTALL_ROOT}/data/destination"

  cat >"${plist_dest}" <<EOFPLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${LAUNCH_LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/bash</string>
    <string>-lc</string>
      <string>set -euo pipefail
ENV_DOT=&quot;${INSTALL_ROOT}/.env&quot;
ENV_LEGACY=&quot;${INSTALL_ROOT}/env&quot;
if [[ ! -f &quot;\$ENV_DOT&quot; ]] &amp;&amp; [[ ! -f &quot;\$ENV_LEGACY&quot; ]]; then
  printf &apos;%s: no secrets file; create %s with PASSWORD=your_secret (not commented with #).\n&apos; &quot;${LAUNCH_LABEL}&quot; &quot;\$ENV_DOT&quot; &gt;&amp;2
else
  for f in &quot;\$ENV_DOT&quot; &quot;\$ENV_LEGACY&quot;; do
    if [[ -f &quot;\$f&quot; ]] &amp;&amp; [[ ! -r &quot;\$f&quot; ]]; then
      printf &apos;%s: secrets file exists but is not readable: %s\n&apos; &quot;${LAUNCH_LABEL}&quot; &quot;\$f&quot; &gt;&amp;2
    fi
  done
fi
_loaded=0
if [[ -r &quot;\$ENV_DOT&quot; ]]; then
  set -a
  source &quot;\$ENV_DOT&quot;
  set +a
  _loaded=1
fi
if [[ &quot;\$_loaded&quot; -eq 0 ]] &amp;&amp; [[ -r &quot;\$ENV_LEGACY&quot; ]]; then
  set -a
  source &quot;\$ENV_LEGACY&quot;
  set +a
fi
cd &quot;${INSTALL_ROOT}&quot;
exec &quot;${BIN_DEST}&quot;</string>
  </array>
  <key>WorkingDirectory</key>
  <string>${INSTALL_ROOT}</string>
  <key>StandardOutPath</key>
  <string>/tmp/kayleedrop.out</string>
  <key>StandardErrorPath</key>
  <string>/tmp/kayleedrop.err</string>
  <key>RunAtLoad</key>
  <false/>
  <key>StartCalendarInterval</key>
  <dict>
    <key>Hour</key><integer>${LAUNCHD_HOUR}</integer>
    <key>Minute</key><integer>${LAUNCHD_MINUTE}</integer>
  </dict>
</dict>
</plist>
EOFPLIST

  local uid gui
  uid="$(id -u)"
  gui="gui/${uid}"

  launchctl bootout "${gui}" "${plist_dest}" 2>/dev/null || true
  launchctl bootstrap "${gui}" "${plist_dest}"

  printf $'\nmacOS (Login LaunchAgent): installed for user %s\n  binary %s\n  data   %s\n  secrets %s (chmod 600; PASSWORD= line must not start with #)\n  plist  %s\n  stdout / stderr: /tmp/kayleedrop.out and /tmp/kayleedrop.err (startup logs missing/unreadable secrets here)\n  daily ~%02d:%02d local wall-clock\ninspect:\n  launchctl print gui/%s/%s\nrun now:\n  launchctl kickstart -k gui/%s/%s\nunload:\n  launchctl bootout gui/%s/%s\n' \
    "$(whoami)" "${BIN_DEST}" "${INSTALL_ROOT}" "${ENV_FILE}" "${plist_dest}" \
    "${LAUNCHD_HOUR}" "${LAUNCHD_MINUTE}" \
    "${uid}" "${LAUNCH_LABEL}" \
    "${uid}" "${LAUNCH_LABEL}" \
    "${uid}" "${LAUNCH_LABEL}"
}

command -v tar >/dev/null 2>&1 || die "tar is required"

if [[ "${OS}" == Darwin ]]; then
  if [[ "${EUID:-0}" -ne 0 ]]; then detect_paths_darwin_user
  else detect_paths_darwin_root
  fi
else
  detect_paths_linux
fi

tmp="$(mktemp)"
tmpdir="$(mktemp -d)"
cleanup() {
  rm -rf "${tmp}" "${tmpdir}"
}
trap cleanup EXIT

if [[ -n "${BINARY_URL}" ]]; then
  http_get_to "${BINARY_URL}" "${tmp}"
else
  arch_asset="$(asset_name_for_machine)"
  if [[ "${RELEASE_TAG}" == latest ]]; then
    url="https://github.com/${REPO}/releases/latest/download/${arch_asset}"
  else
    url="https://github.com/${REPO}/releases/download/${RELEASE_TAG}/${arch_asset}"
  fi
  http_get_to "${url}" "${tmp}" ||
    die "could not fetch ${url} — publish a Release asset ${arch_asset} or BINARY_URL=…"
fi

tar xzf "${tmp}" -C "${tmpdir}"
candidate="${tmpdir}/${BIN_NAME}"
if [[ ! -f "${candidate}" ]]; then
  candidate="$(find "${tmpdir}" -maxdepth 3 -type f -name "${BIN_NAME}" -print | head -n 1)"
fi
if [[ -z "${candidate}" || ! -f "${candidate}" ]]; then
  die "archive missing '${BIN_NAME}' executable"
fi
chmod 0755 "${candidate}"

mkdir -p "${INSTALL_ROOT}/data/source" "${INSTALL_ROOT}/data/destination"

if [[ "${OS}" == Darwin && "${EUID:-0}" -ne 0 ]]; then
  install_binary_portable "${candidate}" "${BIN_DEST}"
else
  install_binary_portable "${candidate}" "${BIN_DEST}"
  if [[ "${OS}" == Linux ]]; then chown -R "${SERVICE_USER}:${SERVICE_USER}" "${INSTALL_ROOT}"
  elif [[ "${OS}" == Darwin && "${EUID:-0}" -eq 0 ]]; then chown -R "${SERVICE_USER}:staff" "${INSTALL_ROOT}"
  fi
fi

if [[ "${OS}" == Linux ]]; then
  write_linux_env_placeholder
  finish_linux
elif [[ "${OS}" == Darwin ]]; then
  write_darwin_env_placeholder
  if [[ "${EUID:-0}" -eq 0 ]]; then finish_darwin_daemon; else finish_darwin_agent; fi
fi

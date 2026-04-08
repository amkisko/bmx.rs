#!/usr/bin/env bash
set -euo pipefail

APP_NAME="bmx"
CARGO_BIN_NAME="bmx_rs"
DEFAULT_REF="main"

if [[ -n "${BASH_SOURCE[0]-}" && -f "${BASH_SOURCE[0]}" ]]; then
  SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
else
  SCRIPT_DIR="$(pwd)"
fi

PREFIX="/usr/local"
REF="${BMX_REF:-${DEFAULT_REF}}"
REPO_URL="${BMX_REPO_URL-}"
LOCAL_SOURCE=""
USE_SUDO=1
UNINSTALL=0

usage() {
  cat <<USAGE
Install ${APP_NAME} system-wide (default: /usr/local/bin/${APP_NAME}).

Usage:
  ./setup.sh [options]

Options:
  --repo <url>         Git repo URL (required when source is not local)
  --ref <git-ref>      Git ref/tag/branch for remote install (default: ${DEFAULT_REF})
  --source <dir>       Explicit local source directory
  --prefix <dir>       Install prefix (default: /usr/local)
  --user               Install under ~/.local (equivalent to --prefix ~/.local)
  --no-sudo            Do not attempt sudo for protected install paths
  --uninstall          Remove installed binary and exit
  -h, --help           Show this help

Examples:
  Local install from repo folder:
    ./setup.sh

  Curl/bootstrap install:
    curl -fsSL <setup.sh-url> | bash -s -- --repo https://github.com/amkisko/bmx.rs.git

  User-local install:
    ./setup.sh --user
USAGE
}

log() {
  printf '[%s] %s\n' "${APP_NAME}" "$*" >&2
}

err() {
  printf '[%s] ERROR: %s\n' "${APP_NAME}" "$*" >&2
}

warn() {
  printf '[%s] WARN: %s\n' "${APP_NAME}" "$*" >&2
}

has_tty() {
  [[ -r /dev/tty && -w /dev/tty ]]
}

prompt_yes_no() {
  local prompt="$1"
  local default="${2:-N}"
  local answer=""

  if ! has_tty; then
    return 1
  fi

  if [[ ${default} == "Y" ]]; then
    printf '%s [Y/n]: ' "${prompt}" > /dev/tty
  else
    printf '%s [y/N]: ' "${prompt}" > /dev/tty
  fi

  IFS= read -r answer < /dev/tty || return 1
  if [[ -z ${answer} ]]; then
    answer="${default}"
  fi

  case "${answer}" in
  y | Y | yes | YES)
    return 0
    ;;
  *)
    return 1
    ;;
  esac
}

detect_pkg_manager() {
  local manager=""
  for manager in brew apt-get dnf yum pacman zypper apk; do
    if command -v "${manager}" >/dev/null 2>&1; then
      printf '%s\n' "${manager}"
      return 0
    fi
  done
  return 1
}

run_with_privilege() {
  if [[ ${EUID} -eq 0 ]]; then
    "$@"
    return $?
  fi

  if command -v sudo >/dev/null 2>&1; then
    sudo "$@"
    return $?
  fi

  err "This step requires root privileges and sudo is not available"
  return 1
}

print_cargo_install_help() {
  local pkg_manager="$1"

  warn "Missing required command: cargo"
  printf 'Install Rust (includes cargo) from the official Rust source:\n' >&2
  printf '  curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal\n' >&2
  printf 'Then load PATH in the current shell:\n' >&2
  printf '  source "$HOME/.cargo/env"\n' >&2

  if [[ -n ${pkg_manager} ]]; then
    printf 'Detected package manager: %s\n' "${pkg_manager}" >&2
  fi
}

install_git_with_package_manager() {
  local pkg_manager="$1"

  case "${pkg_manager}" in
  brew)
    brew install git
    ;;
  apt-get)
    run_with_privilege apt-get update
    run_with_privilege apt-get install -y git
    ;;
  dnf)
    run_with_privilege dnf install -y git
    ;;
  yum)
    run_with_privilege yum install -y git
    ;;
  pacman)
    run_with_privilege pacman -Sy --noconfirm git
    ;;
  zypper)
    run_with_privilege zypper --non-interactive install git
    ;;
  apk)
    run_with_privilege apk add --no-cache git
    ;;
  *)
    return 1
    ;;
  esac
}

install_cargo_with_rustup() {
  local rustup_script
  rustup_script="$(mktemp)"

  if ! command -v curl >/dev/null 2>&1; then
    err "curl is required to install Rust via rustup"
    rm -f "${rustup_script}"
    return 1
  fi

  log "Downloading rustup installer from https://sh.rustup.rs"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "${rustup_script}"
  sh "${rustup_script}" -y --profile minimal
  rm -f "${rustup_script}"

  if [[ -f "${HOME}/.cargo/env" ]]; then
    # shellcheck disable=SC1090
    source "${HOME}/.cargo/env"
  fi
}

ensure_git() {
  if command -v git >/dev/null 2>&1; then
    return 0
  fi

  local pkg_manager=""
  pkg_manager="$(detect_pkg_manager || true)"
  warn "Missing required command: git"

  if [[ -n ${pkg_manager} ]]; then
    printf 'Install command (recommended): ' >&2
    case "${pkg_manager}" in
    brew)
      printf 'brew install git\n' >&2
      ;;
    apt-get)
      printf 'sudo apt-get update && sudo apt-get install -y git\n' >&2
      ;;
    dnf)
      printf 'sudo dnf install -y git\n' >&2
      ;;
    yum)
      printf 'sudo yum install -y git\n' >&2
      ;;
    pacman)
      printf 'sudo pacman -Sy --noconfirm git\n' >&2
      ;;
    zypper)
      printf 'sudo zypper --non-interactive install git\n' >&2
      ;;
    apk)
      printf 'sudo apk add --no-cache git\n' >&2
      ;;
    esac

    if prompt_yes_no "Install git now using ${pkg_manager}?" "Y"; then
      install_git_with_package_manager "${pkg_manager}" || {
        err "Automatic git installation failed"
        return 1
      }
    fi
  fi

  if ! command -v git >/dev/null 2>&1; then
    err "git is required for --repo installs"
    return 1
  fi
}

ensure_cargo() {
  if command -v cargo >/dev/null 2>&1; then
    return 0
  fi

  local pkg_manager=""
  pkg_manager="$(detect_pkg_manager || true)"
  print_cargo_install_help "${pkg_manager}"

  if prompt_yes_no "Install Rust toolchain now via official rustup installer?" "Y"; then
    install_cargo_with_rustup || {
      err "Automatic Rust installation failed"
      return 1
    }
  fi

  if ! command -v cargo >/dev/null 2>&1; then
    err "cargo is still unavailable; install Rust and re-run setup.sh"
    return 1
  fi
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    err "Missing required command: $1"
    exit 1
  fi
}

resolve_abs_path() {
  local p="$1"
  if [[ ${p} == /* ]]; then
    printf '%s\n' "${p}"
  else
    local here
    here="$(pwd)"
    printf '%s\n' "${here}/${p}"
  fi
}

find_local_source() {
  if [[ -n ${LOCAL_SOURCE} ]]; then
    if [[ -f "${LOCAL_SOURCE}/Cargo.toml" ]]; then
      local resolved
      resolved="$(resolve_abs_path "${LOCAL_SOURCE}")"
      printf '%s\n' "${resolved}"
      return 0
    fi
    err "--source does not contain Cargo.toml: ${LOCAL_SOURCE}"
    exit 1
  fi

  if [[ -f "${SCRIPT_DIR}/Cargo.toml" ]]; then
    printf '%s\n' "${SCRIPT_DIR}"
    return 0
  fi

  local here
  here="$(pwd)"
  if [[ -f "${here}/Cargo.toml" ]]; then
    printf '%s\n' "${here}"
    return 0
  fi

  return 1
}

fetch_remote_source() {
  local tmpdir="$1"
  local repo_url="$2"
  local ref="$3"

  ensure_git

  log "Cloning source from ${repo_url} (${ref})"
  git clone --depth 1 --branch "${ref}" "${repo_url}" "${tmpdir}/src" >/dev/null 2>&1 || {
    err "Failed to clone ${repo_url} at ref ${ref}"
    exit 1
  }

  if [[ ! -f "${tmpdir}/src/Cargo.toml" ]]; then
    err "Remote source does not contain Cargo.toml at repository root"
    exit 1
  fi

  printf '%s\n' "${tmpdir}/src"
}

pick_built_binary() {
  local src="$1"
  local candidate_primary="${src}/target/release/${APP_NAME}"
  local candidate_fallback="${src}/target/release/${CARGO_BIN_NAME}"

  if [[ -x ${candidate_primary} ]]; then
    printf '%s\n' "${candidate_primary}"
    return 0
  fi

  if [[ -x ${candidate_fallback} ]]; then
    printf '%s\n' "${candidate_fallback}"
    return 0
  fi

  err "Built binary not found at ${candidate_primary} or ${candidate_fallback}"
  exit 1
}

copy_binary() {
  local src_bin="$1"
  local dest_bin="$2"
  local dest_dir
  dest_dir="$(dirname "${dest_bin}")"

  if [[ ! -d ${dest_dir} ]]; then
    if mkdir -p "${dest_dir}" 2>/dev/null; then
      :
    elif [[ ${USE_SUDO} -eq 1 ]]; then
      require_cmd sudo
      sudo mkdir -p "${dest_dir}"
    else
      err "Cannot create ${dest_dir}; rerun without --no-sudo or use --user"
      exit 1
    fi
  fi

  if install -m 0755 "${src_bin}" "${dest_bin}" 2>/dev/null; then
    return 0
  fi

  if [[ ${USE_SUDO} -eq 1 ]]; then
    require_cmd sudo
    sudo install -m 0755 "${src_bin}" "${dest_bin}"
    return 0
  fi

  err "No write access to ${dest_dir} and --no-sudo was provided"
  err "Try: --user (installs to ~/.local/bin) or --prefix <dir>"
  exit 1
}

preflight_install_permissions() {
  local prefix="$1"
  local dest_dir="${prefix}/bin"

  if [[ -d ${dest_dir} && -w ${dest_dir} ]]; then
    return 0
  fi

  if [[ ! -d ${dest_dir} && -w ${prefix} ]]; then
    return 0
  fi

  if [[ ${USE_SUDO} -eq 0 ]]; then
    err "No write access to ${dest_dir} and --no-sudo was provided"
    err "Use --user (recommended) or choose a writable --prefix"
    exit 1
  fi

  require_cmd sudo
  warn "Install target ${dest_dir} likely requires elevated permissions"

  if has_tty && prompt_yes_no "Acquire sudo permission now?" "Y"; then
    sudo -v || {
      err "Unable to acquire sudo credentials"
      err "Use --user to install without elevated permissions"
      exit 1
    }
  else
    warn "sudo may prompt during install copy step"
  fi
}

build_and_install() {
  local src="$1"
  local prefix="$2"
  local dest_bin="${prefix}/bin/${APP_NAME}"

  ensure_cargo
  preflight_install_permissions "${prefix}"

  log "Building ${APP_NAME} from ${src}"
  cargo build --manifest-path "${src}/Cargo.toml" --release --locked

  local built_bin
  built_bin="$(pick_built_binary "${src}")"

  log "Installing to ${dest_bin}"
  copy_binary "${built_bin}" "${dest_bin}"

  log "Installed ${APP_NAME}"
  log "Run: ${APP_NAME} --help"

  if [[ ${prefix} == "${HOME}/.local" ]]; then
    log "Ensure ~/.local/bin is in PATH"
  fi
}

while [[ ${#} -gt 0 ]]; do
  case "${1}" in
  --repo)
    REPO_URL="${2}"
    shift 2
    ;;
  --ref)
    REF="${2}"
    shift 2
    ;;
  --source)
    LOCAL_SOURCE="${2}"
    shift 2
    ;;
  --prefix)
    PREFIX="${2}"
    shift 2
    ;;
  --user)
    PREFIX="${HOME}/.local"
    shift
    ;;
  --no-sudo)
    USE_SUDO=0
    shift
    ;;
  --uninstall)
    UNINSTALL=1
    shift
    ;;
  -h | --help)
    usage
    exit 0
    ;;
  *)
    err "Unknown argument: ${1}"
    usage
    exit 1
    ;;
  esac
done

PREFIX="$(resolve_abs_path "${PREFIX}")"
DEST_BIN="${PREFIX}/bin/${APP_NAME}"

if [[ ${UNINSTALL} -eq 1 ]]; then
  if [[ -f ${DEST_BIN} ]]; then
    if rm -f "${DEST_BIN}" 2>/dev/null; then
      :
    elif [[ ${USE_SUDO} -eq 1 ]]; then
      require_cmd sudo
      sudo rm -f "${DEST_BIN}"
    else
      err "No write access to remove ${DEST_BIN}; rerun without --no-sudo"
      err "Try uninstall with sudo or reinstall using --user"
      exit 1
    fi
    log "Removed ${DEST_BIN}"
  else
    log "No install found at ${DEST_BIN}"
  fi
  exit 0
fi

SRC_DIR=""
TMPDIR_TO_CLEAN=""

cleanup() {
  if [[ -n ${TMPDIR_TO_CLEAN} && -d ${TMPDIR_TO_CLEAN} ]]; then
    rm -rf "${TMPDIR_TO_CLEAN}"
  fi
}
trap cleanup EXIT

set +e
SRC_DIR="$(find_local_source)"
find_rc=${?}
set -e
if [[ ${find_rc} -eq 0 ]]; then
  log "Using local source: ${SRC_DIR}"
else
  if [[ -z ${REPO_URL} ]]; then
    err "No local source found and --repo is not set"
    err "Example: curl ... | bash -s -- --repo https://github.com/amkisko/bmx.rs.git"
    exit 1
  fi
  TMPDIR_TO_CLEAN="$(mktemp -d)"
  SRC_DIR="$(fetch_remote_source "${TMPDIR_TO_CLEAN}" "${REPO_URL}" "${REF}")"
fi

build_and_install "${SRC_DIR}" "${PREFIX}"

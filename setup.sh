#!/usr/bin/env bash
set -euo pipefail

APP_NAME="bmx"
CARGO_BIN_NAME="bmx_rs"
DEFAULT_REF="main"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"

PREFIX="/usr/local"
REF="${BMX_REF:-${DEFAULT_REF}}"
REPO_URL="${BMX_REPO_URL:-}"
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
  printf '[%s] %s\n' "${APP_NAME}" "$*"
}

err() {
  printf '[%s] ERROR: %s\n' "${APP_NAME}" "$*" >&2
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    err "Missing required command: $1"
    exit 1
  fi
}

resolve_abs_path() {
  local p="$1"
  if [[ "$p" == /* ]]; then
    printf '%s\n' "$p"
  else
    printf '%s\n' "$(pwd)/$p"
  fi
}

find_local_source() {
  if [[ -n "$LOCAL_SOURCE" ]]; then
    if [[ -f "${LOCAL_SOURCE}/Cargo.toml" ]]; then
      printf '%s\n' "$(resolve_abs_path "$LOCAL_SOURCE")"
      return 0
    fi
    err "--source does not contain Cargo.toml: ${LOCAL_SOURCE}"
    exit 1
  fi

  if [[ -f "${SCRIPT_DIR}/Cargo.toml" ]]; then
    printf '%s\n' "${SCRIPT_DIR}"
    return 0
  fi

  if [[ -f "$(pwd)/Cargo.toml" ]]; then
    printf '%s\n' "$(pwd)"
    return 0
  fi

  return 1
}

fetch_remote_source() {
  local tmpdir="$1"
  local repo_url="$2"
  local ref="$3"

  require_cmd git

  log "Cloning source from ${repo_url} (${ref})"
  git clone --depth 1 --branch "$ref" "$repo_url" "$tmpdir/src" >/dev/null 2>&1 || {
    err "Failed to clone ${repo_url} at ref ${ref}"
    exit 1
  }

  if [[ ! -f "$tmpdir/src/Cargo.toml" ]]; then
    err "Remote source does not contain Cargo.toml at repository root"
    exit 1
  fi

  printf '%s\n' "$tmpdir/src"
}

pick_built_binary() {
  local src="$1"
  local candidate_primary="${src}/target/release/${APP_NAME}"
  local candidate_fallback="${src}/target/release/${CARGO_BIN_NAME}"

  if [[ -x "$candidate_primary" ]]; then
    printf '%s\n' "$candidate_primary"
    return 0
  fi

  if [[ -x "$candidate_fallback" ]]; then
    printf '%s\n' "$candidate_fallback"
    return 0
  fi

  err "Built binary not found at ${candidate_primary} or ${candidate_fallback}"
  exit 1
}

copy_binary() {
  local src_bin="$1"
  local dest_bin="$2"

  mkdir -p "$(dirname "$dest_bin")"

  if [[ -w "$(dirname "$dest_bin")" ]]; then
    install -m 0755 "$src_bin" "$dest_bin"
    return 0
  fi

  if [[ "$USE_SUDO" -eq 1 ]]; then
    require_cmd sudo
    sudo install -m 0755 "$src_bin" "$dest_bin"
    return 0
  fi

  err "No write access to $(dirname "$dest_bin") and --no-sudo was provided"
  exit 1
}

build_and_install() {
  local src="$1"
  local prefix="$2"
  local dest_bin="${prefix}/bin/${APP_NAME}"

  require_cmd cargo

  log "Building ${APP_NAME} from ${src}"
  cargo build --manifest-path "${src}/Cargo.toml" --release --locked

  local built_bin
  built_bin="$(pick_built_binary "$src")"

  log "Installing to ${dest_bin}"
  copy_binary "$built_bin" "$dest_bin"

  log "Installed ${APP_NAME}"
  log "Run: ${APP_NAME} --help"

  if [[ "$prefix" == "${HOME}/.local" ]]; then
    log "Ensure ~/.local/bin is in PATH"
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo)
      REPO_URL="$2"
      shift 2
      ;;
    --ref)
      REF="$2"
      shift 2
      ;;
    --source)
      LOCAL_SOURCE="$2"
      shift 2
      ;;
    --prefix)
      PREFIX="$2"
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
    -h|--help)
      usage
      exit 0
      ;;
    *)
      err "Unknown argument: $1"
      usage
      exit 1
      ;;
  esac
done

PREFIX="$(resolve_abs_path "$PREFIX")"
DEST_BIN="${PREFIX}/bin/${APP_NAME}"

if [[ "$UNINSTALL" -eq 1 ]]; then
  if [[ -f "$DEST_BIN" ]]; then
    if [[ -w "$(dirname "$DEST_BIN")" ]]; then
      rm -f "$DEST_BIN"
    elif [[ "$USE_SUDO" -eq 1 ]]; then
      require_cmd sudo
      sudo rm -f "$DEST_BIN"
    else
      err "No write access to remove ${DEST_BIN}; rerun without --no-sudo"
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
  if [[ -n "$TMPDIR_TO_CLEAN" && -d "$TMPDIR_TO_CLEAN" ]]; then
    rm -rf "$TMPDIR_TO_CLEAN"
  fi
}
trap cleanup EXIT

if SRC_DIR="$(find_local_source)"; then
  log "Using local source: ${SRC_DIR}"
else
  if [[ -z "$REPO_URL" ]]; then
    err "No local source found and --repo is not set"
    err "Example: curl ... | bash -s -- --repo https://github.com/amkisko/bmx.rs.git"
    exit 1
  fi
  TMPDIR_TO_CLEAN="$(mktemp -d)"
  SRC_DIR="$(fetch_remote_source "$TMPDIR_TO_CLEAN" "$REPO_URL" "$REF")"
fi

build_and_install "$SRC_DIR" "$PREFIX"

#!/usr/bin/env bash
set -euo pipefail

DEFAULT_REPO="kadaliao/clashctl"
REPO="${REPO:-${1:-${DEFAULT_REPO}}}"
VERSION="${VERSION:-${2:-latest}}"
DEST="${DEST:-/usr/local/bin}"

if [[ -z "${REPO}" ]]; then
  echo "Usage: $0 <owner/repo> [version]" >&2
  echo "Example: $0 you/clashctl 0.1.0" >&2
  echo "You can also set REPO and VERSION env vars." >&2
  exit 1
fi

OS="$(uname -s)"
if [[ "${OS}" != "Darwin" ]]; then
  echo "This installer only supports macOS. Detected: ${OS}" >&2
  exit 1
fi

ARCH="$(uname -m)"
case "${ARCH}" in
  arm64) ARCH=arm64 ;;
  x86_64) ARCH=x86_64 ;;
  *) echo "Unsupported arch: ${ARCH}" >&2; exit 1 ;;
esac

if [[ "${VERSION}" == "latest" ]]; then
  API_URL="https://api.github.com/repos/${REPO}/releases/latest"
else
  API_URL="https://api.github.com/repos/${REPO}/releases/tags/v${VERSION}"
fi

RESPONSE="$(
  curl -sSL \
    -H "Accept: application/vnd.github+json" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    -w '\n%{http_code}' \
    "${API_URL}"
)"
BODY="${RESPONSE%$'\n'*}"
STATUS="${RESPONSE##*$'\n'}"
if [[ "${STATUS}" != "200" ]]; then
  echo "Failed to fetch release metadata from GitHub (HTTP ${STATUS})." >&2
  echo "If you haven't published a release yet, create one and retry." >&2
  if [[ -n "${BODY}" ]]; then
    echo "${BODY}" >&2
  fi
  exit 1
fi

ASSET_URL="$(
  printf '%s' "${BODY}" | ARCH="${ARCH}" python3 -c '
import json
import os
import sys

arch = os.environ["ARCH"]
data = json.load(sys.stdin)
tag = data.get("tag_name", "")
version = tag[1:] if tag.startswith("v") else tag
name = f"clashctl-{version}-macos-{arch}.tar.gz"
for asset in data.get("assets", []):
    if asset.get("name") == name:
        print(asset.get("browser_download_url"))
        sys.exit(0)

sys.stderr.write(f"Release asset not found: {name}\n")
sys.exit(1)
'
)"

TMP_DIR="$(mktemp -d)"
cleanup() { rm -rf "${TMP_DIR}"; }
trap cleanup EXIT

curl -fsSL "${ASSET_URL}" -o "${TMP_DIR}/clashctl.tar.gz"
tar -xzf "${TMP_DIR}/clashctl.tar.gz" -C "${TMP_DIR}"
chmod +x "${TMP_DIR}/clashctl"

if [[ ! -d "${DEST}" ]]; then
  echo "Destination does not exist: ${DEST}" >&2
  exit 1
fi

if [[ -w "${DEST}" ]]; then
  mv "${TMP_DIR}/clashctl" "${DEST}/clashctl"
else
  sudo mv "${TMP_DIR}/clashctl" "${DEST}/clashctl"
fi

echo "Installed clashctl to ${DEST}/clashctl"

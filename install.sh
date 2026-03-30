#!/bin/sh
# Nous installer — https://github.com/teddytennant/nous
# Usage: curl -fsSL https://raw.githubusercontent.com/teddytennant/nous/main/install.sh | sh
set -e

REPO="teddytennant/nous"
INSTALL_DIR="${NOUS_INSTALL_DIR:-/usr/local/bin}"

info() { printf "\033[1;34m==>\033[0m %s\n" "$1"; }
err()  { printf "\033[1;31merror:\033[0m %s\n" "$1" >&2; exit 1; }

detect_target() {
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  os_part="unknown-linux-gnu" ;;
        Darwin) os_part="apple-darwin" ;;
        *)      err "Unsupported OS: $os. Use cargo install or download manually." ;;
    esac

    case "$arch" in
        x86_64|amd64)   arch_part="x86_64" ;;
        aarch64|arm64)   arch_part="aarch64" ;;
        *)               err "Unsupported architecture: $arch" ;;
    esac

    echo "${arch_part}-${os_part}"
}

get_latest_version() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"//;s/".*//'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"//;s/".*//'
    else
        err "curl or wget is required"
    fi
}

download() {
    url="$1"; dest="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$url" -O "$dest"
    fi
}

main() {
    target="$(detect_target)"
    version="${NOUS_VERSION:-$(get_latest_version)}"

    if [ -z "$version" ]; then
        err "Could not determine latest version. Set NOUS_VERSION=vX.Y.Z manually."
    fi

    info "Installing nous ${version} for ${target}"

    archive="nous-${version}-${target}.tar.gz"
    url="https://github.com/${REPO}/releases/download/${version}/${archive}"

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    info "Downloading ${url}"
    download "$url" "${tmpdir}/${archive}"

    info "Extracting"
    tar xzf "${tmpdir}/${archive}" -C "$tmpdir"

    info "Installing to ${INSTALL_DIR}"
    if [ -w "$INSTALL_DIR" ]; then
        cp "${tmpdir}/nous-${version}-${target}/nous" "$INSTALL_DIR/"
        cp "${tmpdir}/nous-${version}-${target}/nous-api" "$INSTALL_DIR/"
    else
        sudo cp "${tmpdir}/nous-${version}-${target}/nous" "$INSTALL_DIR/"
        sudo cp "${tmpdir}/nous-${version}-${target}/nous-api" "$INSTALL_DIR/"
    fi

    chmod +x "${INSTALL_DIR}/nous" "${INSTALL_DIR}/nous-api" 2>/dev/null || \
        sudo chmod +x "${INSTALL_DIR}/nous" "${INSTALL_DIR}/nous-api"

    info "Installed nous ${version} to ${INSTALL_DIR}/nous"
    info "Installed nous-api ${version} to ${INSTALL_DIR}/nous-api"
    echo
    echo "Run 'nous init' to get started."
}

main

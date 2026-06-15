#!/bin/bash
set -e

REPO="antraxbr666/dof"
BINARY="dof"
INSTALL_DIR="/usr/local/bin"

# ─── Colors (Catppuccin Mocha) ───────────────────────────────────
RED='\033[38;2;243;139;168m'
GREEN='\033[38;2;166;227;161m'
YELLOW='\033[38;2;249;226;175m'
BLUE='\033[38;2;137;180;250m'
MAUVE='\033[38;2;203;166;247m'
TEAL='\033[38;2;148;226;213m'
LAVENDER='\033[38;2;180;190;254m'
CRUST='\033[38;2;17;17;27m'
OVERLAY='\033[38;2;205;214;244m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

# ─── Icons ────────────────────────────────────────────────────────
DOCKER="🐳"
ROCKET="🚀"
CHECK="✅"
CROSS="❌"
WARN="⚠️"

info()    { echo -e "  ${DOCKER}  $1" >&2; }
success() { echo -e "  ${CHECK}  ${GREEN}$1${NC}" >&2; }
warn()    { echo -e "  ${WARN}  ${YELLOW}$1${NC}" >&2; }
error()   { echo -e "  ${CROSS}  ${RED}$1${NC}" >&2; exit 1; }

banner() {
    echo "" >&2
    echo -e "  ${LAVENDER}${BOLD}dof${NC} ${DIM}— A beautiful, blazing-fast terminal Docker container view and real-time stats.${NC}" >&2
    echo -e "  ${TEAL}${BOLD}Install Script${NC}" >&2
    echo "" >&2
}

divider() {
    echo -e "  ${DIM}──────────────────────────────────────────${NC}" >&2
}

pause() {
    echo "" >&2
    echo -e "  ${YELLOW}Press Enter to continue...${NC}" >&2
    read -r
}

# ─── Architecture Detection ──────────────────────────────────────
detect_arch() {
    local arch
    arch=$(uname -m)
    case "$arch" in
        x86_64|amd64)   echo "x86_64" ;;
        aarch64|arm64)  echo "aarch64" ;;
        *) error "Unsupported architecture: $arch" ;;
    esac
}

# ─── OS Detection ────────────────────────────────────────────────
detect_os() {
    local os
    os=$(uname -s)
    case "$os" in
        Linux) echo "linux" ;;
        *)     error "Unsupported OS: $os (only Linux is supported)" ;;
    esac
}

# ─── Version Detection ───────────────────────────────────────────
get_latest_version() {
    local version
    version=$(curl -s "https://api.github.com/repos/$REPO/releases" | grep -o '"tag_name": "[^"]*"' | head -1 | sed -E 's/.*"v([^"]+)".*/\1/')
    if [ -z "$version" ]; then
        error "Failed to get latest version"
    fi
    echo "$version"
}

# ─── Download ────────────────────────────────────────────────────
download_binary() {
    local arch=$1
    local version=$2
    local url="https://github.com/$REPO/releases/download/v${version}/dof-${arch}-linux"
    local tmp_file="/tmp/dof-${arch}-linux"

    info "Downloading ${TEAL}${BINARY} v${version}${NC} for ${LAVENDER}${arch}${NC}..."
    echo "" >&2
    curl -#L "$url" -o "$tmp_file"

    if [ ! -f "$tmp_file" ]; then
        error "Download failed"
    fi

    chmod +x "$tmp_file"
    echo "$tmp_file"
}

# ─── Install ─────────────────────────────────────────────────────
install_binary() {
    local tmp_file=$1

    if [ -w "$INSTALL_DIR" ]; then
        rm -f "$INSTALL_DIR/$BINARY"
        cp "$tmp_file" "$INSTALL_DIR/$BINARY"
        chmod +x "$INSTALL_DIR/$BINARY"
    else
        warn "Need sudo to install to $INSTALL_DIR"
        pause
        sudo rm -f "$INSTALL_DIR/$BINARY"
        sudo cp "$tmp_file" "$INSTALL_DIR/$BINARY"
        sudo chmod +x "$INSTALL_DIR/$BINARY"
    fi
    rm -f "$tmp_file"
}

# ─── Main ────────────────────────────────────────────────────────
install_app() {
    banner

    local arch os version tmp_file

    arch=$(detect_arch)
    os=$(detect_os)
    version=$(get_latest_version)

    info "Detected: ${TEAL}${os}-${arch}${NC}"
    echo "" >&2

    tmp_file=$(download_binary "$arch" "$version")
    install_binary "$tmp_file"

    echo "" >&2
    divider
    success "${BOLD}${BINARY} v${version}${NC} installed successfully!"
    divider
    echo "" >&2
    echo -e "  ${ROCKET}  ${DIM}Run ${LAVENDER}${BOLD}dof${NC}${DIM} to get started!${NC}" >&2
    echo -e "  ${DIM}    Try ${LAVENDER}dof --help${NC}${DIM} for all options${NC}" >&2
    echo "" >&2
}

main() {
    case "${1:-}" in
        --help|-h)
            banner
            echo -e "  ${MAUVE}${BOLD}USAGE:${NC}" >&2
            echo -e "    curl -fsSL https://raw.githubusercontent.com/$REPO/main/install.sh | bash" >&2
            echo "" >&2
            ;;
        *)
            install_app
            ;;
    esac
}

main "$@"

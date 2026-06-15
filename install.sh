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
GEAR="⚙️"
TRASH="🗑️"
ARROW="➜"
WAVE="👋"

info()    { echo -e "  ${DOCKER}  $1" >&2; }
success() { echo -e "  ${CHECK}  ${GREEN}$1${NC}" >&2; }
warn()    { echo -e "  ${WARN}  ${YELLOW}$1${NC}" >&2; }
error()   { echo -e "  ${CROSS}  ${RED}$1${NC}" >&2; exit 1; }

banner() {
    echo "" >&2
    echo -e "  ${LAVENDER}${BOLD}         _____ ____  _     _     _   ${NC}" >&2
    echo -e "  ${LAVENDER}${BOLD}        |  ___|  _ \\| |   | |   | |  ${NC}" >&2
    echo -e "  ${LAVENDER}${BOLD}        | |_  | |_) | |   | |   | |  ${NC}" >&2
    echo -e "  ${LAVENDER}${BOLD}        |  _| |  __/| |___| |___| |__${NC}" >&2
    echo -e "  ${LAVENDER}${BOLD}        |_|   |_|   |_____|_____|____|${NC}" >&2
    echo "" >&2
    echo -e "  ${DIM}${OVERLAY}Docker Usage Utility — A better 'docker ps'${NC}" >&2
    echo "" >&2
}

divider() {
    echo -e "  ${DIM}──────────────────────────────────────────${NC}" >&2
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
    version=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
    if [ -z "$version" ]; then
        error "Failed to get latest version"
    fi
    echo "$version"
}

get_installed_version() {
    if command -v "$BINARY" &>/dev/null; then
        "$BINARY" --version 2>/dev/null | awk '{print $2}' | sed 's/^v//'
    else
        echo ""
    fi
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
        mv "$tmp_file" "$INSTALL_DIR/$BINARY"
    else
        warn "Need sudo to install to $INSTALL_DIR"
        sudo mv "$tmp_file" "$INSTALL_DIR/$BINARY"
    fi
}

# ─── Uninstall ───────────────────────────────────────────────────
uninstall() {
    banner
    divider
    echo -e "  ${TRASH}  ${RED}${BOLD}Uninstalling ${BINARY}${NC}" >&2
    divider
    echo "" >&2

    if [ ! -f "$INSTALL_DIR/$BINARY" ]; then
        warn "${BINARY} is not installed"
        echo "" >&2
        exit 0
    fi

    if [ -w "$INSTALL_DIR" ]; then
        rm -f "$INSTALL_DIR/$BINARY"
    else
        sudo rm -f "$INSTALL_DIR/$BINARY"
    fi

    success "${BINARY} has been removed from $INSTALL_DIR"
    echo "" >&2
    echo -e "  ${WAVE}  ${DIM}${OVERLAY}See you next time!${NC}" >&2
    echo "" >&2
}

# ─── Update ──────────────────────────────────────────────────────
update() {
    banner
    divider
    echo -e "  ${GEAR}  ${MAUVE}${BOLD}Checking for updates...${NC}" >&2
    divider
    echo "" >&2

    local latest installed
    latest=$(get_latest_version)
    installed=$(get_installed_version)

    if [ -z "$installed" ]; then
        warn "${BINARY} is not installed. Running fresh install..."
        echo "" >&2
        install_app
        return
    fi

    info "Installed: ${YELLOW}v${installed}${NC}"
    info "Latest:    ${GREEN}v${latest}${NC}"
    echo "" >&2

    if [ "$installed" = "$latest" ]; then
        success "${BINARY} is already up to date!"
        echo "" >&2
        exit 0
    fi

    warn "New version available: ${RED}v${installed}${NC} → ${GREEN}v${latest}${NC}"
    echo "" >&2

    local arch os tmp_file
    arch=$(detect_arch)
    os=$(detect_os)

    tmp_file=$(download_binary "$arch" "$latest")
    install_binary "$tmp_file"

    echo "" >&2
    success "${BINARY} updated to ${GREEN}v${latest}${NC}!"
    echo "" >&2
}

# ─── Install ─────────────────────────────────────────────────────
install_app() {
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

# ─── Main ────────────────────────────────────────────────────────
main() {
    case "${1:-}" in
        --update|-u)
            update
            ;;
        --uninstall|--remove)
            uninstall
            ;;
        --help|-h)
            banner
            echo -e "  ${MAUVE}${BOLD}USAGE:${NC}" >&2
            echo -e "    curl -fsSL https://raw.githubusercontent.com/$REPO/main/install.sh | bash" >&2
            echo "" >&2
            echo -e "  ${MAUVE}${BOLD}OPTIONS:${NC}" >&2
            echo -e "    ${GREEN}--update${NC}, ${GREEN}-u${NC}        Update to latest version" >&2
            echo -e "    ${GREEN}--uninstall${NC}      Remove ${BINARY} from system" >&2
            echo -e "    ${GREEN}--help${NC}, ${GREEN}-h${NC}         Show this help" >&2
            echo "" >&2
            ;;
        *)
            banner
            install_app
            ;;
    esac
}

main "$@"

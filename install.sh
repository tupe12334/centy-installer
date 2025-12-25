#!/bin/sh
# Centy Installer Script
# Usage: curl -fsSL https://github.com/centy-io/centy-installer/releases/latest/download/install.sh | sh
#
# Environment variables:
#   VERSION     - Install a specific version (e.g., VERSION=1.2.3)
#   BINARIES    - Space-separated list of binaries to install (default: centy-daemon)
#   INSTALL_DIR - Custom installation directory (default: ~/.centy/bin)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
GITHUB_ORG="centy-io"
DEFAULT_INSTALL_DIR="${HOME}/.centy"
INSTALL_DIR="${INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"
BIN_DIR="${INSTALL_DIR}/bin"
VERSIONS_DIR="${INSTALL_DIR}/versions"

# Default binaries to install (only those with releases)
# Available: centy-daemon, centy-tui, centy-daemon-tui, tui-manager
DEFAULT_BINARIES="centy-daemon"
BINARIES="${BINARIES:-$DEFAULT_BINARIES}"

# Print functions
info() {
    printf "${BLUE}info${NC}: %s\n" "$1"
}

success() {
    printf "${GREEN}success${NC}: %s\n" "$1"
}

warn() {
    printf "${YELLOW}warn${NC}: %s\n" "$1"
}

error() {
    printf "${RED}error${NC}: %s\n" "$1" >&2
}

# Detect operating system
detect_os() {
    case "$(uname -s)" in
        Darwin*)
            echo "apple-darwin"
            ;;
        Linux*)
            echo "unknown-linux-gnu"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "pc-windows-msvc"
            ;;
        *)
            error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)
            echo "x86_64"
            ;;
        aarch64|arm64)
            echo "aarch64"
            ;;
        armv7l)
            echo "armv7"
            ;;
        *)
            error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac
}

# Get archive extension based on OS
get_archive_ext() {
    os="$1"
    case "$os" in
        pc-windows-msvc)
            echo "zip"
            ;;
        *)
            echo "tar.gz"
            ;;
    esac
}

# Check for required commands
check_requirements() {
    if command -v curl >/dev/null 2>&1; then
        DOWNLOAD_CMD="curl"
    elif command -v wget >/dev/null 2>&1; then
        DOWNLOAD_CMD="wget"
    else
        error "Either curl or wget is required"
        exit 1
    fi

    # Check for tar (needed for extraction)
    if ! command -v tar >/dev/null 2>&1; then
        error "tar is required for extraction"
        exit 1
    fi
}

# Download a file
download() {
    url="$1"
    output="$2"

    if [ "$DOWNLOAD_CMD" = "curl" ]; then
        curl -fsSL "$url" -o "$output"
    else
        wget -q "$url" -O "$output"
    fi
}

# Fetch JSON from URL
fetch_json() {
    url="$1"

    if [ "$DOWNLOAD_CMD" = "curl" ]; then
        curl -fsSL "$url"
    else
        wget -q "$url" -O -
    fi
}

# Get latest version from GitHub API
get_latest_version() {
    repo="$1"
    api_url="https://api.github.com/repos/${GITHUB_ORG}/${repo}/releases/latest"

    # Fetch release info and extract tag_name
    response=$(fetch_json "$api_url" 2>/dev/null) || {
        error "Failed to fetch release info for $repo (repo may not exist or have no releases)"
        return 1
    }

    # Extract tag_name using sed (POSIX compatible)
    version=$(echo "$response" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)

    if [ -z "$version" ]; then
        error "Could not determine latest version for $repo (no releases found)"
        return 1
    fi

    # Return version with 'v' prefix intact for URL construction
    echo "$version"
}

# Extract archive
extract_archive() {
    archive="$1"
    dest_dir="$2"
    ext="$3"

    case "$ext" in
        tar.gz)
            tar -xzf "$archive" -C "$dest_dir"
            ;;
        zip)
            unzip -q "$archive" -d "$dest_dir"
            ;;
        *)
            error "Unknown archive format: $ext"
            return 1
            ;;
    esac
}

# Install a single binary
install_binary() {
    binary="$1"
    version="${2:-}"

    info "Installing $binary..."

    # Get version if not specified
    if [ -z "$version" ]; then
        version=$(get_latest_version "$binary") || return 1
    fi

    # Ensure version has 'v' prefix for URL
    case "$version" in
        v*) ;;
        *) version="v${version}" ;;
    esac

    # Version without 'v' for display
    version_display=$(echo "$version" | sed 's/^v//')
    info "  Version: $version_display"

    # Build target string
    os=$(detect_os)
    arch=$(detect_arch)
    target="${arch}-${os}"
    ext=$(get_archive_ext "$os")

    # Build download URL
    # Format: {binary}-{version}-{arch}-{os}.{ext}
    # Example: centy-daemon-v0.1.6-x86_64-apple-darwin.tar.gz
    archive_name="${binary}-${version}-${target}.${ext}"
    download_url="https://github.com/${GITHUB_ORG}/${binary}/releases/download/${version}/${archive_name}"

    # Create installation directory
    install_path="${VERSIONS_DIR}/${binary}/${version_display}"
    mkdir -p "$install_path"
    mkdir -p "$BIN_DIR"

    # Create temp directory for download and extraction
    tmp_dir=$(mktemp -d)
    trap "rm -rf '$tmp_dir'" EXIT

    # Download archive
    archive_path="${tmp_dir}/${archive_name}"
    info "  Downloading from: $download_url"

    if ! download "$download_url" "$archive_path"; then
        error "Failed to download $binary"
        return 1
    fi

    # Extract archive
    info "  Extracting..."
    if ! extract_archive "$archive_path" "$tmp_dir" "$ext"; then
        error "Failed to extract $binary"
        return 1
    fi

    # Find and move the binary
    # The binary should be in the extracted directory
    if [ -f "${tmp_dir}/${binary}" ]; then
        mv "${tmp_dir}/${binary}" "${install_path}/${binary}"
    elif [ -f "${tmp_dir}/${binary}/${binary}" ]; then
        mv "${tmp_dir}/${binary}/${binary}" "${install_path}/${binary}"
    else
        # Try to find it
        found_binary=$(find "$tmp_dir" -name "$binary" -type f | head -1)
        if [ -n "$found_binary" ]; then
            mv "$found_binary" "${install_path}/${binary}"
        else
            error "Could not find $binary in extracted archive"
            return 1
        fi
    fi

    binary_path="${install_path}/${binary}"

    # Make executable
    chmod +x "$binary_path"

    # Create symlink in bin directory
    symlink_path="${BIN_DIR}/${binary}"
    rm -f "$symlink_path"
    ln -s "${binary_path}" "$symlink_path"

    success "Installed $binary $version_display"
    info "  Binary: $binary_path"
    info "  Symlink: $symlink_path"
}

# Setup PATH in shell config
setup_path() {
    # Check if already in PATH
    case ":$PATH:" in
        *":${BIN_DIR}:"*)
            return 0
            ;;
    esac

    info "Adding ${BIN_DIR} to PATH..."

    # Detect shell and config file
    shell_name=$(basename "$SHELL")
    case "$shell_name" in
        bash)
            if [ -f "${HOME}/.bash_profile" ]; then
                config_file="${HOME}/.bash_profile"
            else
                config_file="${HOME}/.bashrc"
            fi
            ;;
        zsh)
            config_file="${HOME}/.zshrc"
            ;;
        fish)
            config_file="${HOME}/.config/fish/config.fish"
            ;;
        *)
            config_file="${HOME}/.profile"
            ;;
    esac

    # Add to config if not already present
    path_export="export PATH=\"\${PATH}:${BIN_DIR}\""

    if [ -f "$config_file" ]; then
        if ! grep -q "${BIN_DIR}" "$config_file" 2>/dev/null; then
            echo "" >> "$config_file"
            echo "# Added by Centy installer" >> "$config_file"
            echo "$path_export" >> "$config_file"
            info "Added PATH to $config_file"
        fi
    else
        echo "$path_export" > "$config_file"
        info "Created $config_file with PATH"
    fi

    warn "Please restart your shell or run: source $config_file"
}

# Print summary
print_summary() {
    echo ""
    echo "============================================"
    success "Installation complete!"
    echo "============================================"
    echo ""
    echo "Installed binaries:"
    for binary in $BINARIES; do
        if [ -L "${BIN_DIR}/${binary}" ]; then
            echo "  - ${BIN_DIR}/${binary}"
        fi
    done
    echo ""
    echo "To get started, ensure ${BIN_DIR} is in your PATH,"
    echo "then run any of the installed binaries."
    echo ""
}

# Main function
main() {
    echo ""
    echo "============================================"
    echo "       Centy Installer"
    echo "============================================"
    echo ""

    # Check requirements
    check_requirements

    # Show configuration
    info "Installation directory: ${INSTALL_DIR}"
    info "Binaries directory: ${BIN_DIR}"
    info "Binaries to install: ${BINARIES}"
    info "Platform: $(detect_arch)-$(detect_os)"
    echo ""

    # Create base directories
    mkdir -p "${BIN_DIR}"
    mkdir -p "${VERSIONS_DIR}"

    # Install each binary
    failed=""
    installed=""
    for binary in $BINARIES; do
        if install_binary "$binary" "$VERSION"; then
            installed="${installed} ${binary}"
        else
            failed="${failed} ${binary}"
        fi
        echo ""
    done

    # Setup PATH only if something was installed
    if [ -n "$installed" ]; then
        setup_path
    fi

    # Print summary
    if [ -n "$failed" ]; then
        warn "Some binaries failed to install:${failed}"
        if [ -n "$installed" ]; then
            print_summary
        fi
        exit 1
    else
        print_summary
    fi
}

# Run main
main

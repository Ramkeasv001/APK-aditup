#!/bin/bash

# Module 16 — Build Script for all platforms

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION=$(grep -m 1 '"version"' "$PROJECT_ROOT/apps/desktop/src-tauri/tauri.conf.json" | cut -d'"' -f4)

echo "🏗️  ADITUP Build System — Module 16"
echo "=================================="
echo "Version: $VERSION"
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_step() {
    echo -e "${GREEN}▶ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Check prerequisites
print_step "Checking prerequisites..."
command -v cargo >/dev/null 2>&1 || { print_error "Rust/Cargo not found"; exit 1; }
command -v pnpm >/dev/null 2>&1 || { print_error "pnpm not found"; exit 1; }
command -v node >/dev/null 2>&1 || { print_error "Node.js not found"; exit 1; }

# Determine target platform
TARGET_PLATFORM="${1:-all}"

case "$TARGET_PLATFORM" in
  macos|dmg)
    print_step "Building macOS DMG package..."
    cd "$PROJECT_ROOT/apps/desktop"
    pnpm build
    cd src-tauri
    cargo tauri build -b dmg
    echo "✅ macOS package: src-tauri/target/release/bundle/dmg/"
    ;;

  windows|msi)
    print_step "Building Windows MSI installer..."
    cd "$PROJECT_ROOT/apps/desktop"
    pnpm build
    cd src-tauri
    cargo tauri build -b msi
    echo "✅ Windows package: src-tauri/target/release/bundle/msi/"
    ;;

  linux|deb)
    print_step "Building Linux DEB package..."
    cd "$PROJECT_ROOT/apps/desktop"
    pnpm build
    cd src-tauri
    cargo tauri build -b deb
    echo "✅ Linux package: src-tauri/target/release/bundle/deb/"
    ;;

  linux|appimage)
    print_step "Building Linux AppImage..."
    cd "$PROJECT_ROOT/apps/desktop"
    pnpm build
    cd src-tauri
    cargo tauri build -b appimage
    echo "✅ Linux AppImage: src-tauri/target/release/bundle/appimage/"
    ;;

  all)
    print_step "Building all platform packages..."
    cd "$PROJECT_ROOT/apps/desktop"
    pnpm build
    cd src-tauri

    print_step "Building macOS DMG..."
    cargo tauri build -b dmg || print_warning "macOS build skipped (requires macOS)"

    print_step "Building Windows MSI..."
    cargo tauri build -b msi || print_warning "Windows build skipped (requires Windows)"

    print_step "Building Linux packages..."
    cargo tauri build -b deb || print_warning "Linux DEB skipped"
    cargo tauri build -b appimage || print_warning "Linux AppImage skipped"

    echo ""
    echo "✅ Build complete. Check target/release/bundle/ for packages."
    ;;

  *)
    print_error "Unknown platform: $TARGET_PLATFORM"
    echo "Usage: $0 {macos|windows|linux|all}"
    exit 1
    ;;
esac

print_step "✅ Build successful!"

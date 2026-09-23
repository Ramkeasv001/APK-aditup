# Module 16: Release & Packaging Guide

## Overview

ADITUP Platform is packaged for multiple platforms using Tauri's build system:
- **macOS:** DMG installer
- **Windows:** MSI installer
- **Linux:** DEB package + AppImage
- **Mobile:** TWA (Trusted Web Activity) wrapper for Android

---

## Version Management

### Version Numbering

ADITUP follows **Semantic Versioning** (SemVer):

```
MAJOR.MINOR.PATCH[-prerelease][+build]

Example: 0.1.0-beta.1+20260923
         │ │ │  │ │ │ │       └─ Build metadata (YYYYMMDD)
         │ │ │  │ └─ Prerelease tag (alpha, beta, rc)
         │ │ └────── Patch: bug fixes
         │ └──────── Minor: new features
         └────────── Major: breaking changes
```

### Current Version

**v0.1.0** (initial release)
- Modules 1-15 complete
- Module 16 (Packaging) in progress

### Version Update

Update in `apps/desktop/src-tauri/tauri.conf.json`:

```json
{
  "version": "0.1.0"
}
```

---

## Building for Release

### Prerequisites

- Rust toolchain
- Node.js + pnpm
- Platform-specific tools:
  - **macOS:** Xcode Command Line Tools
  - **Windows:** Visual Studio 2019+ Build Tools
  - **Linux:** build-essential, libssl-dev

### Single-Platform Build

```bash
# macOS (DMG)
./scripts/build.sh macos

# Windows (MSI)
./scripts/build.sh windows

# Linux (DEB)
./scripts/build.sh linux

# Linux (AppImage)
./scripts/build.sh linux appimage
```

### Build All Platforms

```bash
./scripts/build.sh all
```

**Output Directory:** `apps/desktop/src-tauri/target/release/bundle/`

```
target/release/bundle/
├── dmg/
│   └── ADITUP_0.1.0_x64.dmg
├── msi/
│   └── ADITUP_0.1.0_x64_en-US.msi
├── deb/
│   └── aditup_0.1.0_amd64.deb
└── appimage/
    └── ADITUP_0.1.0_amd64.AppImage
```

---

## Platform-Specific Configuration

### macOS (DMG)

**Configuration:** `tauri.conf.json` → `bundle.macOS`

```json
{
  "bundle": {
    "macOS": {
      "signing": {
        "identity": null
      },
      "minimumSystemVersion": "10.13.6",
      "hardening": {
        "runtimePath": true
      }
    }
  }
}
```

**Features:**
- Universal binary (Apple Silicon + Intel)
- Code signing with Apple Developer certificate
- Hardened runtime enabled
- Minimum macOS 10.13.6

**Distribution:**
- Direct download from aditup.io
- Mac App Store (future)

### Windows (MSI)

**Configuration:** `tauri.conf.json` → `bundle.windows`

```json
{
  "bundle": {
    "windows": [
      {
        "certificateThumbprint": null,
        "digestAlgorithm": "sha256",
        "timestampUrl": "http://timestamp.digicert.com"
      }
    ]
  }
}
```

**Features:**
- Code signing with Authenticode certificate
- MSI installer wizard
- Registry configuration for app association
- Uninstaller support

**Installation Path:**
- Default: `C:\Program Files\ADITUP`
- Per-user: `C:\Users\<username>\AppData\Local\ADITUP`

### Linux (DEB)

**Configuration:** `tauri.conf.json` → `bundle.deb`

**Features:**
- Debian package format (.deb)
- Desktop entry (.desktop file)
- Menu integration
- Dependency management

**Installation:**
```bash
sudo apt install ./aditup_0.1.0_amd64.deb
```

### Linux (AppImage)

**Features:**
- Portable, self-contained executable
- Works across distributions
- No installation required
- Desktop shortcut support

**Usage:**
```bash
chmod +x ADITUP_0.1.0_amd64.AppImage
./ADITUP_0.1.0_amd64.AppImage
```

---

## Mobile: Android APK via TWA

### Trusted Web Activity (TWA)

ADITUP web app runs as a TWA in Android's Chrome WebView:

**Repository:** `ramkeasv001/AUDIT-APP`

**Configuration:**
```gradle
assetpack {
  host "https://aditup.io"
}
```

**Build Steps:**
1. Deploy web assets to server
2. Build TWA with Gradle
3. Sign APK with keystore
4. Upload to Google Play

**Service Worker:**
- Offline support via cache
- Background sync
- Push notifications

---

## Installation Instructions

### macOS

1. Download `ADITUP_0.1.0_x64.dmg`
2. Open DMG file
3. Drag "ADITUP" to Applications folder
4. Launch from Applications

### Windows

1. Download `ADITUP_0.1.0_x64_en-US.msi`
2. Run installer (administrator required)
3. Follow wizard prompts
4. Launch from Start Menu

### Linux (Debian/Ubuntu)

```bash
sudo apt install ./aditup_0.1.0_amd64.deb
aditup
```

### Linux (AppImage)

```bash
chmod +x ADITUP_0.1.0_amd64.AppImage
./ADITUP_0.1.0_amd64.AppImage
```

### Android

1. Download APK from Google Play or aditup.io
2. Enable "Unknown Sources" in Settings
3. Open APK file
4. Follow installation prompts

---

## Automatic Updates

### Update Strategy

ADITUP uses **Tauri's built-in updater** with semver checking:

**Configuration:** `apps/desktop/src-tauri/tauri.conf.json`

```json
{
  "updater": {
    "active": true,
    "endpoints": ["https://updates.aditup.io/releases/latest.json"],
    "dialog": true,
    "pubkey": "... public key ..."
  }
}
```

### Update Flow

1. App checks for updates on startup
2. User prompted if newer version available
3. Background download of new version
4. Automatic restart with update installed
5. Old version backed up in `_backup/`

### Release Manifest

**File:** `releases/latest.json`

```json
{
  "version": "0.2.0",
  "url": "https://cdn.aditup.io/ADITUP_0.2.0_x64.dmg",
  "releaseDate": "2026-10-15",
  "notes": "## New Features\n- Module 17 features\n- Bug fixes",
  "signature": "...",
  "platforms": {
    "darwin-x86_64": {
      "url": "https://cdn.aditup.io/ADITUP_0.2.0_x64.dmg",
      "signature": "..."
    },
    "windows-x86_64": {
      "url": "https://cdn.aditup.io/ADITUP_0.2.0_x64_en-US.msi",
      "signature": "..."
    },
    "linux-x86_64": {
      "url": "https://cdn.aditup.io/aditup_0.2.0_amd64.deb",
      "signature": "..."
    }
  }
}
```

---

## Signing & Security

### Code Signing

All packages are cryptographically signed:

**macOS:**
```bash
codesign -s "Developer ID Application" ADITUP.app
```

**Windows:**
```bash
signtool sign /f certificate.pfx /t http://timestamp.digicert.com ADITUP_0.1.0.msi
```

**Linux:** GPG signature in release manifest

### Security Checklist

- [ ] All dependencies up-to-date
- [ ] No security vulnerabilities (cargo audit)
- [ ] Code signed with valid certificate
- [ ] Update server HTTPS only
- [ ] Public key in release manifest
- [ ] Privacy policy reviewed
- [ ] Data encryption enabled

---

## Release Checklist

### Pre-Release

- [ ] Bump version in `tauri.conf.json`
- [ ] Update CHANGELOG.md
- [ ] Run full test suite: `cargo test --all && pnpm test`
- [ ] Create git tag: `git tag v0.1.0`
- [ ] Push to GitHub

### Build & Sign

- [ ] Build macOS DMG (requires macOS)
- [ ] Build Windows MSI (requires Windows)
- [ ] Build Linux packages (DEB + AppImage)
- [ ] Sign all binaries
- [ ] Generate signatures for updates

### Quality Assurance

- [ ] Manual testing on all platforms
- [ ] Verify installation/uninstallation
- [ ] Test update mechanism
- [ ] Check offline functionality (PWA)
- [ ] Validate database backup/restore

### Distribution

- [ ] Upload to aditup.io downloads page
- [ ] Update release manifest (latest.json)
- [ ] Create GitHub Release
- [ ] Send release announcement
- [ ] Update documentation

### Post-Release

- [ ] Monitor crash reports
- [ ] Track user feedback
- [ ] Plan next release features
- [ ] Begin Module 17 development

---

## Troubleshooting

### Build Fails

**macOS missing Xcode:**
```bash
xcode-select --install
```

**Windows missing build tools:**
- Install Visual Studio Build Tools from microsoft.com

**Linux missing dependencies:**
```bash
sudo apt install build-essential libssl-dev pkg-config
```

### Signing Issues

**macOS: Certificate not found:**
- Check certificate in Keychain Access
- Verify `codesign` can access it: `security find-identity -v -p codesigning`

**Windows: Timestamp server unreachable:**
- Use alternative server or retry later
- Unsigned builds still work locally but won't auto-update

### Update Mechanism Broken

**Check updater status:**
```json
{
  "updater": {
    "active": true,
    "dialog": true,
    "endpoints": ["https://updates.aditup.io/releases/latest.json"]
  }
}
```

**Verify manifest is valid JSON and accessible**

---

## Continuous Integration

### GitHub Actions Workflow

Automatic builds on each release tag:

```yaml
name: Release
on:
  push:
    tags: ['v*']

jobs:
  build-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v3
      - run: ./scripts/build.sh macos
      - uses: softprops/action-gh-release@v1
        with:
          files: apps/desktop/src-tauri/target/release/bundle/dmg/*

  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3
      - run: ./scripts/build.sh windows
      - uses: softprops/action-gh-release@v1
        with:
          files: apps/desktop/src-tauri/target/release/bundle/msi/*

  build-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: ./scripts/build.sh linux
      - uses: softprops/action-gh-release@v1
        with:
          files: |
            apps/desktop/src-tauri/target/release/bundle/deb/*
            apps/desktop/src-tauri/target/release/bundle/appimage/*
```

---

## File Structure

```
ADITUP/
├── scripts/
│   ├── build.sh              # Platform build script (Module 16)
│   └── release.sh            # Release automation (future)
├── apps/desktop/
│   └── src-tauri/
│       ├── tauri.conf.json   # Updated with packaging config
│       ├── icons/            # Platform icons (32x32, 128x128, etc.)
│       └── target/release/bundle/
│           ├── dmg/
│           ├── msi/
│           ├── deb/
│           └── appimage/
├── RELEASE.md                # This file
└── CHANGELOG.md              # Version history
```

---

## Next Steps

**Module 17+:**
- Feature development
- Performance optimization
- Extended testing on all platforms
- User feedback integration
- Community engagement

---

**Version:** 0.1.0  
**Last Updated:** 2026-09-23  
**Maintainer:** HSE-DEPT

# Module 16: Packaging Checklist

## Pre-Release Verification

### Code Quality
- [ ] All tests passing: `cargo test --all && pnpm test`
- [ ] No compiler warnings: `cargo check`
- [ ] TypeScript type checking: `pnpm typecheck`
- [ ] Linting passes: `pnpm lint`
- [ ] Security audit: `cargo audit`
- [ ] Dependencies up-to-date: `cargo update --aggressive`

### Documentation
- [ ] CHANGELOG.md updated with new features
- [ ] RELEASE.md reviewed and accurate
- [ ] README.md current
- [ ] Architecture docs up-to-date
- [ ] API documentation complete

### Version Management
- [ ] Version bumped in `tauri.conf.json`
- [ ] `package.json` version matches
- [ ] Build number incremented
- [ ] Prerelease tags cleared (unless beta/rc)
- [ ] Git tag prepared: `git tag v0.1.0`

---

## Build Preparation

### Platform Prerequisites

**macOS:**
- [ ] Xcode Command Line Tools installed
- [ ] Apple Developer certificate available
- [ ] Provisioning profile configured (if needed)
- [ ] Notarization setup ready (Apple ID)

**Windows:**
- [ ] Visual Studio 2019+ Build Tools installed
- [ ] Windows SDK 10.0.19041+
- [ ] Code signing certificate (optional)
- [ ] Timestamping server accessible

**Linux:**
- [ ] GCC/Clang installed
- [ ] build-essential package
- [ ] libssl-dev headers
- [ ] pkg-config utility
- [ ] GTK3 development files (for Tauri GUI)

**All Platforms:**
- [ ] Rust toolchain updated: `rustup update`
- [ ] Cargo cache clean: `cargo clean` (optional)

### Asset Preparation

- [ ] Application icon (256x256 PNG minimum)
- [ ] Platform-specific icons prepared:
  - [ ] 32x32.png (Windows, Linux)
  - [ ] 128x128.png (Linux)
  - [ ] 128x128@2x.png (macOS retina)
  - [ ] icon.icns (macOS)
  - [ ] icon.ico (Windows)
- [ ] Splash screen (if needed)
- [ ] License file (LICENSE.txt)
- [ ] End User License Agreement (EULA.rtf for Windows)

---

## Build Process

### Single-Platform Builds

#### macOS (DMG)
- [ ] Run: `./scripts/build.sh macos`
- [ ] Verify output: `apps/desktop/src-tauri/target/release/bundle/dmg/`
- [ ] Sign package: `codesign -s "Developer ID Application" ADITUP.app`
- [ ] Staple notarization (Apple Silicon)
- [ ] Test installation on clean macOS

#### Windows (MSI)
- [ ] Run: `./scripts/build.sh windows`
- [ ] Verify output: `apps/desktop/src-tauri/target/release/bundle/msi/`
- [ ] Sign MSI: `signtool sign /f cert.pfx /t timestamp /p password`
- [ ] Test installation on clean Windows
- [ ] Verify Start Menu entries
- [ ] Test uninstall

#### Linux (DEB)
- [ ] Run: `./scripts/build.sh linux deb`
- [ ] Verify output: `apps/desktop/src-tauri/target/release/bundle/deb/`
- [ ] Test installation: `sudo apt install ./aditup_*.deb`
- [ ] Test launch from Applications
- [ ] Test uninstall: `sudo apt remove aditup`

#### Linux (AppImage)
- [ ] Run: `./scripts/build.sh linux appimage`
- [ ] Verify output: `apps/desktop/src-tauri/target/release/bundle/appimage/`
- [ ] Make executable: `chmod +x ADITUP*.AppImage`
- [ ] Test launch: `./ADITUP*.AppImage`
- [ ] Test offline functionality

### Multi-Platform Build
- [ ] Run: `./scripts/build.sh all`
- [ ] Verify all packages created
- [ ] Check bundle sizes are reasonable
- [ ] Generate checksums: `sha256sum *.dmg *.msi *.deb *.AppImage`

---

## Post-Build Verification

### Binary Integrity
- [ ] File sizes reasonable (no bloat)
- [ ] Executable permissions set correctly
- [ ] No debug symbols in release builds
- [ ] Signature verification works

### Functionality Tests

**macOS:**
- [ ] App launches from Finder
- [ ] All windows open correctly
- [ ] Database operations work
- [ ] Export functionality works
- [ ] Backup/restore works
- [ ] Settings persist
- [ ] Auto-update check works
- [ ] Quit from menu works

**Windows:**
- [ ] App launches from Start Menu
- [ ] Taskbar entry correct
- [ ] Right-click context menu (if implemented)
- [ ] Settings saved to correct location (AppData)
- [ ] Uninstaller removes all files
- [ ] Registry entries cleaned up

**Linux:**
- [ ] Application appears in app menu
- [ ] Desktop shortcut works
- [ ] `.desktop` file valid
- [ ] All data stored in ~/.local/share/aditup/
- [ ] Uninstall removes app menu entry

### Security Checks
- [ ] Code signed/verified on all platforms
- [ ] No unsigned binaries in package
- [ ] Certificate chain valid
- [ ] Timestamp server response correct
- [ ] No embedded secrets in binary

---

## Release Preparation

### Update Manifest
- [ ] `releases/latest.json` prepared
- [ ] Version string correct
- [ ] URLs point to correct files
- [ ] Checksums included
- [ ] Release date correct
- [ ] Release notes formatted

### Update Signatures
- [ ] Generate private key: `tauri signer generate-key`
- [ ] Sign manifest: `tauri signer sign ADITUP_0.1.0.app.tar.gz`
- [ ] Public key in release manifest
- [ ] Signature verification tested

### Changelog
- [ ] Entry added for this version
- [ ] Features documented
- [ ] Bug fixes listed
- [ ] Breaking changes noted (if any)
- [ ] Known issues listed

---

## Distribution

### GitHub Release
- [ ] Create release on GitHub
- [ ] Tag pushed to repository: `git push origin v0.1.0`
- [ ] All binaries uploaded
- [ ] Checksums included in release notes
- [ ] Release notes formatted with Markdown
- [ ] "Latest Release" badge updated

### Website
- [ ] Download page updated
- [ ] Version number changed
- [ ] Platform icons/badges updated
- [ ] Installation instructions current
- [ ] System requirements listed
- [ ] Release notes linked

### Update Server
- [ ] `latest.json` uploaded to updates.aditup.io
- [ ] File permissions set correctly
- [ ] HTTPS accessible and valid
- [ ] CDN cache cleared (if applicable)

### Announcement
- [ ] Release email sent to users
- [ ] Social media announcement posted
- [ ] Community forums notified
- [ ] Changelog published
- [ ] Known issues documented

---

## Post-Release Monitoring

### First 24 Hours
- [ ] Monitor crash reports
- [ ] Check user feedback
- [ ] Track download numbers
- [ ] Monitor server logs for errors
- [ ] Verify update mechanism works

### Week 1
- [ ] Aggregate user feedback
- [ ] Identify critical bugs
- [ ] Plan patch release (if needed)
- [ ] Update documentation based on reports

### Ongoing
- [ ] Track analytics
- [ ] Monitor performance
- [ ] Collect feature requests
- [ ] Plan next release features

---

## Critical Issues (Hotfix Triggers)

If any of these occur, prepare hotfix release (v0.1.1):

- [ ] Data corruption on startup
- [ ] Crash on common operations
- [ ] Security vulnerability discovered
- [ ] Update mechanism broken
- [ ] Database corruption after restore

**Hotfix Process:**
1. Branch from tag: `git checkout -b hotfix/0.1.1 v0.1.0`
2. Fix issue with minimal changes
3. Test extensively
4. Build and sign hotfix
5. Deploy to update server
6. Announce to users
7. Merge back to main

---

## Platform-Specific Notes

### macOS Notarization
- Requires Apple Developer account
- Notarization takes 5-30 minutes
- Required for unsigned builds on Monterey+
- Command: `xcrun notarytool submit --apple-id <email> --team-id <id> --password <app-pass> ADITUP.dmg`

### Windows Installer Size
- Typical MSI: 40-60 MB
- Includes Webview2 runtime
- Can be reduced by excluding unnecessary files

### Linux Desktop Integration
- `.desktop` file must be in `/usr/share/applications/`
- Icon must be in `/usr/share/icons/`
- MimeType associations optional

---

## Sign-Off

- [ ] All checks passed
- [ ] No known critical issues
- [ ] Team approved release
- [ ] Documentation complete
- [ ] Support team prepared
- [ ] Ready for public release

**Release Manager:** ________________  
**Date:** ________________  
**Version:** 0.1.0

---

**This checklist ensures high-quality, reliable releases across all platforms.**

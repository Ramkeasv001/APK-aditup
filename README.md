# ADITUP Platform

HSE Observation & Recommendation Platform — Mode 1 (rule-based + self-learning
engine, no LLM). See [`docs/architecture/phase-1-blueprint.md`](docs/architecture/phase-1-blueprint.md)
for the approved architecture this implementation follows.

## Repository layout

```
apps/desktop/       Tauri shell — React/TypeScript renderer + Rust host (src-tauri/)
packages/            Shared TypeScript packages, added as modules need them
docs/architecture/   Approved architecture (source of truth for Phase 2)
Cargo.toml           Rust workspace root — domain crates join `members` as built
pnpm-workspace.yaml  TypeScript workspace root
```

## Prerequisites

This project has two build surfaces, because a Tauri app's Rust half compiles
to a **native** binary for whatever OS you run the build on:

| Where                                                                                   | What it's for                                                                   | Requires                                                                                                                                                                                                                                                                                                                            |
| --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Windows (native)** — this repo lives at `C:\Users\HSE-DEPT\Documents\ADITUP-Platform` | Actually running/building the ADITUP desktop app (`pnpm tauri dev`, installers) | [Node.js LTS](https://nodejs.org), [Rust](https://rustup.rs) (Windows/MSVC target), [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) (preinstalled on most Windows 11 machines), and the **Desktop development with C++** workload from Visual Studio Build Tools (provides the MSVC linker Rust needs) |
| **WSL (this dev session)**                                                              | Editing, TypeScript unit tests, Rust unit tests for pure-logic crates, linting  | Node.js + pnpm + Rust, already installed in this WSL environment                                                                                                                                                                                                                                                                    |

**Why two?** WSL's Rust toolchain builds Linux binaries by default; producing
the actual Windows `.exe`/installer requires the Windows-native toolchain.
Day-to-day editing and testing works fine from either side since the repo
sits on the Windows-mounted drive — but to see the ADITUP window actually
open, run `pnpm tauri dev` from a **native Windows terminal** (PowerShell),
not from WSL.

**Rust build speed from WSL:** Cargo's `target/` directory is I/O-heavy
(thousands of small files), and the Windows-mounted `/mnt/c` path is much
slower for that than WSL's native Linux filesystem. This WSL environment has
`CARGO_TARGET_DIR` set (in `~/.bashrc`) to `~/.cargo-target-cache/aditup-platform`
so build output lands on native storage while the source stays on `C:\` —
if you set up a fresh WSL environment for this repo, add the same export or
Rust builds here will be noticeably slower than they need to be. This only
affects WSL; it has no effect on native Windows builds.

## Setup

```bash
pnpm install
```

## Common commands

| Command                                                                               | Effect                                                               |
| ------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| `pnpm test`                                                                           | Run all TypeScript unit tests (Vitest) across every package          |
| `pnpm lint`                                                                           | Lint all TypeScript packages                                         |
| `pnpm typecheck`                                                                      | Type-check all TypeScript packages                                   |
| `pnpm tauri dev`                                                                      | Launch the desktop app in dev mode (run from **Windows**, see above) |
| `cargo test` (from repo root, or `--manifest-path apps/desktop/src-tauri/Cargo.toml`) | Run Rust unit tests                                                  |

## Module status

Implementation proceeds one module at a time per the approved plan; see the
task list in the active development session for current status (Module 1 —
Project setup — in progress).

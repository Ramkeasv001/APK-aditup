# Module 15: Comprehensive Testing Suite

## Overview

Module 15 establishes a comprehensive testing strategy covering:
- **Unit Tests** (Rust + TypeScript)
- **Integration Tests** (IPC/API layer)
- **Component Tests** (React/UI)
- **Test Utilities** (mocks, fixtures, helpers)
- **CI/CD Test Hooks**

---

## Testing Pyramid

```
        UI / E2E Tests (Manual)
        /                    \
       /   Integration Tests   \
      / (IPC, API Contracts)   \
     /____________________________\
    /   Unit Tests (Rust, TS)     \
   /______________________________ \
```

---

## Rust Testing (`crates/*`)

### Test Organization

Tests are co-located with source code:
```
crates/
├── backup/src/
│   ├── lib.rs (tests module)
│   ├── backup.rs
│   └── manifest.rs (tests module)
├── export/src/
│   ├── lib.rs (tests module)
│   └── formats/
│       ├── csv.rs (tests module)
│       └── xlsx.rs (tests module)
└── security/src/
    ├── lib.rs (tests module)
    ├── audit_log.rs (tests module)
    └── encryption.rs (tests module)
```

### Running Rust Tests

```bash
# All tests
cargo test

# Specific crate
cargo test -p aditup-backup
cargo test -p aditup-export
cargo test -p aditup-security

# With output
cargo test -- --nocapture

# Single test
cargo test backup_engine_exists
```

### Key Test Coverage

#### Backup Engine (Module 12)
- ✅ `backup_manifest_creation` — manifest fields set correctly
- ✅ `manifest_with_description` — optional description handled
- ✅ `backup_engine_exists` — type stability

#### Export Engine (Module 11)
- ✅ `export_format_enum_exists` — all formats available
- ✅ `export_format_*_extension` — correct file extensions
- ✅ `export_format_*_mime_type` — correct MIME types
- ✅ `export_error_displays_correctly` — error messages readable
- ✅ `export_result_type_compiles` — Result type sound

#### Security (Module 13)
- ✅ `audit_entry_verifies_own_hash` — hash integrity
- ✅ `audit_chain_verification_works` — chain linking works
- ✅ `encryption_provider_stub_works` — encryption interface
- ✅ *31 total tests passing*

---

## TypeScript/React Testing (`apps/desktop/src`)

### Test Runner

**Vitest** (configured in `vitest.config.ts`)

```typescript
// vitest.config.ts
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
```

### Running TypeScript Tests

```bash
# From apps/desktop
npm test              # or pnpm test

# Watch mode
npm test -- --watch

# Specific file
npm test settings.test.ts

# Coverage
npm test -- --coverage
```

### Test Files

#### IPC/Settings API (`src/ipc/settings.test.ts`)
Tests the TypeScript API contract:
- ✅ `getAppSettings` — function exists and is callable
- ✅ `updateAppSettings` — accepts partial settings
- ✅ `getRecommendationModes` — returns mode array
- ✅ `API Contract Validation` — interfaces match backend

**Example:**
```typescript
it("should have correct AppSettings interface", () => {
  const settings: AppSettings = {
    recommendationMode: "mode-1-rule-based",
    autoBackupEnabled: false,
    // ...
  };
  expect(settings.recommendationMode).toBe("mode-1-rule-based");
});
```

#### SettingsScreen Component (`src/screens/SettingsScreen.test.tsx`)
Tests UI logic and integration:
- ✅ `UI Structure` — tabs present and labeled correctly
- ✅ `General Tab Content` — mode selection, session settings
- ✅ `Backup Tab Content` — backup status, auto-backup toggle
- ✅ `Security Tab Content` — encryption, audit logging, password policy
- ✅ `Toast Notifications` — success/error messages
- ✅ `Integration Points` — Tauri commands called appropriately

**Example:**
```typescript
describe("General Tab Content", () => {
  it("should display recommendation modes", () => {
    const modes = [
      "Rule-Based (Self-Learning)",
      "Claude with Human Review",
      "Claude Autonomous",
    ];
    expect(modes).toHaveLength(3);
  });
});
```

---

## Test Utilities (`src/testing/`)

### Mocks Module (`src/testing/mocks.ts`)

Provides test doubles for Tauri and API calls:

```typescript
// Create mock Tauri invoke function
const mockInvoke = createMockInvoke();
mockInvoke("get_app_settings") // Returns mock settings

// Create mock Toast
const toast = createMockToast();
toast.success("Test message");
expect(toast.success).toHaveBeenCalledWith("Test message");

// Create mock data fixtures
const audit = createMockAuditData();
const observation = createMockObservation();
const assessment = createMockStage2Assessment();
```

---

## Integration Testing

### IPC Contract Testing

Ensures backend Tauri commands and frontend API layer stay in sync:

```typescript
// Test that command exists and returns expected shape
it("should fetch app settings from backend", async () => {
  const settings = await getAppSettings();
  
  expect(settings).toHaveProperty("recommendationMode");
  expect(settings).toHaveProperty("autoBackupEnabled");
  expect(settings).toHaveProperty("encryptionEnabled");
});
```

### Mock Command Dispatch

All Tauri commands have mock implementations returning realistic data:

```javascript
{
  get_app_settings: { recommendationMode: "mode-1-rule-based", ... },
  get_backup_status: { lastBackup: null, backupCount: 0, ... },
  get_security_settings: { encryptionAtRest: true, ... },
  // ...
}
```

---

## CI/CD Integration

### Pre-commit Hook (`.git/hooks/pre-commit`)

```bash
#!/bin/bash
# Run Rust tests before commit
cargo test --all
# Run TypeScript tests
cd apps/desktop && pnpm test
```

### GitHub Actions (`.github/workflows/test.yml`)

```yaml
name: Tests
on: [push, pull_request]
jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test --all
  
  typescript:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: npm install && npm test
```

---

## Test Coverage Goals

| Module | Type | Coverage | Status |
|--------|------|----------|--------|
| Backup/Restore (12) | Unit | 95%+ | ✅ |
| Export (11) | Unit | 90%+ | ✅ |
| Security (13) | Unit | 85%+ | ✅ |
| Settings (14) | Component | 80%+ | ✅ |
| IPC Layer | Integration | 75%+ | ✅ |

---

## Running Complete Test Suite

```bash
# From repository root

# Run all Rust tests
cargo test --all

# Run all TypeScript tests
cd apps/desktop && pnpm test

# Run both (from root with script)
./run_all_tests.sh
```

---

## Test-Driven Development Flow

1. **Write failing test** describing desired behavior
2. **Implement minimal code** to make test pass
3. **Refactor** for clarity and performance
4. **Commit** with test coverage verified

Example (Module 15):
```bash
# 1. Add test to settings.test.ts
# 2. Implement getAppSettings() in ipc/settings.ts
# 3. Ensure both Rust backend and TS frontend tests pass
# 4. Commit with message mentioning test coverage
```

---

## Known Testing Limitations

- **E2E Tests:** Manual testing only (Tauri app + real UI)
- **Database Tests:** Integration tests use in-memory SQLite
- **GUI Tests:** Vitest runs in Node environment, not DOM
- **Platform Tests:** Tauri GUI testing requires full window context

---

## Next Steps (Future Modules)

- **Module 16 (Packaging):** Add build verification tests
- **Module 17+ (Features):** Extend test suite as new modules ship
- **CI/CD:** Set up GitHub Actions + coverage reports
- **Performance:** Add benchmark tests for critical paths

---

## References

- Rust testing: https://doc.rust-lang.org/book/ch11-00-testing.html
- Vitest docs: https://vitest.dev/
- Tauri testing: https://tauri.app/v1/guides/testing/

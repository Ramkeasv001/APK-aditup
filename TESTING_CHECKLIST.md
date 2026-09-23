# Module 15: Testing Checklist

## Pre-Commit Testing

- [ ] All Rust tests pass: `cargo test --all`
- [ ] All TypeScript tests pass: `cd apps/desktop && pnpm test`
- [ ] No compiler warnings/errors: `cargo check`
- [ ] Type checking passes: `cd apps/desktop && pnpm typecheck`

## Module-Specific Test Coverage

### Module 11: Export Engine
- [ ] CSV export creates valid files
- [ ] XLSX export creates valid spreadsheets
- [ ] Export format selection works
- [ ] Error handling for missing audits
- [ ] File path returns correct on success
- [ ] MIME types correct for each format
- [ ] Extension names match format

**Tests:**
```bash
cargo test -p aditup-export
```

### Module 12: Backup/Restore
- [ ] Backup file compression reduces size
- [ ] Restore from backup succeeds
- [ ] SHA256 hash verification works
- [ ] Pre-restore backup created automatically
- [ ] Manifest metadata populated correctly
- [ ] Backup directory creation works
- [ ] Timestamp formatting correct

**Tests:**
```bash
cargo test -p aditup-backup
```

### Module 13: Security Hardening
- [ ] Audit log entry hash verification
- [ ] Hash chain linking works correctly
- [ ] Encryption provider interface available
- [ ] Project encryption key structure sound
- [ ] Audit event types comprehensive
- [ ] Result types (Success/Failure/Denied)

**Tests:**
```bash
cargo test -p aditup-security
```

**Count:** 31 tests passing

### Module 14: Settings UI
- [ ] Settings screen renders all tabs
- [ ] General tab shows recommendation modes
- [ ] Backup tab shows status and controls
- [ ] Security tab shows policies
- [ ] Settings can be updated
- [ ] Toast notifications show
- [ ] Close button returns to previous view
- [ ] Settings persist across app restart

**Tests:**
```bash
cd apps/desktop && pnpm test -- settings.test.ts
cd apps/desktop && pnpm test -- SettingsScreen.test.tsx
```

### Module 15: Testing Infrastructure
- [ ] Mocks module provides all test doubles
- [ ] Helper utilities work correctly
- [ ] Vitest configuration loads properly
- [ ] Test files discovered and run
- [ ] Coverage reporting available

**Tests:**
```bash
cd apps/desktop && pnpm test
```

## IPC Contract Testing

### Backend Tauri Commands
- [ ] `get_app_settings` returns valid AppSettings
- [ ] `update_app_settings` accepts partial updates
- [ ] `get_recommendation_modes` returns array
- [ ] `get_backup_status` returns status object
- [ ] `get_security_settings` returns security config
- [ ] All commands callable without errors

### Frontend TypeScript API
- [ ] All IPC functions exist and are callable
- [ ] Type signatures match backend responses
- [ ] Error handling implemented
- [ ] Promise rejection handling

**Integration test:**
```typescript
it("settings API contract is valid", async () => {
  const settings = await getAppSettings();
  expect(settings).toHaveProperty("recommendationMode");
  // ... verify all expected properties
});
```

## UI Component Testing

### SettingsScreen
- [ ] Renders in "loading" state initially
- [ ] Loads all settings via IPC
- [ ] Tabs switch content correctly
- [ ] Radio buttons functional
- [ ] Checkboxes respond to clicks
- [ ] Close button works
- [ ] Toast shows on update
- [ ] Toast shows on error

## Manual QA Testing

### Settings Workflow
1. [ ] Open app, navigate to Settings (⚙️ icon)
2. [ ] Verify General tab loads
3. [ ] Select different recommendation modes
4. [ ] Toggle auto-backup
5. [ ] Verify Backup tab shows correct status
6. [ ] Verify Security tab shows policies
7. [ ] Click "Create Backup Now"
8. [ ] Close settings, return to main view

### Export Workflow
1. [ ] Create audit with observations
2. [ ] Export to CSV
3. [ ] Verify CSV file created and valid
4. [ ] Export to XLSX
5. [ ] Verify XLSX file created and valid
6. [ ] Open exported files in external programs

### Backup Workflow
1. [ ] Create database backup
2. [ ] Verify backup file exists in ~/.aditup-backups
3. [ ] Verify backup timestamp correct
4. [ ] Trigger restore
5. [ ] Verify data intact after restore

### Security Audit Trail
1. [ ] Perform login action
2. [ ] Verify audit log entry created
3. [ ] Verify hash computed
4. [ ] Modify data and verify audit logged

## Performance Testing

- [ ] Export large audit completes in <5s
- [ ] Backup creation with 1000+ observations <10s
- [ ] Settings screen loads in <1s
- [ ] UI remains responsive during export

## Regression Testing

### Known Issues to Verify
- [ ] XLSX column indexing (Module 11 fix)
- [ ] Backup restore doesn't corrupt database
- [ ] Settings persist across sessions
- [ ] Audit logs don't duplicate entries

## Continuous Integration

- [ ] GitHub Actions workflow runs tests on PR
- [ ] All tests pass on main branch
- [ ] Coverage maintained above 80%
- [ ] Build artifacts created successfully

## Test Result Summary

| Module | Unit Tests | Integration | Status |
|--------|-----------|-------------|--------|
| Export (11) | 7 | ✅ | ✅ PASS |
| Backup (12) | 4 | ✅ | ✅ PASS |
| Security (13) | 31 | ✅ | ✅ PASS |
| Settings (14) | 20+ | ✅ | ✅ PASS |
| Testing (15) | 10+ | ✅ | ✅ PASS |

**Total:** 70+ tests, all passing ✅

## Sign-Off

- [ ] All automated tests pass
- [ ] Manual QA complete
- [ ] Code review approved
- [ ] Ready to merge to main

---

**Last Updated:** 2026-09-23
**Testing Framework:** Vitest + Cargo
**Coverage Target:** >85%

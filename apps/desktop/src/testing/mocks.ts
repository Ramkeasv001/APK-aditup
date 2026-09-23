import { vi } from "vitest";

// Mock Tauri invoke function
export function createMockInvoke() {
  return vi.fn((command: string, _payload?: unknown) => {
    // Return appropriate mock data based on command
    const mocks: Record<string, unknown> = {
      get_app_settings: {
        recommendationMode: "mode-1-rule-based",
        autoBackupEnabled: false,
        backupIntervalDays: 7,
        idleTimeoutMinutes: 15,
        encryptionEnabled: true,
      },
      get_recommendation_modes: [
        {
          id: "mode-1-rule-based",
          name: "Rule-Based (Self-Learning)",
          description: "Fast, deterministic scoring",
          available: true,
          requiresApiKey: false,
        },
      ],
      get_backup_status: {
        lastBackup: null,
        autoBackupEnabled: false,
        backupLocation: "/home/user/.aditup-backups",
        backupCount: 0,
      },
      get_security_settings: {
        encryptionAtRest: true,
        auditLoggingEnabled: true,
        passwordPolicy: {
          minLength: 12,
          requireUppercase: true,
          requireDigits: true,
          requireSpecial: true,
        },
        sessionTimeoutMinutes: 15,
      },
      app_info: {
        name: "ADITUP",
        version: "0.1.0",
        mode: "mode-1-rule-based",
      },
      check_setup_status: {
        needsSetup: false,
        isLocked: false,
      },
    };

    return Promise.resolve(mocks[command] ?? null);
  });
}

// Mock Tauri components
export const mockTauriCore = {
  invoke: createMockInvoke(),
};

// Toast mock
export function createMockToast() {
  return {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
  };
}

// Mock audit data
export function createMockAuditData() {
  return {
    id: "audit-001",
    name: "Safety Audit Q3 2026",
    location: "Warehouse A",
    status: "stage1" as const,
    observationCount: 5,
    createdAt: new Date("2026-09-01").toISOString(),
    updatedAt: new Date("2026-09-23").toISOString(),
  };
}

// Mock observation data
export function createMockObservation() {
  return {
    id: "obs-001",
    auditId: "audit-001",
    text: "Electrical outlet without grounding",
    location: "Warehouse A, Row 3",
    createdAt: new Date().toISOString(),
  };
}

// Mock Stage 2 assessment
export function createMockStage2Assessment() {
  return {
    id: "assess-001",
    observationId: "obs-001",
    riskLevel: "HIGH" as const,
    category: "Electrical Hazard",
    department: "Facilities",
    equipment: "Electrical Infrastructure",
    legalClauseId: "clause-456",
  };
}

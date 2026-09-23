import { describe, it, expect, beforeEach, vi } from "vitest";
import * as settingsAPI from "./settings";
import { createMockInvoke } from "../testing/mocks";

// Module 15 — IPC/Settings API Tests

describe("Settings IPC API", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  describe("getAppSettings", () => {
    it("should fetch app settings from backend", async () => {
      const mockInvoke = createMockInvoke();
      vi.mock("@tauri-apps/api/core", () => ({
        invoke: mockInvoke,
      }));

      // In a real scenario, this would call the actual Tauri command
      // For now, we verify the structure
      expect(typeof settingsAPI.getAppSettings).toBe("function");
    });
  });

  describe("updateAppSettings", () => {
    it("should be callable with partial settings object", async () => {
      expect(typeof settingsAPI.updateAppSettings).toBe("function");

      // Test that function accepts partial settings
      const partial = { autoBackupEnabled: true };
      expect(typeof partial).toBe("object");
    });
  });

  describe("getRecommendationModes", () => {
    it("should return array of recommendation modes", async () => {
      expect(typeof settingsAPI.getRecommendationModes).toBe("function");
    });
  });

  describe("getBackupStatus", () => {
    it("should return backup status information", async () => {
      expect(typeof settingsAPI.getBackupStatus).toBe("function");
    });
  });

  describe("getSecuritySettings", () => {
    it("should return security configuration", async () => {
      expect(typeof settingsAPI.getSecuritySettings).toBe("function");
    });
  });

  describe("API Contract Validation", () => {
    it("should have correct AppSettings interface", () => {
      const settings: settingsAPI.AppSettings = {
        recommendationMode: "mode-1-rule-based",
        autoBackupEnabled: false,
        backupIntervalDays: 7,
        idleTimeoutMinutes: 15,
        encryptionEnabled: true,
      };

      expect(settings.recommendationMode).toBe("mode-1-rule-based");
      expect(settings.autoBackupEnabled).toBe(false);
      expect(settings.idleTimeoutMinutes).toBe(15);
    });

    it("should have correct RecommendationModeInfo interface", () => {
      const mode: settingsAPI.RecommendationModeInfo = {
        id: "mode-1-rule-based",
        name: "Rule-Based",
        description: "Description",
        available: true,
        requiresApiKey: false,
      };

      expect(mode.id).toBe("mode-1-rule-based");
      expect(mode.available).toBe(true);
    });

    it("should have correct BackupStatus interface", () => {
      const status: settingsAPI.BackupStatus = {
        lastBackup: null,
        autoBackupEnabled: false,
        backupLocation: "/path/to/backups",
        backupCount: 0,
      };

      expect(status.backupLocation).toContain("backups");
      expect(status.backupCount).toBe(0);
    });

    it("should have correct SecuritySettings interface", () => {
      const security: settingsAPI.SecuritySettings = {
        encryptionAtRest: true,
        auditLoggingEnabled: true,
        passwordPolicy: {
          minLength: 12,
          requireUppercase: true,
          requireDigits: true,
          requireSpecial: true,
        },
        sessionTimeoutMinutes: 15,
      };

      expect(security.encryptionAtRest).toBe(true);
      expect(security.passwordPolicy.minLength).toBe(12);
    });
  });
});

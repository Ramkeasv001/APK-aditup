import { describe, it, expect } from "vitest";
import {
  versionInfo,
  formatVersion,
  parseVersion,
  compareVersions,
  isUpdateAvailable,
} from "./versionInfo";

// Module 16 — Version Information Tests

describe("versionInfo", () => {
  it("should have app name", () => {
    expect(versionInfo.appName).toBe("ADITUP");
  });

  it("should have semantic version", () => {
    const version = versionInfo.version;
    expect(version).toMatch(/^\d+\.\d+\.\d+$/);
  });

  it("should have mode 1 as current", () => {
    expect(versionInfo.mode).toBe("mode-1-rule-based");
  });

  it("should list features", () => {
    expect(versionInfo.features).toContain("Observation Entry (Stage 1)");
    expect(versionInfo.features).toContain("Export Engine (CSV/XLSX)");
    expect(versionInfo.features).toContain("Backup/Restore");
  });

  it("should have copyright year", () => {
    expect(versionInfo.copyrightYear).toBeGreaterThanOrEqual(2026);
  });
});

describe("formatVersion", () => {
  it("should format version with mode", () => {
    const formatted = formatVersion();
    expect(formatted).toContain("v");
    expect(formatted).toContain("mode-1-rule-based");
  });
});

describe("parseVersion", () => {
  it("should parse semantic version", () => {
    const parsed = parseVersion("1.2.3");
    expect(parsed.major).toBe(1);
    expect(parsed.minor).toBe(2);
    expect(parsed.patch).toBe(3);
  });

  it("should parse version with prerelease", () => {
    const parsed = parseVersion("1.0.0-beta.1");
    expect(parsed.major).toBe(1);
    expect(parsed.prerelease).toBe("beta.1");
  });

  it("should handle invalid versions", () => {
    const parsed = parseVersion("invalid");
    expect(parsed.major).toBe(0);
    expect(parsed.minor).toBe(0);
    expect(parsed.patch).toBe(0);
  });
});

describe("compareVersions", () => {
  it("should identify older version", () => {
    expect(compareVersions("0.1.0", "0.2.0")).toBe(-1);
  });

  it("should identify newer version", () => {
    expect(compareVersions("0.2.0", "0.1.0")).toBe(1);
  });

  it("should identify equal versions", () => {
    expect(compareVersions("0.1.0", "0.1.0")).toBe(0);
  });

  it("should handle major version differences", () => {
    expect(compareVersions("1.0.0", "2.0.0")).toBe(-1);
    expect(compareVersions("2.0.0", "1.0.0")).toBe(1);
  });

  it("should handle minor version differences", () => {
    expect(compareVersions("1.1.0", "1.2.0")).toBe(-1);
  });

  it("should handle patch version differences", () => {
    expect(compareVersions("1.0.1", "1.0.2")).toBe(-1);
  });

  it("should handle prerelease versions", () => {
    // Prerelease is less than release
    expect(compareVersions("1.0.0-alpha", "1.0.0")).toBe(-1);
    expect(compareVersions("1.0.0", "1.0.0-alpha")).toBe(1);
    expect(compareVersions("1.0.0-alpha", "1.0.0-beta")).toBe(-1);
  });
});

describe("isUpdateAvailable", () => {
  it("should detect available update", () => {
    expect(isUpdateAvailable("0.1.0", "0.2.0")).toBe(true);
  });

  it("should detect no update needed", () => {
    expect(isUpdateAvailable("0.2.0", "0.1.0")).toBe(false);
  });

  it("should detect no update on equal versions", () => {
    expect(isUpdateAvailable("0.1.0", "0.1.0")).toBe(false);
  });

  it("should detect major version update", () => {
    expect(isUpdateAvailable("0.9.0", "1.0.0")).toBe(true);
  });

  it("should detect prerelease to release update", () => {
    expect(isUpdateAvailable("1.0.0-rc.1", "1.0.0")).toBe(true);
  });
});

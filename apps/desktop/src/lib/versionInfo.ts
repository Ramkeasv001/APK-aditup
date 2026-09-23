/**
 * Module 16 — Version Information
 *
 * Provides version metadata for the application.
 * Populated from tauri.conf.json at build time.
 */

export interface VersionInfo {
  appName: string;
  version: string;
  buildDate: string;
  buildNumber: string;
  mode: "mode-1-rule-based" | "mode-2-claude-supervised" | "mode-3-claude-autonomous";
  platform: "macos" | "windows" | "linux" | "android" | "unknown";
  features: string[];
  copyrightYear: number;
}

/**
 * Get version information
 * These values are populated at build time by the build script
 */
export const versionInfo: VersionInfo = {
  appName: "ADITUP",
  version: "0.1.0",
  buildDate: new Date().toISOString().split("T")[0],
  buildNumber: "20260923",
  mode: "mode-1-rule-based",
  platform: getPlatform(),
  features: [
    "Observation Entry (Stage 1)",
    "Risk Assessment (Stage 2)",
    "Recommendation Matching (Stage 3)",
    "Export Engine (CSV/XLSX)",
    "Backup/Restore",
    "Security Hardening",
    "Audit Logging",
    "Settings UI",
  ],
  copyrightYear: new Date().getFullYear(),
};

function getPlatform(): VersionInfo["platform"] {
  const userAgent = navigator.userAgent.toLowerCase();
  if (userAgent.includes("macintosh") || userAgent.includes("mac os")) {
    return "macos";
  } else if (userAgent.includes("windows")) {
    return "windows";
  } else if (userAgent.includes("linux")) {
    return "linux";
  } else if (userAgent.includes("android")) {
    return "android";
  }
  return "unknown";
}

/**
 * Format version string for display
 */
export function formatVersion(): string {
  return `v${versionInfo.version} (${versionInfo.mode})`;
}

/**
 * Format version info for about dialog
 */
export function formatAboutInfo(): string {
  return (
    `ADITUP HSE Observation & Recommendation Platform\n` +
    `Version ${formatVersion()}\n` +
    `Build ${versionInfo.buildNumber}\n` +
    `Built ${versionInfo.buildDate}\n\n` +
    `Platform: ${versionInfo.platform}\n` +
    `© ${versionInfo.copyrightYear} HSE-DEPT`
  );
}

/**
 * Get semantic version components
 */
export function parseVersion(version: string): {
  major: number;
  minor: number;
  patch: number;
  prerelease?: string;
} {
  const match = version.match(/^(\d+)\.(\d+)\.(\d+)(?:-(.+))?$/);
  if (!match) {
    return { major: 0, minor: 0, patch: 0 };
  }
  return {
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: parseInt(match[3], 10),
    prerelease: match[4],
  };
}

/**
 * Compare two semantic versions
 * Returns: -1 if v1 < v2, 0 if equal, 1 if v1 > v2
 */
export function compareVersions(v1: string, v2: string): -1 | 0 | 1 {
  const ver1 = parseVersion(v1);
  const ver2 = parseVersion(v2);

  if (ver1.major !== ver2.major) return ver1.major > ver2.major ? 1 : -1;
  if (ver1.minor !== ver2.minor) return ver1.minor > ver2.minor ? 1 : -1;
  if (ver1.patch !== ver2.patch) return ver1.patch > ver2.patch ? 1 : -1;

  // Prerelease versions are less than release versions
  if (ver1.prerelease && !ver2.prerelease) return -1;
  if (!ver1.prerelease && ver2.prerelease) return 1;
  if (ver1.prerelease === ver2.prerelease) return 0;

  return ver1.prerelease! > ver2.prerelease! ? 1 : -1;
}

/**
 * Check if update is available
 */
export function isUpdateAvailable(currentVersion: string, newVersion: string): boolean {
  return compareVersions(currentVersion, newVersion) < 0;
}

import { invoke } from "@tauri-apps/api/core";

export interface AppSettings {
  recommendationMode: string;
  autoBackupEnabled: boolean;
  backupIntervalDays: number;
  idleTimeoutMinutes: number;
  encryptionEnabled: boolean;
}

export interface BackupStatus {
  lastBackup: string | null;
  autoBackupEnabled: boolean;
  backupLocation: string;
  backupCount: number;
}

export interface SecuritySettings {
  encryptionAtRest: boolean;
  auditLoggingEnabled: boolean;
  passwordPolicy: PasswordPolicy;
  sessionTimeoutMinutes: number;
}

export interface PasswordPolicy {
  minLength: number;
  requireUppercase: boolean;
  requireDigits: boolean;
  requireSpecial: boolean;
}

export interface RecommendationModeInfo {
  id: string;
  name: string;
  description: string;
  available: boolean;
  requiresApiKey: boolean;
}

export async function getAppSettings(): Promise<AppSettings> {
  return invoke("get_app_settings");
}

export async function updateAppSettings(
  partial: Partial<AppSettings>
): Promise<AppSettings> {
  return invoke("update_app_settings", { input: partial });
}

export async function getRecommendationModes(): Promise<
  RecommendationModeInfo[]
> {
  return invoke("get_recommendation_modes");
}

export async function getBackupStatus(): Promise<BackupStatus> {
  return invoke("get_backup_status");
}

export async function getSecuritySettings(): Promise<SecuritySettings> {
  return invoke("get_security_settings");
}

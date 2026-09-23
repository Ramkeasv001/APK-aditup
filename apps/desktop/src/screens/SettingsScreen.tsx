import { useEffect, useState } from "react";
import {
  getAppSettings,
  updateAppSettings,
  getRecommendationModes,
  getBackupStatus,
  getSecuritySettings,
  type AppSettings,
  type RecommendationModeInfo,
  type BackupStatus,
  type SecuritySettings,
} from "../ipc/settings";
import { useToast } from "../components/Toast";
import { Button } from "../components/Button";
import { Card } from "../components/Card";

interface SettingsScreenProps {
  onClose: () => void;
}

export function SettingsScreen({ onClose }: SettingsScreenProps) {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [modes, setModes] = useState<RecommendationModeInfo[]>([]);
  const [backupStatus, setBackupStatus] = useState<BackupStatus | null>(null);
  const [securitySettings, setSecuritySettings] = useState<SecuritySettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<"general" | "backup" | "security">("general");
  const toast = useToast();

  useEffect(() => {
    async function load() {
      try {
        const [appSettings, recModes, backup, security] = await Promise.all([
          getAppSettings(),
          getRecommendationModes(),
          getBackupStatus(),
          getSecuritySettings(),
        ]);
        setSettings(appSettings);
        setModes(recModes);
        setBackupStatus(backup);
        setSecuritySettings(security);
      } catch (err) {
        toast.error(`Failed to load settings: ${err}`);
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [toast]);

  async function handleRecommendationModeChange(newMode: string) {
    if (!settings) return;
    try {
      const updated = await updateAppSettings({
        recommendationMode: newMode,
      });
      setSettings(updated);
      toast.success("Recommendation mode updated");
    } catch (err) {
      toast.error(`Failed to update settings: ${err}`);
    }
  }

  async function handleAutoBackupToggle() {
    if (!settings) return;
    try {
      const updated = await updateAppSettings({
        autoBackupEnabled: !settings.autoBackupEnabled,
      });
      setSettings(updated);
      toast.success(`Auto-backup ${updated.autoBackupEnabled ? "enabled" : "disabled"}`);
    } catch (err) {
      toast.error(`Failed to update settings: ${err}`);
    }
  }

  if (loading) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-bg">
        <p className="text-sm text-ink-muted">Loading settings…</p>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-bg p-6">
      <div className="mx-auto max-w-2xl">
        <div className="mb-6 flex items-center justify-between">
          <h1 className="text-2xl font-bold text-ink">Settings</h1>
          <button
            onClick={onClose}
            className="text-ink-muted hover:text-ink"
          >
            ✕
          </button>
        </div>

        {/* Tab Navigation */}
        <div className="mb-6 flex gap-2 border-b border-line">
          <button
            onClick={() => setActiveTab("general")}
            className={`px-4 py-2 text-sm font-medium ${
              activeTab === "general"
                ? "border-b-2 border-action-accent text-ink"
                : "text-ink-muted hover:text-ink"
            }`}
          >
            General
          </button>
          <button
            onClick={() => setActiveTab("backup")}
            className={`px-4 py-2 text-sm font-medium ${
              activeTab === "backup"
                ? "border-b-2 border-action-accent text-ink"
                : "text-ink-muted hover:text-ink"
            }`}
          >
            Backup
          </button>
          <button
            onClick={() => setActiveTab("security")}
            className={`px-4 py-2 text-sm font-medium ${
              activeTab === "security"
                ? "border-b-2 border-action-accent text-ink"
                : "text-ink-muted hover:text-ink"
            }`}
          >
            Security
          </button>
        </div>

        {/* General Settings Tab */}
        {activeTab === "general" && settings && (
          <div className="space-y-4">
            <Card>
              <h2 className="mb-4 text-lg font-semibold text-ink">Recommendation Mode</h2>
              <p className="mb-4 text-sm text-ink-muted">
                Choose how audit recommendations are generated and presented.
              </p>
              <div className="space-y-3">
                {modes.map((mode) => (
                  <label key={mode.id} className="flex items-start gap-3">
                    <input
                      type="radio"
                      name="recommendationMode"
                      value={mode.id}
                      checked={settings.recommendationMode === mode.id}
                      onChange={() => handleRecommendationModeChange(mode.id)}
                      disabled={!mode.available}
                      className="mt-1"
                    />
                    <div className="flex-1">
                      <p className={`font-medium ${!mode.available ? "text-ink-muted" : "text-ink"}`}>
                        {mode.name}
                        {!mode.available && (
                          <span className="ml-2 text-xs text-info-accent">(Coming soon)</span>
                        )}
                      </p>
                      <p className="text-xs text-ink-muted">{mode.description}</p>
                      {mode.requiresApiKey && (
                        <p className="mt-1 text-xs text-warning-accent">Requires API key configuration</p>
                      )}
                    </div>
                  </label>
                ))}
              </div>
            </Card>

            <Card>
              <h2 className="mb-4 text-lg font-semibold text-ink">Session Settings</h2>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-ink">
                    Idle Timeout
                  </label>
                  <p className="mt-1 text-sm text-ink-muted">
                    {settings.idleTimeoutMinutes} minutes
                  </p>
                  <p className="text-xs text-ink-muted">
                    App auto-locks when idle for this duration
                  </p>
                </div>
                <div>
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={settings.encryptionEnabled}
                      disabled
                      className="cursor-not-allowed"
                    />
                    <span className="text-sm font-medium text-ink">
                      Encryption at Rest
                    </span>
                  </label>
                  <p className="text-xs text-ink-muted">
                    Database is encrypted with SQLCipher
                  </p>
                </div>
              </div>
            </Card>
          </div>
        )}

        {/* Backup Settings Tab */}
        {activeTab === "backup" && backupStatus && (
          <div className="space-y-4">
            <Card>
              <h2 className="mb-4 text-lg font-semibold text-ink">Backup Status</h2>
              <div className="space-y-3">
                <div>
                  <p className="text-sm text-ink-muted">Location</p>
                  <p className="text-sm font-mono text-ink">{backupStatus.backupLocation}</p>
                </div>
                <div>
                  <p className="text-sm text-ink-muted">Backups Created</p>
                  <p className="text-sm text-ink">{backupStatus.backupCount}</p>
                </div>
                {backupStatus.lastBackup && (
                  <div>
                    <p className="text-sm text-ink-muted">Last Backup</p>
                    <p className="text-sm text-ink">{backupStatus.lastBackup}</p>
                  </div>
                )}
              </div>
            </Card>

            <Card>
              <h2 className="mb-4 text-lg font-semibold text-ink">Automatic Backups</h2>
              <label className="flex items-center gap-3">
                <input
                  type="checkbox"
                  checked={settings.autoBackupEnabled}
                  onChange={handleAutoBackupToggle}
                />
                <span className="text-sm font-medium text-ink">
                  Enable automatic daily backups
                </span>
              </label>
              {settings.autoBackupEnabled && (
                <p className="mt-2 text-xs text-ink-muted">
                  Backups will be created every {settings.backupIntervalDays} days
                </p>
              )}
            </Card>

            <Card>
              <h2 className="mb-4 text-lg font-semibold text-ink">Manual Backup</h2>
              <Button>Create Backup Now</Button>
            </Card>
          </div>
        )}

        {/* Security Settings Tab */}
        {activeTab === "security" && securitySettings && (
          <div className="space-y-4">
            <Card>
              <h2 className="mb-4 text-lg font-semibold text-ink">Encryption</h2>
              <div className="space-y-3">
                <label className="flex items-center gap-3">
                  <input
                    type="checkbox"
                    checked={securitySettings.encryptionAtRest}
                    disabled
                    className="cursor-not-allowed"
                  />
                  <span className="text-sm font-medium text-ink">
                    Encryption at Rest (Always Enabled)
                  </span>
                </label>
              </div>
            </Card>

            <Card>
              <h2 className="mb-4 text-lg font-semibold text-ink">Audit Logging</h2>
              <label className="flex items-center gap-3">
                <input
                  type="checkbox"
                  checked={securitySettings.auditLoggingEnabled}
                  disabled
                  className="cursor-not-allowed"
                />
                <span className="text-sm font-medium text-ink">
                  Audit Logging (Always Enabled)
                </span>
              </label>
              <p className="mt-2 text-xs text-ink-muted">
                All security events and data modifications are logged with cryptographic integrity
              </p>
            </Card>

            <Card>
              <h2 className="mb-4 text-lg font-semibold text-ink">Password Policy</h2>
              <div className="space-y-2">
                <div className="text-sm">
                  <p className="text-ink-muted">Minimum length</p>
                  <p className="font-medium text-ink">{securitySettings.passwordPolicy.minLength} characters</p>
                </div>
                <div className="text-sm">
                  <p className="text-ink-muted">Requirements</p>
                  <ul className="mt-1 space-y-1 text-ink">
                    {securitySettings.passwordPolicy.requireUppercase && (
                      <li>✓ Uppercase letters</li>
                    )}
                    {securitySettings.passwordPolicy.requireDigits && (
                      <li>✓ Numbers</li>
                    )}
                    {securitySettings.passwordPolicy.requireSpecial && (
                      <li>✓ Special characters</li>
                    )}
                  </ul>
                </div>
              </div>
            </Card>

            <Card>
              <h2 className="mb-4 text-lg font-semibold text-ink">Key Management</h2>
              <Button>Rotate Encryption Key</Button>
            </Card>
          </div>
        )}

        <div className="mt-6 flex justify-end gap-3">
          <Button onClick={onClose} variant="ghost">
            Close
          </Button>
        </div>
      </div>
    </div>
  );
}

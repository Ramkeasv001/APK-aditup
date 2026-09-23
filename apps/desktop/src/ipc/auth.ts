import { invoke } from "@tauri-apps/api/core";

export interface SessionStatus {
  needsSetup: boolean;
  isLocked: boolean;
}

export interface SetupResult {
  recoveryKey: string;
}

export interface UnlockResult {
  username: string;
}

export function getSessionStatus(): Promise<SessionStatus> {
  return invoke<SessionStatus>("session_status");
}

export function runSetup(password: string): Promise<SetupResult> {
  return invoke<SetupResult>("run_setup", { password });
}

export function unlock(password: string): Promise<UnlockResult> {
  return invoke<UnlockResult>("unlock", { password });
}

export function lockSession(): Promise<void> {
  return invoke<void>("lock_session");
}

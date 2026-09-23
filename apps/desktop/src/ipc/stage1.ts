import { invoke } from "@tauri-apps/api/core";

export type AuditStatus = "draft" | "stage2" | "stage3" | "finalized";

export interface Audit {
  id: string;
  siteId: string;
  auditorUserId: string;
  auditDate: string;
  status: AuditStatus;
  scopeNotes: string | null;
}

export interface Observation {
  id: string;
  auditId: string;
  entryHash: string;
  text: string;
  location: string | null;
  createdAt: string;
}

export interface StartAuditInput {
  client: string;
  siteName: string;
  location: string | null;
  auditDate: string;
  scopeNotes: string | null;
}

export function startAudit(input: StartAuditInput): Promise<Audit> {
  return invoke<Audit>("start_audit", { ...input });
}

export function listObservations(auditId: string): Promise<Observation[]> {
  return invoke<Observation[]>("list_observations", { auditId });
}

export function addObservation(
  auditId: string,
  text: string,
  location: string | null,
): Promise<Observation> {
  return invoke<Observation>("add_observation", { auditId, text, location });
}

export function updateObservation(id: string, text: string): Promise<Observation> {
  return invoke<Observation>("update_observation", { id, text });
}

export function deleteObservation(id: string): Promise<void> {
  return invoke<void>("delete_observation", { id });
}

export function advanceToStage2(auditId: string): Promise<Audit> {
  return invoke<Audit>("advance_to_stage2", { auditId });
}

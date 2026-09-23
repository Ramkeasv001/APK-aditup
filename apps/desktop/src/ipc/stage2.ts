import { invoke } from "@tauri-apps/api/core";
import type { Audit } from "./stage1";

export type RiskLevel = "low" | "medium" | "high" | "critical";

export interface Stage2Assessment {
  id: string;
  observationId: string;
  riskLevel: RiskLevel;
  category: string | null;
  department: string | null;
  equipment: string | null;
  legalClauseId: string | null;
}

export interface RankedLegalClause {
  legalBankId: string;
  standard: string;
  clause: string;
  text: string;
  confidencePercent: number;
}

export interface Stage2Row {
  observationId: string;
  text: string;
  location: string | null;
  assessment: Stage2Assessment | null;
  suggestedClause: RankedLegalClause | null;
}

export function listStage2Rows(auditId: string): Promise<Stage2Row[]> {
  return invoke<Stage2Row[]>("list_stage2_rows", { auditId });
}

export interface SaveStage2AssessmentInput {
  observationId: string;
  riskLevel: RiskLevel;
  category: string | null;
  department: string | null;
  equipment: string | null;
  legalClauseId: string | null;
}

export function saveStage2Assessment(input: SaveStage2AssessmentInput): Promise<Stage2Assessment> {
  return invoke<Stage2Assessment>("save_stage2_assessment", { ...input });
}

export function advanceToStage3(auditId: string): Promise<Audit> {
  return invoke<Audit>("advance_to_stage3", { auditId });
}

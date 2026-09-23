import { invoke } from "@tauri-apps/api/core";

export interface RiskLevelCounts {
  low: number;
  medium: number;
  high: number;
  critical: number;
}

export interface Stage3StatusCounts {
  matched: number;
  written: number;
  escalated: number;
}

export interface LabeledCount {
  label: string;
  count: number;
}

export interface AuditAnalytics {
  auditId: string;
  totalObservations: number;
  assessedCount: number;
  riskLevels: RiskLevelCounts;
  topCategories: LabeledCount[];
  topDepartments: LabeledCount[];
  stage3Status: Stage3StatusCounts;
  averageMatchScore: number | null;
  resolutionRate: number;
}

export function getAuditAnalytics(auditId: string): Promise<AuditAnalytics> {
  return invoke<AuditAnalytics>("get_audit_analytics", { auditId });
}

import { invoke } from "@tauri-apps/api/core";
import type { Audit, Observation } from "./stage1";

export type Stage3MatchStatus = "matched" | "written" | "escalated";

export interface Stage3Match {
  id: string;
  observationId: string;
  bankKey: string | null;
  matchedRowId: string | null;
  matchScore: number | null;
  status: Stage3MatchStatus;
  finalText: string | null;
}

export interface Stage3ListRow {
  observation: Observation;
  matchState: Stage3Match | null;
}

export type ConfidenceBand = "low" | "medium" | "high";

export interface CandidateScores {
  keywordOverlap: number;
  tfidf: number;
  fuzzySimilarity: number;
  synonymContribution: number;
  structuredMatch: number;
}

export interface RecommendationText {
  id: string;
  text: string;
}

export interface RankedCandidate {
  observationBankId: string;
  bankKey: string;
  topic: string;
  label: string;
  text: string;
  recommendations: RecommendationText[];
  confidencePercent: number;
  confidenceBand: ConfidenceBand;
  scores: CandidateScores;
}

export interface Stage3Suggestions {
  candidates: RankedCandidate[];
  matchState: Stage3Match | null;
}

export function listStage3Rows(auditId: string): Promise<Stage3ListRow[]> {
  return invoke<Stage3ListRow[]>("list_stage3_rows", { auditId });
}

export function suggestCandidates(observationId: string): Promise<Stage3Suggestions> {
  return invoke<Stage3Suggestions>("suggest_candidates", { observationId });
}

export interface SelectCandidateInput {
  observationId: string;
  bankKey: string;
  matchedRowId: string;
  matchScore: number;
  finalText: string;
  scores: CandidateScores;
}

export function selectCandidate(input: SelectCandidateInput): Promise<Stage3Match> {
  return invoke<Stage3Match>("select_candidate", { input });
}

export interface RejectCandidateInput {
  observationId: string;
  bankKey: string;
  scores: CandidateScores;
}

export function rejectCandidate(input: RejectCandidateInput): Promise<void> {
  return invoke<void>("reject_candidate", { input });
}

export function writeOwnRecommendation(observationId: string, text: string): Promise<Stage3Match> {
  return invoke<Stage3Match>("write_own_recommendation", { observationId, text });
}

export function editFinalText(observationId: string, newText: string): Promise<Stage3Match> {
  return invoke<Stage3Match>("edit_final_text", { observationId, newText });
}

export function escalateToAdmin(observationId: string): Promise<void> {
  return invoke<void>("escalate_to_admin", { observationId });
}

export function finalizeAudit(auditId: string): Promise<Audit> {
  return invoke<Audit>("finalize_audit", { auditId });
}

import { invoke } from "@tauri-apps/api/core";

// ------------------------------------------------------------ Admin auth

export function isAdminPinSet(): Promise<boolean> {
  return invoke<boolean>("is_admin_pin_set");
}

export function setAdminPin(pin: string): Promise<void> {
  return invoke<void>("set_admin_pin", { pin });
}

export function verifyAdminPin(pin: string): Promise<void> {
  return invoke<void>("verify_admin_pin", { pin });
}

export function exitAdmin(): Promise<void> {
  return invoke<void>("exit_admin");
}

// -------------------------------------------------------------- Pending

export type ObservationPendingStatus = "pending" | "approved" | "rejected" | "merged";

export interface PendingObservation {
  id: number;
  status: ObservationPendingStatus;
  bankKeyGuess: string | null;
  sourceEntryHash: string;
  payload: string;
  createdAt: string;
}

export interface PendingObservationPayload {
  text: string;
  location: string | null;
  tags: { category: string | null; department: string | null; equipment: string | null };
}

export function parsePendingPayload(payload: string): PendingObservationPayload {
  return JSON.parse(payload) as PendingObservationPayload;
}

export function listPendingObservations(): Promise<PendingObservation[]> {
  return invoke<PendingObservation[]>("list_pending_observations");
}

export function resolvePendingObservation(
  id: number,
  decision: "approve" | "reject",
): Promise<void> {
  return invoke<void>("resolve_pending_observation", { id, decision });
}

// -------------------------------------------------------------- Keywords

export type TermPendingStatus = "pending" | "approved" | "rejected";

export interface PendingKeyword {
  id: number;
  term: string;
  contexts: string[];
  frequencyObserved: number;
  status: TermPendingStatus;
}

export function listPendingKeywords(): Promise<PendingKeyword[]> {
  return invoke<PendingKeyword[]>("list_pending_keywords");
}

export function approvePendingKeyword(id: number): Promise<void> {
  return invoke<void>("approve_pending_keyword", { id });
}

export function rejectPendingKeyword(id: number): Promise<void> {
  return invoke<void>("reject_pending_keyword", { id });
}

// ------------------------------------------------------------- Synonyms

export interface PendingSynonym {
  id: number;
  termA: string;
  termB: string;
  confidence: number;
  status: TermPendingStatus;
}

export function listPendingSynonyms(): Promise<PendingSynonym[]> {
  return invoke<PendingSynonym[]>("list_pending_synonyms");
}

export function approvePendingSynonym(id: number): Promise<void> {
  return invoke<void>("approve_pending_synonym", { id });
}

export function rejectPendingSynonym(id: number): Promise<void> {
  return invoke<void>("reject_pending_synonym", { id });
}

// -------------------------------------------------------- Merge requests

export type MergeRequestStatus = "pending" | "merged" | "kept_both" | "discarded";
export type MergeDecision = "merge" | "keep_both" | "discard";

export interface MergeRequest {
  id: number;
  candidateAId: number;
  candidateBId: number;
  similarity: number;
  status: MergeRequestStatus;
}

export interface MergeRequestRow {
  request: MergeRequest;
  candidateA: PendingObservation | null;
  candidateB: PendingObservation | null;
}

export function runClusterScan(): Promise<number[]> {
  return invoke<number[]>("run_cluster_scan");
}

export function listMergeRequests(): Promise<MergeRequestRow[]> {
  return invoke<MergeRequestRow[]>("list_merge_requests");
}

export function resolveMergeRequest(id: number, decision: MergeDecision): Promise<void> {
  return invoke<void>("resolve_merge_request", { id, decision });
}

// ------------------------------------------------------------- Publish

export interface ObservationBankEntry {
  id: string;
  bankKey: string;
  topic: string;
  label: string;
  text: string;
  version: number;
}

export function listApprovedPendingObservations(): Promise<PendingObservation[]> {
  return invoke<PendingObservation[]>("list_approved_pending_observations");
}

export function listBankKeys(): Promise<string[]> {
  return invoke<string[]>("list_bank_keys");
}

export interface CommitPublishInput {
  pendingId: number;
  bankKey: string;
  topic: string;
  label: string;
}

export function commitPublish(input: CommitPublishInput): Promise<ObservationBankEntry> {
  return invoke<ObservationBankEntry>("commit_publish", { input });
}

// --------------------------------------------------------------- Stats

export interface AdminStats {
  pendingObservations: number;
  approvedObservations: number;
  rejectedObservations: number;
  pendingKeywords: number;
  pendingSynonyms: number;
  pendingMergeRequests: number;
  rejectionRatio: number | null;
}

export function getAdminStats(): Promise<AdminStats> {
  return invoke<AdminStats>("get_admin_stats");
}

// ------------------------------------------------------- History search

export interface SearchEvent {
  id: number;
  entryHash: string;
  timestamp: string;
  query: string;
}
export interface SelectionEvent {
  id: number;
  entryHash: string;
  timestamp: string;
  chosenBankKey: string | null;
}
export interface RejectionEvent {
  id: number;
  entryHash: string;
  timestamp: string;
  rejectedBankKey: string | null;
}
export interface EditEvent {
  id: number;
  entryHash: string;
  timestamp: string;
  before: string | null;
  after: string | null;
}
export interface ApprovalEvent {
  id: number;
  entryHash: string;
  timestamp: string;
  adminUserId: string | null;
}
export interface ConfidenceEvent {
  id: number;
  entryHash: string;
  timestamp: string;
  breakdownJson: string;
}

export interface EntryTimeline {
  searches: SearchEvent[];
  selections: SelectionEvent[];
  rejections: RejectionEvent[];
  edits: EditEvent[];
  approvals: ApprovalEvent[];
  confidenceSnapshots: ConfidenceEvent[];
}

export function getEntryTimeline(entryHash: string): Promise<EntryTimeline> {
  return invoke<EntryTimeline>("get_entry_timeline", { entryHash });
}

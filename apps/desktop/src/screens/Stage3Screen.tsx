import { useEffect, useState } from "react";
import { Button } from "../components/Button";
import { Card } from "../components/Card";
import { useToast } from "../components/Toast";
import type { Audit, Observation } from "../ipc/stage1";
import {
  editFinalText,
  escalateToAdmin,
  finalizeAudit,
  listStage3Rows,
  rejectCandidate,
  selectCandidate,
  suggestCandidates,
  writeOwnRecommendation,
  type ConfidenceBand,
  type RankedCandidate,
  type Stage3Match,
} from "../ipc/stage3";

interface Stage3ScreenProps {
  audit: Audit;
  onContinue: (audit: Audit) => void;
  onBack: () => void;
}

interface RowUiState {
  observation: Observation;
  matchState: Stage3Match | null;
  expanded: boolean;
  loadingCandidates: boolean;
  candidates: RankedCandidate[] | null;
  rejectedBankKeys: string[];
  writingOwn: boolean;
  ownText: string;
  editingFinal: boolean;
  editText: string;
}

function confidenceBadgeClass(band: ConfidenceBand): string {
  if (band === "high") return "bg-accent-strong text-on-accent";
  if (band === "medium") return "border border-line-strong bg-surface-alt text-ink";
  return "border border-line text-ink-muted";
}

function candidateFinalText(candidate: RankedCandidate): string {
  if (candidate.recommendations.length > 0) {
    return candidate.recommendations.map((r) => r.text).join("\n");
  }
  return candidate.text;
}

/**
 * Stage 3 — Search & recommend (§4). This is where Modules 4-9 actually
 * meet the user: every expand calls `RuleBasedProvider::suggest` (Module 8)
 * with the current feedback-adjusted weights, and every
 * select/reject/write/escalate logs through `KleEngine` (Module 9), which
 * is what closes the self-learning loop.
 *
 * The bottom-bar action is labeled "Finalize audit," not "Export finished
 * Stage 3 observations" as §4 names it — actually generating an XLSX/PPTX/
 * PDF/CSV file is Module 11 (Export Engine), not built yet. This only
 * advances the audit's status; see commands/stage3.rs.
 */
export function Stage3Screen({ audit, onContinue, onBack }: Stage3ScreenProps) {
  const [rows, setRows] = useState<RowUiState[]>([]);
  const [loading, setLoading] = useState(true);
  const [finalizing, setFinalizing] = useState(false);
  const { notify } = useToast();

  useEffect(() => {
    let cancelled = false;
    listStage3Rows(audit.id)
      .then((result) => {
        if (cancelled) return;
        setRows(
          result.map((row) => ({
            observation: row.observation,
            matchState: row.matchState,
            expanded: false,
            loadingCandidates: false,
            candidates: null,
            rejectedBankKeys: [],
            writingOwn: false,
            ownText: "",
            editingFinal: false,
            editText: "",
          })),
        );
      })
      .catch((err: unknown) => notify(String(err), "error"))
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [audit.id, notify]);

  function patchRow(observationId: string, patch: Partial<RowUiState>) {
    setRows((current) =>
      current.map((row) => (row.observation.id === observationId ? { ...row, ...patch } : row)),
    );
  }

  async function toggleExpand(row: RowUiState) {
    const observationId = row.observation.id;
    if (row.expanded) {
      patchRow(observationId, { expanded: false });
      return;
    }
    patchRow(observationId, { expanded: true });
    if (row.candidates !== null) return;

    patchRow(observationId, { loadingCandidates: true });
    try {
      const result = await suggestCandidates(observationId);
      patchRow(observationId, {
        candidates: result.candidates,
        matchState: result.matchState,
      });
    } catch (err) {
      notify(String(err), "error");
    } finally {
      patchRow(observationId, { loadingCandidates: false });
    }
  }

  async function handleUseCandidate(row: RowUiState, candidate: RankedCandidate) {
    try {
      const matchState = await selectCandidate({
        observationId: row.observation.id,
        bankKey: candidate.bankKey,
        matchedRowId: candidate.observationBankId,
        matchScore: candidate.confidencePercent,
        finalText: candidateFinalText(candidate),
        scores: candidate.scores,
      });
      patchRow(row.observation.id, { matchState });
    } catch (err) {
      notify(String(err), "error");
    }
  }

  async function handleRejectCandidate(row: RowUiState, candidate: RankedCandidate) {
    try {
      await rejectCandidate({
        observationId: row.observation.id,
        bankKey: candidate.bankKey,
        scores: candidate.scores,
      });
      patchRow(row.observation.id, {
        rejectedBankKeys: [...row.rejectedBankKeys, candidate.bankKey],
      });
    } catch (err) {
      notify(String(err), "error");
    }
  }

  async function handleWriteOwnSubmit(row: RowUiState) {
    if (!row.ownText.trim()) return;
    try {
      const matchState = await writeOwnRecommendation(row.observation.id, row.ownText.trim());
      patchRow(row.observation.id, { matchState, writingOwn: false, ownText: "" });
    } catch (err) {
      notify(String(err), "error");
    }
  }

  async function handleSaveEdit(row: RowUiState) {
    try {
      const matchState = await editFinalText(row.observation.id, row.editText.trim());
      patchRow(row.observation.id, { matchState, editingFinal: false });
    } catch (err) {
      notify(String(err), "error");
    }
  }

  async function handleEscalate(row: RowUiState) {
    try {
      await escalateToAdmin(row.observation.id);
      patchRow(row.observation.id, {
        matchState: {
          id: row.matchState?.id ?? "",
          observationId: row.observation.id,
          bankKey: null,
          matchedRowId: null,
          matchScore: null,
          status: "escalated",
          finalText: null,
        },
      });
    } catch (err) {
      notify(String(err), "error");
    }
  }

  const resolvedCount = rows.filter((row) => row.matchState !== null).length;
  const canFinalize = rows.length > 0 && resolvedCount === rows.length;

  async function handleFinalize() {
    setFinalizing(true);
    try {
      const updated = await finalizeAudit(audit.id);
      onContinue(updated);
    } catch (err) {
      notify(String(err), "error");
    } finally {
      setFinalizing(false);
    }
  }

  return (
    <div className="mx-auto min-h-screen max-w-3xl px-4 py-10">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-xs font-medium uppercase tracking-wide text-ink-muted">Stage 3</p>
          <h1 className="font-display text-xl font-bold text-ink">Search &amp; recommend</h1>
        </div>
        <Button variant="ghost" onClick={onBack}>
          Back to Stage 2
        </Button>
      </div>

      {!loading && rows.length === 0 && (
        <Card className="mt-6">
          <p className="text-sm text-ink-muted">
            Go back to Stage 1, log at least one observation, then return here.
          </p>
        </Card>
      )}

      <div className="mt-4 flex flex-col gap-3">
        {rows.map((row) => {
          const status = row.matchState?.status ?? null;
          return (
            <Card key={row.observation.id} className="p-0">
              <button
                type="button"
                onClick={() => void toggleExpand(row)}
                className="flex w-full items-center justify-between gap-3 px-4 py-3 text-left"
              >
                <span className="text-sm text-ink">{row.observation.text}</span>
                <span className="flex shrink-0 items-center gap-2">
                  {status === "matched" && (
                    <span className="rounded-full bg-accent-strong px-2.5 py-0.5 text-xs font-medium text-on-accent">
                      Matched
                    </span>
                  )}
                  {status === "written" && (
                    <span className="rounded-full bg-surface-alt px-2.5 py-0.5 text-xs font-medium text-ink">
                      Written
                    </span>
                  )}
                  {status === "escalated" && (
                    <span className="rounded-full border border-line-strong px-2.5 py-0.5 text-xs font-medium text-ink-muted">
                      Escalated
                    </span>
                  )}
                  <span className="text-ink-muted">{row.expanded ? "▲" : "▼"}</span>
                </span>
              </button>

              {row.expanded && (
                <div className="border-t border-line px-4 py-4">
                  {row.matchState?.finalText && !row.editingFinal && (
                    <div className="mb-4 rounded-lg border border-line bg-surface-alt p-3">
                      <p className="text-xs font-medium uppercase tracking-wide text-ink-muted">
                        Final recommendation
                      </p>
                      <p className="mt-1 whitespace-pre-line text-sm text-ink">
                        {row.matchState.finalText}
                      </p>
                      <button
                        type="button"
                        onClick={() =>
                          patchRow(row.observation.id, {
                            editingFinal: true,
                            editText: row.matchState?.finalText ?? "",
                          })
                        }
                        className="mt-2 text-xs font-medium text-accent-strong hover:underline"
                      >
                        Edit
                      </button>
                    </div>
                  )}

                  {row.editingFinal && (
                    <div className="mb-4 flex flex-col gap-2">
                      <textarea
                        value={row.editText}
                        onChange={(event) =>
                          patchRow(row.observation.id, { editText: event.target.value })
                        }
                        rows={3}
                        className="rounded-lg border border-line bg-surface px-3 py-2 text-sm text-ink"
                      />
                      <div className="flex gap-2">
                        <Button onClick={() => void handleSaveEdit(row)}>Save</Button>
                        <Button
                          variant="ghost"
                          onClick={() => patchRow(row.observation.id, { editingFinal: false })}
                        >
                          Cancel
                        </Button>
                      </div>
                    </div>
                  )}

                  {row.loadingCandidates && (
                    <p className="text-sm text-ink-muted">Finding candidates…</p>
                  )}

                  {!row.loadingCandidates && row.candidates && row.candidates.length === 0 && (
                    <p className="text-sm text-ink-muted">
                      Nothing relevant found in the bank yet — write your own below or escalate this
                      to Admin.
                    </p>
                  )}

                  {!row.loadingCandidates && row.candidates && row.candidates.length > 0 && (
                    <div className="flex flex-col gap-2">
                      {row.candidates.map((candidate) => {
                        const isApplied =
                          row.matchState?.matchedRowId === candidate.observationBankId;
                        const isRejected = row.rejectedBankKeys.includes(candidate.bankKey);
                        return (
                          <div
                            key={candidate.observationBankId}
                            className="flex items-start justify-between gap-3 rounded-lg border border-line p-3"
                          >
                            <div>
                              <div className="flex items-center gap-2">
                                <span
                                  className={`rounded-full px-2 py-0.5 text-xs font-semibold ${confidenceBadgeClass(candidate.confidenceBand)}`}
                                >
                                  {Math.round(candidate.confidencePercent)}%
                                </span>
                                <span className="text-xs font-medium text-ink-muted">
                                  {candidate.label}
                                </span>
                              </div>
                              <p className="mt-1 text-sm text-ink">
                                {candidateFinalText(candidate)}
                              </p>
                            </div>
                            <div className="flex shrink-0 flex-col items-end gap-1">
                              {isApplied ? (
                                <span className="rounded-full bg-accent-strong px-3 py-1 text-xs font-medium text-on-accent">
                                  Applied
                                </span>
                              ) : (
                                <Button
                                  variant="ghost"
                                  onClick={() => void handleUseCandidate(row, candidate)}
                                >
                                  Use this
                                </Button>
                              )}
                              {!isApplied && (
                                <button
                                  type="button"
                                  disabled={isRejected}
                                  onClick={() => void handleRejectCandidate(row, candidate)}
                                  className="text-xs text-ink-muted hover:text-risk-critical disabled:opacity-50"
                                >
                                  {isRejected ? "Rejected" : "Reject"}
                                </button>
                              )}
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  )}

                  <div className="mt-4 border-t border-line pt-4">
                    <p className="text-xs font-medium uppercase tracking-wide text-ink-muted">
                      Write your own
                    </p>
                    <textarea
                      value={row.ownText}
                      onChange={(event) =>
                        patchRow(row.observation.id, { ownText: event.target.value })
                      }
                      rows={2}
                      placeholder="Type a recommendation manually…"
                      className="mt-2 w-full rounded-lg border border-line bg-surface px-3 py-2 text-sm text-ink placeholder:text-ink-muted"
                    />
                    <div className="mt-2 flex items-center justify-between">
                      <Button
                        variant="ghost"
                        disabled={!row.ownText.trim()}
                        onClick={() => void handleWriteOwnSubmit(row)}
                      >
                        Use this text
                      </Button>
                      <button
                        type="button"
                        onClick={() => void handleEscalate(row)}
                        className="text-xs font-medium text-ink-muted hover:text-ink"
                      >
                        Escalate to Admin as new
                      </button>
                    </div>
                  </div>
                </div>
              )}
            </Card>
          );
        })}
      </div>

      {rows.length > 0 && (
        <div className="mt-6 flex flex-col items-end gap-2">
          <p className="text-xs text-ink-muted">
            {resolvedCount} of {rows.length} resolved
          </p>
          <Button disabled={!canFinalize || finalizing} onClick={handleFinalize}>
            {finalizing ? "Finalizing…" : "Finalize audit"}
          </Button>
        </div>
      )}
    </div>
  );
}

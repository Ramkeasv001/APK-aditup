import { useEffect, useState } from "react";
import { Button } from "../components/Button";
import { Card } from "../components/Card";
import { useToast } from "../components/Toast";
import {
  advanceToStage3,
  listStage2Rows,
  saveStage2Assessment,
  type RankedLegalClause,
  type RiskLevel,
} from "../ipc/stage2";
import type { Audit } from "../ipc/stage1";

interface Stage2ScreenProps {
  audit: Audit;
  onContinue: (audit: Audit) => void;
  onBack: () => void;
}

interface RowState {
  observationId: string;
  text: string;
  location: string | null;
  riskLevel: RiskLevel | "";
  category: string;
  department: string;
  equipment: string;
  legalClauseId: string | null;
  suggestedClause: RankedLegalClause | null;
}

const RISK_LEVELS: { value: RiskLevel; label: string; chipClass: string }[] = [
  { value: "low", label: "Low", chipClass: "bg-risk-low" },
  { value: "medium", label: "Medium", chipClass: "bg-risk-medium" },
  { value: "high", label: "High", chipClass: "bg-risk-high" },
  { value: "critical", label: "Critical", chipClass: "bg-risk-critical" },
];

/**
 * Stage 2 — Risk rating & legal mapping (§4). Two simplifications, both
 * necessary given what's built so far and worth calling out rather than
 * hiding:
 *
 * - Category/department/equipment are free-text, not pickers from a
 *   curated taxonomy — no module has built a category/department/
 *   equipment tag management system yet.
 * - The legal clause suggestion has no "change" dialog with alternatives —
 *   just the single top suggestion with a "Use this clause" action, since
 *   legal_bank has no admin-facing browse/search UI yet (and starts empty
 *   on a fresh install regardless). The `legal_clause_id` column is a
 *   foreign key, so there's deliberately no free-text override — typing an
 *   arbitrary clause reference would either fail to save or silently
 *   attach to nothing.
 */
export function Stage2Screen({ audit, onContinue, onBack }: Stage2ScreenProps) {
  const [rows, setRows] = useState<RowState[]>([]);
  const [loading, setLoading] = useState(true);
  const [advancing, setAdvancing] = useState(false);
  const { notify } = useToast();

  useEffect(() => {
    let cancelled = false;
    listStage2Rows(audit.id)
      .then((result) => {
        if (cancelled) return;
        setRows(
          result.map((row) => ({
            observationId: row.observationId,
            text: row.text,
            location: row.location,
            riskLevel: row.assessment?.riskLevel ?? "",
            category: row.assessment?.category ?? "",
            department: row.assessment?.department ?? "",
            equipment: row.assessment?.equipment ?? "",
            legalClauseId: row.assessment?.legalClauseId ?? null,
            suggestedClause: row.suggestedClause,
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

  function updateRow(observationId: string, patch: Partial<RowState>) {
    setRows((current) =>
      current.map((row) => (row.observationId === observationId ? { ...row, ...patch } : row)),
    );
  }

  async function persist(row: RowState) {
    if (row.riskLevel === "") return;
    try {
      await saveStage2Assessment({
        observationId: row.observationId,
        riskLevel: row.riskLevel,
        category: row.category.trim() || null,
        department: row.department.trim() || null,
        equipment: row.equipment.trim() || null,
        legalClauseId: row.legalClauseId,
      });
    } catch (err) {
      notify(String(err), "error");
    }
  }

  function handleRiskLevelChange(observationId: string, value: string) {
    const riskLevel = value as RiskLevel | "";
    updateRow(observationId, { riskLevel });
    const row = rows.find((entry) => entry.observationId === observationId);
    if (row) void persist({ ...row, riskLevel });
  }

  function handleFieldBlur(observationId: string) {
    const row = rows.find((entry) => entry.observationId === observationId);
    if (row) void persist(row);
  }

  function applySuggestedClause(observationId: string) {
    const row = rows.find((entry) => entry.observationId === observationId);
    if (!row?.suggestedClause) return;
    const updated = { ...row, legalClauseId: row.suggestedClause.legalBankId };
    updateRow(observationId, { legalClauseId: updated.legalClauseId });
    void persist(updated);
  }

  const counts = rows.reduce(
    (acc, row) => {
      if (row.riskLevel === "") acc.unrated += 1;
      else acc[row.riskLevel] += 1;
      return acc;
    },
    { low: 0, medium: 0, high: 0, critical: 0, unrated: 0 },
  );

  const canContinue = rows.length > 0 && counts.unrated === 0;

  async function handleContinue() {
    setAdvancing(true);
    try {
      const updated = await advanceToStage3(audit.id);
      onContinue(updated);
    } catch (err) {
      notify(String(err), "error");
    } finally {
      setAdvancing(false);
    }
  }

  return (
    <div className="mx-auto min-h-screen max-w-4xl px-4 py-10">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-xs font-medium uppercase tracking-wide text-ink-muted">Stage 2</p>
          <h1 className="font-display text-xl font-bold text-ink">
            Risk rating &amp; legal mapping
          </h1>
        </div>
        <Button variant="ghost" onClick={onBack}>
          Back to Stage 1
        </Button>
      </div>

      {!loading && rows.length === 0 && (
        <Card className="mt-6">
          <p className="text-sm text-ink-muted">
            Go back to Stage 1, log at least one observation, then return here.
          </p>
        </Card>
      )}

      {rows.length > 0 && (
        <>
          <div className="mt-4 flex flex-wrap gap-2">
            {RISK_LEVELS.map((level) => (
              <span
                key={level.value}
                className={`inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-medium text-white ${level.chipClass}`}
              >
                {level.label}: {counts[level.value]}
              </span>
            ))}
            {counts.unrated > 0 && (
              <span className="inline-flex items-center gap-1.5 rounded-full border border-line-strong px-3 py-1 text-xs font-medium text-ink-muted">
                Unrated: {counts.unrated}
              </span>
            )}
          </div>

          <div className="mt-4 flex flex-col gap-3">
            {rows.map((row) => (
              <Card key={row.observationId} className="p-4">
                <p className="text-sm text-ink">{row.text}</p>
                {row.location && <p className="mt-1 text-xs text-ink-muted">{row.location}</p>}

                <div className="mt-3 grid grid-cols-2 gap-3 sm:grid-cols-4">
                  <label className="flex flex-col gap-1 text-xs font-medium text-ink-muted">
                    Risk level
                    <select
                      value={row.riskLevel}
                      onChange={(event) =>
                        handleRiskLevelChange(row.observationId, event.target.value)
                      }
                      className="rounded-lg border border-line bg-surface px-2 py-1.5 text-sm text-ink focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
                    >
                      <option value="">Select…</option>
                      {RISK_LEVELS.map((level) => (
                        <option key={level.value} value={level.value}>
                          {level.label}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="flex flex-col gap-1 text-xs font-medium text-ink-muted">
                    Category
                    <input
                      value={row.category}
                      onChange={(event) =>
                        updateRow(row.observationId, { category: event.target.value })
                      }
                      onBlur={() => handleFieldBlur(row.observationId)}
                      className="rounded-lg border border-line bg-surface px-2 py-1.5 text-sm text-ink focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
                    />
                  </label>
                  <label className="flex flex-col gap-1 text-xs font-medium text-ink-muted">
                    Department
                    <input
                      value={row.department}
                      onChange={(event) =>
                        updateRow(row.observationId, { department: event.target.value })
                      }
                      onBlur={() => handleFieldBlur(row.observationId)}
                      className="rounded-lg border border-line bg-surface px-2 py-1.5 text-sm text-ink focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
                    />
                  </label>
                  <label className="flex flex-col gap-1 text-xs font-medium text-ink-muted">
                    Equipment
                    <input
                      value={row.equipment}
                      onChange={(event) =>
                        updateRow(row.observationId, { equipment: event.target.value })
                      }
                      onBlur={() => handleFieldBlur(row.observationId)}
                      className="rounded-lg border border-line bg-surface px-2 py-1.5 text-sm text-ink focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
                    />
                  </label>
                </div>

                <div className="mt-3 border-t border-line pt-3">
                  {row.suggestedClause ? (
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <p className="text-xs font-medium uppercase tracking-wide text-ink-muted">
                          Suggested clause · {row.suggestedClause.standard}
                        </p>
                        <p className="mt-0.5 text-sm text-ink">
                          {row.suggestedClause.clause} — {row.suggestedClause.text}
                        </p>
                      </div>
                      {row.legalClauseId === row.suggestedClause.legalBankId ? (
                        <span className="shrink-0 rounded-full bg-surface-alt px-3 py-1 text-xs font-medium text-accent-strong">
                          Applied
                        </span>
                      ) : (
                        <Button
                          variant="ghost"
                          className="shrink-0"
                          onClick={() => applySuggestedClause(row.observationId)}
                        >
                          Use this clause
                        </Button>
                      )}
                    </div>
                  ) : (
                    <p className="text-xs text-ink-muted">
                      No legal clause on file yet for this observation.
                    </p>
                  )}
                </div>
              </Card>
            ))}
          </div>

          <div className="mt-6 flex flex-col items-end gap-2">
            {!canContinue && (
              <p className="text-xs text-ink-muted">Rate every observation to continue.</p>
            )}
            <Button disabled={!canContinue || advancing} onClick={handleContinue}>
              {advancing ? "Continuing…" : "Save & continue to Stage 3"}
            </Button>
          </div>
        </>
      )}
    </div>
  );
}

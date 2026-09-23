import { useEffect, useState } from "react";
import { Button } from "../components/Button";
import { Card } from "../components/Card";
import { useToast } from "../components/Toast";
import { getAuditAnalytics, type AuditAnalytics } from "../ipc/analytics";
import { exportAuditReport } from "../ipc/export";
import type { Audit } from "../ipc/stage1";

interface AnalyticsScreenProps {
  audit: Audit;
  onLock: () => void;
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <Card className="p-4">
      <p className="text-xs font-medium uppercase tracking-wide text-ink-muted">{label}</p>
      <p className="mt-1 font-display text-2xl font-bold text-ink">{value}</p>
    </Card>
  );
}

function BarRow({
  label,
  count,
  total,
  colorClass,
}: {
  label: string;
  count: number;
  total: number;
  colorClass: string;
}) {
  const pct = total > 0 ? Math.round((count / total) * 100) : 0;
  return (
    <div className="flex items-center gap-3">
      <span className="w-32 shrink-0 truncate text-sm text-ink-muted" title={label}>
        {label}
      </span>
      <div className="h-2 flex-1 overflow-hidden rounded-full bg-surface-alt">
        <div className={`h-full ${colorClass}`} style={{ width: `${pct}%` }} />
      </div>
      <span className="w-8 shrink-0 text-right text-sm text-ink">{count}</span>
    </div>
  );
}

/**
 * Module 10 (part 6/7) — Analytics. Single-audit dashboard shown once an
 * audit reaches "finalized," backed entirely by `get_audit_analytics`
 * (three existing `list_for_audit` queries aggregated in Rust — no new
 * repository methods). Real XLSX/PPTX/PDF/CSV export is still Module 11;
 * this is read-only.
 */
export function AnalyticsScreen({ audit, onLock }: AnalyticsScreenProps) {
  const [data, setData] = useState<AuditAnalytics | null>(null);
  const [exportingFormat, setExportingFormat] = useState<"xlsx" | "csv" | null>(null);
  const { notify } = useToast();

  useEffect(() => {
    getAuditAnalytics(audit.id)
      .then(setData)
      .catch((err: unknown) => notify(String(err), "error"));
  }, [audit.id, notify]);

  async function handleExport(format: "xlsx" | "csv") {
    setExportingFormat(format);
    try {
      const result = await exportAuditReport(audit.id, format);
      notify(`Exported to ${format.toUpperCase()}: ${result.filePath}`, "success");
    } catch (err) {
      notify(String(err), "error");
    } finally {
      setExportingFormat(null);
    }
  }

  return (
    <div className="mx-auto min-h-screen max-w-4xl px-4 py-10">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-xs font-medium uppercase tracking-wide text-ink-muted">
            Audit finalized
          </p>
          <h1 className="font-display text-xl font-bold text-ink">Analytics</h1>
        </div>
        <Button variant="ghost" onClick={onLock}>
          Lock now
        </Button>
      </div>

      {!data ? (
        <p className="mt-6 text-sm text-ink-muted">Loading…</p>
      ) : (
        <div className="mt-6 flex flex-col gap-6">
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
            <Stat label="Observations" value={String(data.totalObservations)} />
            <Stat label="Risk-assessed" value={String(data.assessedCount)} />
            <Stat label="Resolved (Stage 3)" value={`${Math.round(data.resolutionRate * 100)}%`} />
            <Stat
              label="Avg. match confidence"
              value={
                data.averageMatchScore === null ? "—" : `${Math.round(data.averageMatchScore)}%`
              }
            />
          </div>

          <Card>
            <h2 className="font-display text-sm font-bold text-ink">Risk levels</h2>
            <div className="mt-4 flex flex-col gap-3">
              <BarRow
                label="Low"
                count={data.riskLevels.low}
                total={data.assessedCount}
                colorClass="bg-risk-low"
              />
              <BarRow
                label="Medium"
                count={data.riskLevels.medium}
                total={data.assessedCount}
                colorClass="bg-risk-medium"
              />
              <BarRow
                label="High"
                count={data.riskLevels.high}
                total={data.assessedCount}
                colorClass="bg-risk-high"
              />
              <BarRow
                label="Critical"
                count={data.riskLevels.critical}
                total={data.assessedCount}
                colorClass="bg-risk-critical"
              />
            </div>
          </Card>

          <Card>
            <h2 className="font-display text-sm font-bold text-ink">Stage 3 outcome</h2>
            <div className="mt-4 flex flex-col gap-3">
              <BarRow
                label="Matched"
                count={data.stage3Status.matched}
                total={data.totalObservations}
                colorClass="bg-accent"
              />
              <BarRow
                label="Written"
                count={data.stage3Status.written}
                total={data.totalObservations}
                colorClass="bg-gold"
              />
              <BarRow
                label="Escalated"
                count={data.stage3Status.escalated}
                total={data.totalObservations}
                colorClass="bg-risk-high"
              />
            </div>
          </Card>

          {data.topCategories.length > 0 && (
            <Card>
              <h2 className="font-display text-sm font-bold text-ink">Top categories</h2>
              <div className="mt-4 flex flex-col gap-3">
                {data.topCategories.map((row) => (
                  <BarRow
                    key={row.label}
                    label={row.label}
                    count={row.count}
                    total={data.assessedCount}
                    colorClass="bg-accent"
                  />
                ))}
              </div>
            </Card>
          )}

          {data.topDepartments.length > 0 && (
            <Card>
              <h2 className="font-display text-sm font-bold text-ink">Top departments</h2>
              <div className="mt-4 flex flex-col gap-3">
                {data.topDepartments.map((row) => (
                  <BarRow
                    key={row.label}
                    label={row.label}
                    count={row.count}
                    total={data.assessedCount}
                    colorClass="bg-accent"
                  />
                ))}
              </div>
            </Card>
          )}

          <Card>
            <h2 className="font-display text-sm font-bold text-ink">Export audit</h2>
            <p className="mt-2 text-xs text-ink-muted">
              Download this audit's data as a spreadsheet. PDF and PowerPoint export coming later.
            </p>
            <div className="mt-4 flex gap-3">
              <Button onClick={() => handleExport("xlsx")} disabled={exportingFormat !== null}>
                {exportingFormat === "xlsx" ? "Exporting…" : "Excel (.xlsx)"}
              </Button>
              <Button onClick={() => handleExport("csv")} disabled={exportingFormat !== null}>
                {exportingFormat === "csv" ? "Exporting…" : "CSV (.csv)"}
              </Button>
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}

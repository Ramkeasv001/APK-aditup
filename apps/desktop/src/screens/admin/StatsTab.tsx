import { useEffect, useState } from "react";
import { Card } from "../../components/Card";
import { useToast } from "../../components/Toast";
import { getAdminStats, type AdminStats } from "../../ipc/admin";

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <Card className="p-4">
      <p className="text-xs font-medium uppercase tracking-wide text-ink-muted">{label}</p>
      <p className="mt-1 font-display text-2xl font-bold text-ink">{value}</p>
    </Card>
  );
}

/**
 * §4 describes contribution-by-auditor, average-confidence-by-bank,
 * category-coverage gaps, and unused-official-rows dashboards too — none
 * of those have an aggregation query built yet (Module 9 explicitly scoped
 * them out), so this only shows the two metrics that are real: queue
 * backlog counts and the rejection ratio.
 */
export function StatsTab() {
  const [stats, setStats] = useState<AdminStats | null>(null);
  const { notify } = useToast();

  useEffect(() => {
    getAdminStats()
      .then(setStats)
      .catch((err: unknown) => notify(String(err), "error"));
  }, [notify]);

  if (!stats) return <p className="text-sm text-ink-muted">Loading…</p>;

  return (
    <div className="flex flex-col gap-4">
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
        <Stat label="Pending observations" value={String(stats.pendingObservations)} />
        <Stat label="Approved, awaiting publish" value={String(stats.approvedObservations)} />
        <Stat label="Rejected observations" value={String(stats.rejectedObservations)} />
        <Stat label="Pending keywords" value={String(stats.pendingKeywords)} />
        <Stat label="Pending synonyms" value={String(stats.pendingSynonyms)} />
        <Stat label="Pending merge requests" value={String(stats.pendingMergeRequests)} />
        <Stat
          label="Rejection ratio"
          value={stats.rejectionRatio === null ? "—" : `${Math.round(stats.rejectionRatio * 100)}%`}
        />
      </div>
      <p className="text-xs text-ink-muted">
        Contribution-by-auditor, average-confidence-by-bank, category-coverage gaps, and
        unused-official-rows aren't shown here — those need dedicated aggregation queries that
        haven't been built yet.
      </p>
    </div>
  );
}

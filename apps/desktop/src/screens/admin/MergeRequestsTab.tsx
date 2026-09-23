import { useEffect, useState } from "react";
import { Button } from "../../components/Button";
import { Card } from "../../components/Card";
import { useToast } from "../../components/Toast";
import {
  listMergeRequests,
  parsePendingPayload,
  resolveMergeRequest,
  type MergeRequestRow,
} from "../../ipc/admin";

function candidateText(row: MergeRequestRow["candidateA"]): string {
  if (!row) return "(no longer available)";
  return parsePendingPayload(row.payload).text;
}

/** §4: side-by-side diff of the two candidates + Merge/Keep both/Discard. */
export function MergeRequestsTab() {
  const [rows, setRows] = useState<MergeRequestRow[]>([]);
  const [loading, setLoading] = useState(true);
  const { notify } = useToast();

  useEffect(() => {
    listMergeRequests()
      .then(setRows)
      .catch((err: unknown) => notify(String(err), "error"))
      .finally(() => setLoading(false));
  }, [notify]);

  async function handle(id: number, decision: "merge" | "keep_both" | "discard") {
    try {
      await resolveMergeRequest(id, decision);
      setRows((current) => current.filter((row) => row.request.id !== id));
    } catch (err) {
      notify(String(err), "error");
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <p className="text-sm text-ink-muted">Likely duplicates found by the cluster scan.</p>
      {loading && <p className="text-sm text-ink-muted">Loading…</p>}
      {!loading && rows.length === 0 && (
        <p className="text-sm text-ink-muted">
          No merge requests. Run a cluster scan from Pending.
        </p>
      )}
      {rows.map((row) => (
        <Card key={row.request.id} className="p-4">
          <p className="text-xs font-medium uppercase tracking-wide text-ink-muted">
            {Math.round(row.request.similarity * 100)}% similar
          </p>
          <div className="mt-2 grid grid-cols-2 gap-3">
            <div className="rounded-lg border border-line p-3 text-sm text-ink">
              {candidateText(row.candidateA)}
            </div>
            <div className="rounded-lg border border-line p-3 text-sm text-ink">
              {candidateText(row.candidateB)}
            </div>
          </div>
          <div className="mt-3 flex gap-2">
            <Button onClick={() => void handle(row.request.id, "merge")}>
              Merge into existing
            </Button>
            <Button variant="ghost" onClick={() => void handle(row.request.id, "keep_both")}>
              Keep both
            </Button>
            <Button variant="ghost" onClick={() => void handle(row.request.id, "discard")}>
              Discard
            </Button>
          </div>
        </Card>
      ))}
    </div>
  );
}

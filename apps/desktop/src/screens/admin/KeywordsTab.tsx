import { useEffect, useState } from "react";
import { Button } from "../../components/Button";
import { Card } from "../../components/Card";
import { useToast } from "../../components/Toast";
import {
  approvePendingKeyword,
  listPendingKeywords,
  rejectPendingKeyword,
  type PendingKeyword,
} from "../../ipc/admin";

export function KeywordsTab() {
  const [rows, setRows] = useState<PendingKeyword[]>([]);
  const [loading, setLoading] = useState(true);
  const { notify } = useToast();

  useEffect(() => {
    listPendingKeywords()
      .then(setRows)
      .catch((err: unknown) => notify(String(err), "error"))
      .finally(() => setLoading(false));
  }, [notify]);

  async function handle(id: number, action: "approve" | "reject") {
    try {
      if (action === "approve") await approvePendingKeyword(id);
      else await rejectPendingKeyword(id);
      setRows((current) => current.filter((row) => row.id !== id));
    } catch (err) {
      notify(String(err), "error");
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <p className="text-sm text-ink-muted">Terms mined from novel observations (Module 9).</p>
      {loading && <p className="text-sm text-ink-muted">Loading…</p>}
      {!loading && rows.length === 0 && <p className="text-sm text-ink-muted">Nothing pending.</p>}
      {rows.map((row) => (
        <Card key={row.id} className="flex items-center justify-between p-4">
          <div>
            <p className="text-sm font-medium text-ink">{row.term}</p>
            <p className="mt-0.5 text-xs text-ink-muted">
              Seen {row.frequencyObserved}× · contexts: {row.contexts.join(", ") || "none"}
            </p>
          </div>
          <div className="flex gap-2">
            <Button onClick={() => void handle(row.id, "approve")}>Approve</Button>
            <Button variant="ghost" onClick={() => void handle(row.id, "reject")}>
              Reject
            </Button>
          </div>
        </Card>
      ))}
    </div>
  );
}

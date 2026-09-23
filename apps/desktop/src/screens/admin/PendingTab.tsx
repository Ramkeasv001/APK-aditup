import { useEffect, useState } from "react";
import { Button } from "../../components/Button";
import { Card } from "../../components/Card";
import { useToast } from "../../components/Toast";
import {
  listPendingObservations,
  parsePendingPayload,
  resolvePendingObservation,
  runClusterScan,
  type PendingObservation,
} from "../../ipc/admin";

/**
 * §4: "Pending — list of staged observations with Approve / Reject /
 * Edit-then-approve per row; bulk 'Cluster scan'..." Edit-then-approve
 * isn't wired up — there's no repository method to update a pending
 * observation's payload before approving, only to change its status — so
 * this is Approve/Reject only, noted rather than silently dropped.
 */
export function PendingTab() {
  const [rows, setRows] = useState<PendingObservation[]>([]);
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const { notify } = useToast();

  function load() {
    setLoading(true);
    listPendingObservations()
      .then(setRows)
      .catch((err: unknown) => notify(String(err), "error"))
      .finally(() => setLoading(false));
  }

  useEffect(load, [notify]);

  async function handleResolve(id: number, decision: "approve" | "reject") {
    try {
      await resolvePendingObservation(id, decision);
      setRows((current) => current.filter((row) => row.id !== id));
      notify(decision === "approve" ? "Approved" : "Rejected", "success");
    } catch (err) {
      notify(String(err), "error");
    }
  }

  async function handleClusterScan() {
    setScanning(true);
    try {
      const created = await runClusterScan();
      notify(
        `Cluster scan found ${created.length} likely duplicate pair(s) — see Merge Requests.`,
        "success",
      );
    } catch (err) {
      notify(String(err), "error");
    } finally {
      setScanning(false);
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-ink-muted">
          Observations that scored below the novelty threshold in Stage 3.
        </p>
        <Button variant="ghost" disabled={scanning} onClick={() => void handleClusterScan()}>
          {scanning ? "Scanning…" : "Run cluster scan"}
        </Button>
      </div>

      {loading && <p className="text-sm text-ink-muted">Loading…</p>}
      {!loading && rows.length === 0 && <p className="text-sm text-ink-muted">Nothing pending.</p>}

      {rows.map((row) => {
        const payload = parsePendingPayload(row.payload);
        return (
          <Card key={row.id} className="p-4">
            <p className="text-sm text-ink">{payload.text}</p>
            {payload.location && <p className="mt-1 text-xs text-ink-muted">{payload.location}</p>}
            <p className="mt-1 text-xs text-ink-muted">
              Guessed bank: {row.bankKeyGuess ?? "none"} · Hash: {row.sourceEntryHash.slice(0, 12)}…
            </p>
            <div className="mt-3 flex gap-2">
              <Button onClick={() => void handleResolve(row.id, "approve")}>Approve</Button>
              <Button variant="ghost" onClick={() => void handleResolve(row.id, "reject")}>
                Reject
              </Button>
            </div>
          </Card>
        );
      })}
    </div>
  );
}

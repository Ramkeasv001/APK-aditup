import { useEffect, useState } from "react";
import { Button } from "../../components/Button";
import { Card } from "../../components/Card";
import { useToast } from "../../components/Toast";
import {
  commitPublish,
  listApprovedPendingObservations,
  listBankKeys,
  parsePendingPayload,
  type PendingObservation,
} from "../../ipc/admin";

interface DraftFields {
  bankKey: string;
  topic: string;
  label: string;
  confirming: boolean;
}

/**
 * §4: preview then a single confirmed "Commit publish" — irreversible (a
 * publish is a new bank version; there's no delete, only republishing an
 * earlier version). The "preview" here is simply showing the exact text
 * that will become the new observation_bank row, since these are new rows
 * rather than edits to an existing one needing a diff.
 */
export function PublishTab() {
  const [rows, setRows] = useState<PendingObservation[]>([]);
  const [bankKeys, setBankKeys] = useState<string[]>([]);
  const [drafts, setDrafts] = useState<Record<number, DraftFields>>({});
  const [loading, setLoading] = useState(true);
  const { notify } = useToast();

  useEffect(() => {
    Promise.all([listApprovedPendingObservations(), listBankKeys()])
      .then(([observations, keys]) => {
        setRows(observations);
        setBankKeys(keys);
        setDrafts(
          Object.fromEntries(
            observations.map((row) => [
              row.id,
              { bankKey: row.bankKeyGuess ?? "", topic: "", label: "", confirming: false },
            ]),
          ),
        );
      })
      .catch((err: unknown) => notify(String(err), "error"))
      .finally(() => setLoading(false));
  }, [notify]);

  function updateDraft(id: number, patch: Partial<DraftFields>) {
    setDrafts((current) => {
      const existing = current[id];
      if (!existing) return current;
      return { ...current, [id]: { ...existing, ...patch } };
    });
  }

  async function handleCommit(row: PendingObservation) {
    const draft = drafts[row.id];
    if (!draft?.bankKey.trim() || !draft.topic.trim() || !draft.label.trim()) return;
    try {
      await commitPublish({
        pendingId: row.id,
        bankKey: draft.bankKey.trim(),
        topic: draft.topic.trim(),
        label: draft.label.trim(),
      });
      setRows((current) => current.filter((entry) => entry.id !== row.id));
      notify("Published to the official bank.", "success");
    } catch (err) {
      notify(String(err), "error");
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <p className="text-sm text-ink-muted">
        Approved observations ready to become official bank rows.
      </p>
      {loading && <p className="text-sm text-ink-muted">Loading…</p>}
      {!loading && rows.length === 0 && (
        <p className="text-sm text-ink-muted">Nothing approved and waiting to publish.</p>
      )}
      {rows.map((row) => {
        const draft = drafts[row.id];
        if (!draft) return null;
        const payload = parsePendingPayload(row.payload);
        const canCommit = draft.bankKey.trim() && draft.topic.trim() && draft.label.trim();

        return (
          <Card key={row.id} className="p-4">
            <p className="text-xs font-medium uppercase tracking-wide text-ink-muted">
              Will publish as
            </p>
            <p className="mt-1 text-sm text-ink">{payload.text}</p>

            <div className="mt-3 grid grid-cols-3 gap-3">
              <label className="flex flex-col gap-1 text-xs font-medium text-ink-muted">
                Bank key
                <input
                  list={`bank-keys-${row.id}`}
                  value={draft.bankKey}
                  onChange={(event) => updateDraft(row.id, { bankKey: event.target.value })}
                  className="rounded-lg border border-line bg-surface px-2 py-1.5 text-sm text-ink"
                />
                <datalist id={`bank-keys-${row.id}`}>
                  {bankKeys.map((key) => (
                    <option key={key} value={key} />
                  ))}
                </datalist>
              </label>
              <label className="flex flex-col gap-1 text-xs font-medium text-ink-muted">
                Topic
                <input
                  value={draft.topic}
                  onChange={(event) => updateDraft(row.id, { topic: event.target.value })}
                  className="rounded-lg border border-line bg-surface px-2 py-1.5 text-sm text-ink"
                />
              </label>
              <label className="flex flex-col gap-1 text-xs font-medium text-ink-muted">
                Label
                <input
                  value={draft.label}
                  onChange={(event) => updateDraft(row.id, { label: event.target.value })}
                  className="rounded-lg border border-line bg-surface px-2 py-1.5 text-sm text-ink"
                />
              </label>
            </div>

            <div className="mt-3 flex items-center gap-3">
              {!draft.confirming ? (
                <Button
                  variant="ghost"
                  disabled={!canCommit}
                  onClick={() => updateDraft(row.id, { confirming: true })}
                >
                  Review
                </Button>
              ) : (
                <>
                  <span className="text-xs text-risk-critical">
                    This cannot be undone. Publish "{draft.label}" into bank "{draft.bankKey}"?
                  </span>
                  <Button onClick={() => void handleCommit(row)}>Commit publish</Button>
                  <Button
                    variant="ghost"
                    onClick={() => updateDraft(row.id, { confirming: false })}
                  >
                    Cancel
                  </Button>
                </>
              )}
            </div>
          </Card>
        );
      })}
    </div>
  );
}

import { useEffect, useState, type FormEvent } from "react";
import { Button } from "../components/Button";
import { TextField } from "../components/TextField";
import { Card } from "../components/Card";
import { useToast } from "../components/Toast";
import {
  addObservation,
  advanceToStage2,
  deleteObservation,
  listObservations,
  updateObservation,
  type Audit,
  type Observation,
} from "../ipc/stage1";

interface ObservationEntryScreenProps {
  audit: Audit;
  onContinue: (audit: Audit) => void;
}

/**
 * Stage 1 — New observation (§4). Photo attachment isn't wired up yet — it
 * needs an encrypted evidence blob store (§9) that hasn't been built in any
 * completed module — so this is text + location only for now.
 *
 * Delete has no confirmation dialog: §4 only requires one "if the entry has
 * already been scored in Stage 2/3," and nothing reaches this screen with a
 * Stage 2/3 score yet (those screens land in Parts 3-4).
 */
export function ObservationEntryScreen({ audit, onContinue }: ObservationEntryScreenProps) {
  const [observations, setObservations] = useState<Observation[]>([]);
  const [text, setText] = useState("");
  const [location, setLocation] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingText, setEditingText] = useState("");
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [advancing, setAdvancing] = useState(false);
  const { notify } = useToast();

  useEffect(() => {
    let cancelled = false;
    listObservations(audit.id)
      .then((result) => {
        if (!cancelled) setObservations(result);
      })
      .catch((err: unknown) => notify(String(err), "error"))
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [audit.id, notify]);

  async function handleAdd(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!text.trim()) return;
    setSubmitting(true);
    try {
      const created = await addObservation(audit.id, text.trim(), location.trim() || null);
      setObservations((current) => [...current, created]);
      setText("");
      setLocation("");
    } catch (err) {
      notify(String(err), "error");
    } finally {
      setSubmitting(false);
    }
  }

  function startEditing(observation: Observation) {
    setEditingId(observation.id);
    setEditingText(observation.text);
  }

  async function saveEdit(id: string) {
    try {
      const updated = await updateObservation(id, editingText.trim());
      setObservations((current) => current.map((entry) => (entry.id === id ? updated : entry)));
      setEditingId(null);
    } catch (err) {
      notify(String(err), "error");
    }
  }

  async function handleDelete(id: string) {
    try {
      await deleteObservation(id);
      setObservations((current) => current.filter((entry) => entry.id !== id));
    } catch (err) {
      notify(String(err), "error");
    }
  }

  async function handleContinue() {
    setAdvancing(true);
    try {
      const updated = await advanceToStage2(audit.id);
      onContinue(updated);
    } catch (err) {
      notify(String(err), "error");
    } finally {
      setAdvancing(false);
    }
  }

  return (
    <div className="mx-auto min-h-screen max-w-2xl px-4 py-10">
      <p className="text-xs font-medium uppercase tracking-wide text-ink-muted">Stage 1</p>
      <h1 className="font-display text-xl font-bold text-ink">New observation</h1>

      <Card className="mt-4">
        <form className="flex flex-col gap-4" onSubmit={handleAdd}>
          <div className="flex flex-col gap-1.5">
            <label className="text-sm font-medium text-ink" htmlFor="observation-text">
              Observation
            </label>
            <textarea
              id="observation-text"
              value={text}
              onChange={(event) => setText(event.target.value)}
              rows={3}
              placeholder="Describe what you observed…"
              className="rounded-lg border border-line bg-surface px-3 py-2 text-sm text-ink placeholder:text-ink-muted focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
            />
          </div>
          <TextField
            label="Location"
            value={location}
            onChange={(event) => setLocation(event.target.value)}
            hint="Optional — e.g. Bay 3, East stairwell"
          />
          <Button type="submit" disabled={!text.trim() || submitting}>
            {submitting ? "Adding…" : "Add entry"}
          </Button>
        </form>
      </Card>

      <div className="mt-6 flex flex-col gap-3">
        {loading && <p className="text-sm text-ink-muted">Loading…</p>}
        {!loading && observations.length === 0 && (
          <p className="text-sm text-ink-muted">No observations logged yet.</p>
        )}
        {observations.map((observation) => (
          <Card key={observation.id} className="p-4">
            {editingId === observation.id ? (
              <div className="flex flex-col gap-2">
                <textarea
                  value={editingText}
                  onChange={(event) => setEditingText(event.target.value)}
                  rows={2}
                  className="rounded-lg border border-line bg-surface px-3 py-2 text-sm text-ink"
                />
                <div className="flex gap-2">
                  <Button onClick={() => saveEdit(observation.id)}>Save</Button>
                  <Button variant="ghost" onClick={() => setEditingId(null)}>
                    Cancel
                  </Button>
                </div>
              </div>
            ) : (
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="text-sm text-ink">{observation.text}</p>
                  {observation.location && (
                    <p className="mt-1 text-xs text-ink-muted">{observation.location}</p>
                  )}
                </div>
                <div className="flex shrink-0 gap-1">
                  <button
                    type="button"
                    aria-label="Edit"
                    onClick={() => startEditing(observation)}
                    className="rounded p-1.5 text-ink-muted hover:bg-surface-alt hover:text-ink"
                  >
                    ✎
                  </button>
                  <button
                    type="button"
                    aria-label="Delete"
                    onClick={() => handleDelete(observation.id)}
                    className="rounded p-1.5 text-ink-muted hover:bg-surface-alt hover:text-risk-critical"
                  >
                    ✕
                  </button>
                </div>
              </div>
            )}
          </Card>
        ))}
      </div>

      <div className="mt-6 flex flex-col items-end gap-2">
        {observations.length === 0 && (
          <p className="text-xs text-ink-muted">Add at least one observation to continue.</p>
        )}
        <Button disabled={observations.length === 0 || advancing} onClick={handleContinue}>
          {advancing ? "Continuing…" : "Review & continue to Stage 2"}
        </Button>
      </div>
    </div>
  );
}

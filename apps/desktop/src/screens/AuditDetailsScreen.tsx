import { useState, type FormEvent } from "react";
import { Button } from "../components/Button";
import { TextField } from "../components/TextField";
import { Card } from "../components/Card";
import { startAudit, type Audit } from "../ipc/stage1";

interface AuditDetailsScreenProps {
  onCreated: (audit: Audit) => void;
}

function todayIsoDate(): string {
  return new Date().toISOString().slice(0, 10);
}

/**
 * Stage 1 — Audit details (§4). No "back to Landing" button here: there is
 * no project-browsing Landing screen yet (out of this module's scope,
 * noted in commands/stage1.rs) — every submission starts a fresh
 * project/site, so this is the first screen after unlock.
 */
export function AuditDetailsScreen({ onCreated }: AuditDetailsScreenProps) {
  const [client, setClient] = useState("");
  const [siteName, setSiteName] = useState("");
  const [location, setLocation] = useState("");
  const [auditDate, setAuditDate] = useState(todayIsoDate());
  const [scopeNotes, setScopeNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const canContinue =
    client.trim().length > 0 && siteName.trim().length > 0 && auditDate.length > 0;

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canContinue) return;

    setError(null);
    setSubmitting(true);
    try {
      const audit = await startAudit({
        client: client.trim(),
        siteName: siteName.trim(),
        location: location.trim() || null,
        auditDate,
        scopeNotes: scopeNotes.trim() || null,
      });
      onCreated(audit);
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-bg px-4 py-10">
      <Card className="w-full max-w-lg">
        <p className="text-xs font-medium uppercase tracking-wide text-ink-muted">Stage 1</p>
        <h1 className="font-display text-xl font-bold text-ink">Audit details</h1>
        <p className="mt-1 text-sm text-ink-muted">
          Tell us about this audit before logging observations.
        </p>
        <form className="mt-6 flex flex-col gap-4" onSubmit={handleSubmit}>
          <TextField
            label="Client"
            value={client}
            onChange={(event) => setClient(event.target.value)}
            required
          />
          <div className="grid grid-cols-2 gap-4">
            <TextField
              label="Site"
              value={siteName}
              onChange={(event) => setSiteName(event.target.value)}
              required
            />
            <TextField
              label="Location"
              value={location}
              onChange={(event) => setLocation(event.target.value)}
              hint="Optional"
            />
          </div>
          <TextField
            label="Audit date"
            type="date"
            value={auditDate}
            onChange={(event) => setAuditDate(event.target.value)}
            required
          />
          <div className="flex flex-col gap-1.5">
            <label className="text-sm font-medium text-ink" htmlFor="scope-notes">
              Scope notes
            </label>
            <textarea
              id="scope-notes"
              value={scopeNotes}
              onChange={(event) => setScopeNotes(event.target.value)}
              rows={3}
              placeholder="What does this audit cover?"
              className="rounded-lg border border-line bg-surface px-3 py-2 text-sm text-ink placeholder:text-ink-muted focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
            />
          </div>
          {error && <p className="text-sm text-risk-critical">{error}</p>}
          <Button type="submit" disabled={!canContinue || submitting}>
            {submitting ? "Starting…" : "Continue to observations"}
          </Button>
        </form>
      </Card>
    </div>
  );
}

import { useState, type FormEvent } from "react";
import { Button } from "../../components/Button";
import { TextField } from "../../components/TextField";
import { Card } from "../../components/Card";
import { useToast } from "../../components/Toast";
import { getEntryTimeline, type EntryTimeline } from "../../ipc/admin";

/**
 * §4: "paste or select an entry hash." Only "paste" is implemented —
 * nothing in the UI yet surfaces a browsable list of entry hashes to
 * select from, so that half is a known gap, not a silently dropped one.
 */
export function HistorySearchTab() {
  const [hash, setHash] = useState("");
  const [timeline, setTimeline] = useState<EntryTimeline | null>(null);
  const [searching, setSearching] = useState(false);
  const { notify } = useToast();

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!hash.trim()) return;
    setSearching(true);
    try {
      setTimeline(await getEntryTimeline(hash.trim()));
    } catch (err) {
      notify(String(err), "error");
    } finally {
      setSearching(false);
    }
  }

  const totalEvents =
    (timeline?.searches.length ?? 0) +
    (timeline?.selections.length ?? 0) +
    (timeline?.rejections.length ?? 0) +
    (timeline?.edits.length ?? 0) +
    (timeline?.approvals.length ?? 0) +
    (timeline?.confidenceSnapshots.length ?? 0);

  return (
    <div className="flex flex-col gap-4">
      <form className="flex items-end gap-2" onSubmit={handleSubmit}>
        <div className="flex-1">
          <TextField
            label="Entry hash"
            value={hash}
            onChange={(event) => setHash(event.target.value)}
            hint="The full source_entry_hash for one observation"
          />
        </div>
        <Button type="submit" disabled={searching || !hash.trim()}>
          {searching ? "Searching…" : "Look up"}
        </Button>
      </form>

      {timeline && totalEvents === 0 && (
        <p className="text-sm text-ink-muted">No history recorded for that hash.</p>
      )}

      {timeline && totalEvents > 0 && (
        <div className="flex flex-col gap-2">
          {timeline.searches.map((event) => (
            <Card key={`search-${event.id}`} className="p-3 text-sm">
              <span className="font-medium text-ink-muted">Search</span> · {event.timestamp} —
              &ldquo;{event.query}&rdquo;
            </Card>
          ))}
          {timeline.selections.map((event) => (
            <Card key={`selection-${event.id}`} className="p-3 text-sm">
              <span className="font-medium text-ink-muted">Selection</span> · {event.timestamp} —
              chose {event.chosenBankKey ?? "(unknown bank)"}
            </Card>
          ))}
          {timeline.rejections.map((event) => (
            <Card key={`rejection-${event.id}`} className="p-3 text-sm">
              <span className="font-medium text-ink-muted">Rejection</span> · {event.timestamp} —
              rejected {event.rejectedBankKey ?? "(unknown bank)"}
            </Card>
          ))}
          {timeline.edits.map((event) => (
            <Card key={`edit-${event.id}`} className="p-3 text-sm">
              <span className="font-medium text-ink-muted">Edit</span> · {event.timestamp}
              <div className="mt-1 text-xs text-ink-muted">
                {event.before ?? "(none)"} → {event.after ?? "(none)"}
              </div>
            </Card>
          ))}
          {timeline.approvals.map((event) => (
            <Card key={`approval-${event.id}`} className="p-3 text-sm">
              <span className="font-medium text-ink-muted">Approval</span> · {event.timestamp} — by{" "}
              {event.adminUserId ?? "unknown"}
            </Card>
          ))}
          {timeline.confidenceSnapshots.map((event) => (
            <Card key={`confidence-${event.id}`} className="p-3 text-sm">
              <span className="font-medium text-ink-muted">Confidence snapshot</span> ·{" "}
              {event.timestamp}
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

import { useState, type FormEvent } from "react";
import { Button } from "../components/Button";
import { TextField } from "../components/TextField";
import { Card } from "../components/Card";
import { runSetup } from "../ipc/auth";

interface SetupScreenProps {
  onComplete: () => void;
}

/**
 * First-run Setup wizard (§4). Deliberately has no username field — Mode 1
 * v1 is single-account, and the User Journey's Setup wizard is
 * password-only; see the `OWNER_USERNAME` comment in the Rust command.
 */
export function SetupScreen({ onComplete }: SetupScreenProps) {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [recoveryKey, setRecoveryKey] = useState<string | null>(null);
  const [acknowledged, setAcknowledged] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    if (password.length < 8) {
      setError("Password must be at least 8 characters.");
      return;
    }
    if (password !== confirm) {
      setError("Passwords do not match.");
      return;
    }

    setSubmitting(true);
    try {
      const result = await runSetup(password);
      setRecoveryKey(result.recoveryKey);
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  }

  if (recoveryKey) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-bg px-4">
        <Card className="w-full max-w-md">
          <h1 className="font-display text-xl font-bold text-ink">Save your recovery key</h1>
          <p className="mt-2 text-sm text-ink-muted">
            If you forget your password, this is the only way back in — ADITUP cannot reset it for
            you. Store it somewhere safe outside this app (a password manager, a printed copy).
          </p>
          <div className="mt-4 select-all break-all rounded-lg border border-line-strong bg-surface-alt p-4 font-mono text-sm text-ink">
            {recoveryKey}
          </div>
          <label className="mt-4 flex items-start gap-2 text-sm text-ink">
            <input
              type="checkbox"
              checked={acknowledged}
              onChange={(event) => setAcknowledged(event.target.checked)}
              className="mt-0.5"
            />
            I&rsquo;ve saved this recovery key somewhere safe.
          </label>
          <Button className="mt-4 w-full" disabled={!acknowledged} onClick={onComplete}>
            Continue to ADITUP
          </Button>
        </Card>
      </div>
    );
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-bg px-4">
      <Card className="w-full max-w-md">
        <h1 className="font-display text-xl font-bold text-ink">Set up ADITUP</h1>
        <p className="mt-2 text-sm text-ink-muted">
          Create a master password to encrypt this device&rsquo;s audit database. Choose something
          you won&rsquo;t forget — there is no password reset except your recovery key.
        </p>
        <form className="mt-6 flex flex-col gap-4" onSubmit={handleSubmit}>
          <TextField
            label="Master password"
            type="password"
            autoComplete="new-password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            hint="At least 8 characters."
            required
          />
          <TextField
            label="Confirm password"
            type="password"
            autoComplete="new-password"
            value={confirm}
            onChange={(event) => setConfirm(event.target.value)}
            error={error ?? undefined}
            required
          />
          <Button type="submit" disabled={submitting}>
            {submitting ? "Setting up…" : "Create master password"}
          </Button>
        </form>
      </Card>
    </div>
  );
}

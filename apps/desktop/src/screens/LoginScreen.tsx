import { useState, type FormEvent } from "react";
import { Button } from "../components/Button";
import { TextField } from "../components/TextField";
import { Card } from "../components/Card";
import { unlock } from "../ipc/auth";

interface LoginScreenProps {
  onUnlocked: () => void;
}

/** Login/unlock screen (§4). A failed attempt shows the lockout message
 * verbatim from the Rust `AttemptLimiter` — it already states the retry
 * window, so there's nothing for the UI to add. */
export function LoginScreen({ onUnlocked }: LoginScreenProps) {
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await unlock(password);
      onUnlocked();
    } catch (err) {
      setError(String(err));
      setPassword("");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-bg px-4">
      <Card className="w-full max-w-sm">
        <h1 className="font-display text-xl font-bold text-ink">ADITUP</h1>
        <p className="mt-1 text-sm text-ink-muted">Enter your password to unlock.</p>
        <form className="mt-6 flex flex-col gap-4" onSubmit={handleSubmit}>
          <TextField
            label="Password"
            type="password"
            autoComplete="current-password"
            autoFocus
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            error={error ?? undefined}
            required
          />
          <Button type="submit" disabled={submitting || password.length === 0}>
            {submitting ? "Unlocking…" : "Unlock"}
          </Button>
        </form>
      </Card>
    </div>
  );
}

import { useEffect, useState, type FormEvent } from "react";
import { Button } from "../components/Button";
import { TextField } from "../components/TextField";
import { Card } from "../components/Card";
import { isAdminPinSet, setAdminPin, verifyAdminPin } from "../ipc/admin";

interface AdminGateScreenProps {
  onUnlocked: () => void;
  onCancel: () => void;
}

/**
 * Admin Console elevation (§4/§9) — a PIN independent of the login
 * password, checked separately regardless of who's logged in. First visit
 * on a fresh install prompts to set one; every visit after that asks for
 * it, subject to the 3-attempt lockout enforced in commands/admin_auth.rs.
 */
export function AdminGateScreen({ onUnlocked, onCancel }: AdminGateScreenProps) {
  const [pinIsSet, setPinIsSet] = useState<boolean | null>(null);
  const [pin, setPin] = useState("");
  const [confirmPin, setConfirmPin] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    isAdminPinSet()
      .then(setPinIsSet)
      .catch((err: unknown) => setError(String(err)));
  }, []);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    if (pinIsSet === false) {
      if (pin.length < 4) {
        setError("PIN must be at least 4 digits.");
        return;
      }
      if (pin !== confirmPin) {
        setError("PINs do not match.");
        return;
      }
    }

    setSubmitting(true);
    try {
      if (pinIsSet) {
        await verifyAdminPin(pin);
      } else {
        await setAdminPin(pin);
      }
      onUnlocked();
    } catch (err) {
      setError(String(err));
      setPin("");
      setConfirmPin("");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-bg px-4">
      <Card className="w-full max-w-sm">
        <h1 className="font-display text-xl font-bold text-ink">Admin Console</h1>
        <p className="mt-1 text-sm text-ink-muted">
          {pinIsSet === false
            ? "Set an Admin PIN to protect this area."
            : "Enter the Admin PIN to continue."}
        </p>
        {pinIsSet !== null && (
          <form className="mt-6 flex flex-col gap-4" onSubmit={handleSubmit}>
            <TextField
              label="Admin PIN"
              type="password"
              inputMode="numeric"
              autoFocus
              value={pin}
              onChange={(event) => setPin(event.target.value)}
              required
            />
            {pinIsSet === false && (
              <TextField
                label="Confirm PIN"
                type="password"
                inputMode="numeric"
                value={confirmPin}
                onChange={(event) => setConfirmPin(event.target.value)}
                error={error ?? undefined}
                required
              />
            )}
            {pinIsSet && error && <p className="text-sm text-risk-critical">{error}</p>}
            <div className="flex gap-2">
              <Button type="submit" disabled={submitting || pin.length === 0}>
                {submitting ? "Checking…" : pinIsSet ? "Unlock" : "Set PIN"}
              </Button>
              <Button type="button" variant="ghost" onClick={onCancel}>
                Cancel
              </Button>
            </div>
          </form>
        )}
      </Card>
    </div>
  );
}

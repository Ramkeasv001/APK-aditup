import { useCallback, useEffect, useState } from "react";
import { ToastProvider } from "./components/Toast";
import { SetupScreen } from "./screens/SetupScreen";
import { LoginScreen } from "./screens/LoginScreen";
import { UnlockedShell } from "./screens/UnlockedShell";
import { getSessionStatus } from "./ipc/auth";

type AppPhase = "loading" | "error" | "needs-setup" | "locked" | "unlocked";

function App() {
  const [phase, setPhase] = useState<AppPhase>("loading");
  const [error, setError] = useState<string | null>(null);

  const refreshStatus = useCallback(() => {
    getSessionStatus()
      .then((status) => {
        if (status.needsSetup) {
          setPhase("needs-setup");
        } else if (status.isLocked) {
          setPhase("locked");
        } else {
          setPhase("unlocked");
        }
      })
      .catch((err) => {
        setError(String(err));
        setPhase("error");
      });
  }, []);

  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  // Idle-timeout auto-lock (§4/§9) is enforced on the Rust side (every
  // authenticated command drops the database once the session's idle
  // window elapses — see `AppState::with_db`), but nothing surfaces that
  // here until the next command runs. A user idle on a screen that isn't
  // issuing any commands (reading, or mid-typing before a save) would
  // otherwise sit on a stale "unlocked" view well past the real timeout.
  // Polling while unlocked, as `session_status`'s own doc comment
  // anticipates, catches that within one interval instead.
  useEffect(() => {
    if (phase !== "unlocked") return;
    const id = window.setInterval(refreshStatus, 30_000);
    return () => window.clearInterval(id);
  }, [phase, refreshStatus]);

  return (
    <ToastProvider>
      {phase === "loading" && (
        <div className="flex min-h-screen items-center justify-center bg-bg">
          <p className="text-sm text-ink-muted">Loading…</p>
        </div>
      )}
      {phase === "error" && (
        <div className="flex min-h-screen items-center justify-center bg-bg px-4">
          <p className="text-sm text-risk-critical">Startup error: {error}</p>
        </div>
      )}
      {phase === "needs-setup" && <SetupScreen onComplete={refreshStatus} />}
      {phase === "locked" && <LoginScreen onUnlocked={refreshStatus} />}
      {phase === "unlocked" && <UnlockedShell onLocked={refreshStatus} />}
    </ToastProvider>
  );
}

export default App;

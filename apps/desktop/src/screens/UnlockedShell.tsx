import { useState, type ReactNode } from "react";
import { lockSession } from "../ipc/auth";
import type { Audit } from "../ipc/stage1";
import { AuditDetailsScreen } from "./AuditDetailsScreen";
import { ObservationEntryScreen } from "./ObservationEntryScreen";
import { Stage2Screen } from "./Stage2Screen";
import { Stage3Screen } from "./Stage3Screen";
import { AdminGateScreen } from "./AdminGateScreen";
import { AdminConsoleScreen } from "./AdminConsoleScreen";
import { AnalyticsScreen } from "./AnalyticsScreen";
import { SettingsScreen } from "./SettingsScreen";

interface UnlockedShellProps {
  onLocked: () => void;
}

type View =
  | "audit-details"
  | "observations"
  | "stage2"
  | "stage3"
  | "analytics"
  | "admin-gate"
  | "admin-console"
  | "settings";

/**
 * Everything shown once the app is unlocked. `view` is tracked separately
 * from `audit.status` so "Back to Stage N" can return to an earlier screen
 * without needing to (and without being able to) revert the audit's status
 * in the database — the status state machine only moves forward (Module
 * 2's `advance_status`).
 *
 * The floating "Admin" control is rendered here, not inside each Stage
 * screen, so opening the Admin Console works the same from anywhere
 * without threading an `onOpenAdmin` prop through four already-built
 * components.
 */
export function UnlockedShell({ onLocked }: UnlockedShellProps) {
  const [audit, setAudit] = useState<Audit | null>(null);
  const [view, setView] = useState<View>("audit-details");
  const [preAdminView, setPreAdminView] = useState<View>("audit-details");

  async function handleLock() {
    await lockSession();
    onLocked();
  }

  function handleAuditCreated(created: Audit) {
    setAudit(created);
    setView("observations");
  }

  function handleObservationsContinue(updated: Audit) {
    setAudit(updated);
    setView("stage2");
  }

  function handleStage2Continue(updated: Audit) {
    setAudit(updated);
    setView("stage3");
  }

  function handleStage3Continue(updated: Audit) {
    setAudit(updated);
    setView("analytics");
  }

  function openAdmin() {
    setPreAdminView(view);
    setView("admin-gate");
  }

  function openSettings() {
    setPreAdminView(view);
    setView("settings");
  }

  const showAdminEntry = view !== "admin-gate" && view !== "admin-console" && view !== "settings";

  let content: ReactNode;
  if (view === "admin-gate") {
    content = (
      <AdminGateScreen
        onUnlocked={() => setView("admin-console")}
        onCancel={() => setView(preAdminView)}
      />
    );
  } else if (view === "admin-console") {
    content = <AdminConsoleScreen onExit={() => setView(preAdminView)} />;
  } else if (view === "settings") {
    content = <SettingsScreen onClose={() => setView(preAdminView)} />;
  } else if (view === "audit-details" || !audit) {
    content = <AuditDetailsScreen onCreated={handleAuditCreated} />;
  } else if (view === "observations") {
    content = <ObservationEntryScreen audit={audit} onContinue={handleObservationsContinue} />;
  } else if (view === "stage2") {
    content = (
      <Stage2Screen
        audit={audit}
        onContinue={handleStage2Continue}
        onBack={() => setView("observations")}
      />
    );
  } else if (view === "stage3") {
    content = (
      <Stage3Screen
        audit={audit}
        onContinue={handleStage3Continue}
        onBack={() => setView("stage2")}
      />
    );
  } else {
    content = <AnalyticsScreen audit={audit} onLock={handleLock} />;
  }

  return (
    <>
      {showAdminEntry && (
        <div className="fixed right-4 top-4 z-10 flex gap-2">
          <button
            type="button"
            onClick={openSettings}
            className="rounded-full border border-line-strong bg-surface px-3 py-1 text-xs font-medium text-ink-muted hover:text-ink"
            title="Settings"
          >
            ⚙️
          </button>
          <button
            type="button"
            onClick={openAdmin}
            className="rounded-full border border-line-strong bg-surface px-3 py-1 text-xs font-medium text-ink-muted hover:text-ink"
          >
            Admin
          </button>
        </div>
      )}
      {content}
    </>
  );
}

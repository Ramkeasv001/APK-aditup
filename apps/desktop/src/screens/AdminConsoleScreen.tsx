import * as Tabs from "@radix-ui/react-tabs";
import clsx from "clsx";
import { Button } from "../components/Button";
import { exitAdmin } from "../ipc/admin";
import { PendingTab } from "./admin/PendingTab";
import { KeywordsTab } from "./admin/KeywordsTab";
import { SynonymsTab } from "./admin/SynonymsTab";
import { MergeRequestsTab } from "./admin/MergeRequestsTab";
import { PublishTab } from "./admin/PublishTab";
import { StatsTab } from "./admin/StatsTab";
import { HistorySearchTab } from "./admin/HistorySearchTab";

interface AdminConsoleScreenProps {
  onExit: () => void;
}

const TABS = [
  { value: "pending", label: "Pending", Component: PendingTab },
  { value: "keywords", label: "Keywords", Component: KeywordsTab },
  { value: "synonyms", label: "Synonyms", Component: SynonymsTab },
  { value: "merge-requests", label: "Merge Requests", Component: MergeRequestsTab },
  { value: "publish", label: "Publish", Component: PublishTab },
  { value: "stats", label: "Stats", Component: StatsTab },
  { value: "history", label: "History Search", Component: HistorySearchTab },
] as const;

export function AdminConsoleScreen({ onExit }: AdminConsoleScreenProps) {
  async function handleExit() {
    await exitAdmin();
    onExit();
  }

  return (
    <div className="mx-auto min-h-screen max-w-4xl px-4 py-10">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-xs font-medium uppercase tracking-wide text-ink-muted">Admin</p>
          <h1 className="font-display text-xl font-bold text-ink">Admin Console</h1>
        </div>
        <Button variant="ghost" onClick={() => void handleExit()}>
          Exit Admin Console
        </Button>
      </div>

      <Tabs.Root defaultValue="pending" className="mt-6">
        <Tabs.List className="flex flex-wrap gap-1 border-b border-line">
          {TABS.map((tab) => (
            <Tabs.Trigger
              key={tab.value}
              value={tab.value}
              className={clsx(
                "rounded-t-lg px-3 py-2 text-sm font-medium text-ink-muted",
                "data-[state=active]:border-b-2 data-[state=active]:border-accent-strong data-[state=active]:text-ink",
              )}
            >
              {tab.label}
            </Tabs.Trigger>
          ))}
        </Tabs.List>
        {TABS.map(({ value, Component }) => (
          <Tabs.Content key={value} value={value} className="pt-4">
            <Component />
          </Tabs.Content>
        ))}
      </Tabs.Root>
    </div>
  );
}

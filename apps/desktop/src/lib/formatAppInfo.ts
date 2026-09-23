export interface AppInfo {
  name: string;
  version: string;
  mode: string;
}

/** Pure formatter kept separate from the component so it's unit-testable without a DOM. */
export function formatAppInfo(info: AppInfo): string {
  return `Scaffold OK — ${info.name} v${info.version} (${info.mode})`;
}

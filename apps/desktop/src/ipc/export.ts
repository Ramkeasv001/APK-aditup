import { invoke } from "@tauri-apps/api/core";

export interface ExportResult {
  filePath: string;
  format: string;
  mimeType: string;
}

export async function exportAuditReport(
  auditId: string,
  format: "xlsx" | "csv",
): Promise<ExportResult> {
  return invoke<ExportResult>("export_audit_report", {
    input: {
      auditId,
      format,
    },
  });
}

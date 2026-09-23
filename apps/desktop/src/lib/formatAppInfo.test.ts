import { describe, expect, it } from "vitest";
import { formatAppInfo } from "./formatAppInfo";

describe("formatAppInfo", () => {
  it("formats name, version, and mode into one status line", () => {
    const result = formatAppInfo({
      name: "ADITUP",
      version: "0.1.0",
      mode: "mode-1-rule-based",
    });

    expect(result).toBe("Scaffold OK — ADITUP v0.1.0 (mode-1-rule-based)");
  });
});

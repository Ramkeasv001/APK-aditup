import { vi } from "vitest";

/**
 * Module 15 — Test Helper Utilities
 * Common testing utilities for component and integration tests
 */

/**
 * Delay execution for specified milliseconds (useful for async test sequencing)
 */
export async function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Wait for condition to be true (with timeout)
 */
export async function waitFor(
  condition: () => boolean,
  options: { timeout?: number; interval?: number } = {}
): Promise<void> {
  const { timeout = 5000, interval = 50 } = options;
  const startTime = Date.now();

  while (!condition()) {
    if (Date.now() - startTime > timeout) {
      throw new Error("Timeout waiting for condition");
    }
    await delay(interval);
  }
}

/**
 * Create a deferred Promise (useful for manual async test control)
 */
export function createDeferred<T>(): {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (reason: unknown) => void;
} {
  let resolve: (value: T) => void;
  let reject: (reason: unknown) => void;

  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });

  return {
    promise,
    resolve: resolve!,
    reject: reject!,
  };
}

/**
 * Simulate user interaction on input element
 */
export function simulateInputChange(
  element: HTMLInputElement,
  value: string
): void {
  element.value = value;
  element.dispatchEvent(new Event("change", { bubbles: true }));
  element.dispatchEvent(new Event("input", { bubbles: true }));
}

/**
 * Simulate button click
 */
export function simulateClick(element: HTMLElement): void {
  element.dispatchEvent(new MouseEvent("click", { bubbles: true }));
}

/**
 * Create mock localStorage for tests
 */
export function createMockLocalStorage(): Storage {
  let store: Record<string, string> = {};

  return {
    length: 0,
    clear: () => {
      store = {};
    },
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => {
      store[key] = String(value);
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    key: (index: number) => Object.keys(store)[index] ?? null,
  };
}

/**
 * Assert that an async function throws an error
 */
export async function expectAsyncThrow(
  fn: () => Promise<unknown>,
  message?: string
): Promise<Error> {
  try {
    await fn();
    throw new Error(`Expected function to throw${message ? `: ${message}` : ""}`);
  } catch (err) {
    if (err instanceof Error && err.message.startsWith("Expected function")) {
      throw err;
    }
    return err as Error;
  }
}

/**
 * Create a spy function with call tracking
 */
export function createSpyFunction<T extends (...args: unknown[]) => unknown>(
  implementation?: T
) {
  const calls: unknown[][] = [];
  const results: unknown[] = [];
  const errors: Error[] = [];

  const spy = vi.fn((...args: unknown[]) => {
    calls.push(args);
    try {
      const result = implementation?.(...args);
      results.push(result);
      return result;
    } catch (err) {
      errors.push(err as Error);
      throw err;
    }
  });

  return {
    fn: spy,
    calls,
    results,
    errors,
    getLastCall: () => calls[calls.length - 1],
    getLastResult: () => results[results.length - 1],
    getLastError: () => errors[errors.length - 1],
  };
}

/**
 * Test data builder for creating complex test objects
 */
export class AuditBuilder {
  private audit: Record<string, unknown> = {
    id: "audit-001",
    name: "Safety Audit",
    location: "Warehouse A",
    status: "stage1",
    observationCount: 0,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };

  withId(id: string): this {
    this.audit.id = id;
    return this;
  }

  withName(name: string): this {
    this.audit.name = name;
    return this;
  }

  withLocation(location: string): this {
    this.audit.location = location;
    return this;
  }

  withStatus(status: string): this {
    this.audit.status = status;
    return this;
  }

  withObservationCount(count: number): this {
    this.audit.observationCount = count;
    return this;
  }

  build(): Record<string, unknown> {
    return { ...this.audit };
  }
}

/**
 * Compare two objects recursively
 */
export function deepEqual(
  actual: unknown,
  expected: unknown,
  path = ""
): { equal: boolean; diffs: string[] } {
  const diffs: string[] = [];

  if (actual === expected) {
    return { equal: true, diffs: [] };
  }

  if (typeof actual !== typeof expected) {
    diffs.push(`${path}: type mismatch (${typeof actual} vs ${typeof expected})`);
    return { equal: false, diffs };
  }

  if (typeof actual !== "object" || actual === null || expected === null) {
    diffs.push(`${path}: value mismatch (${actual} vs ${expected})`);
    return { equal: false, diffs };
  }

  const actualObj = actual as Record<string, unknown>;
  const expectedObj = expected as Record<string, unknown>;
  const allKeys = new Set([
    ...Object.keys(actualObj),
    ...Object.keys(expectedObj),
  ]);

  for (const key of allKeys) {
    const newPath = path ? `${path}.${key}` : key;
    const { equal, diffs: keyDiffs } = deepEqual(
      actualObj[key],
      expectedObj[key],
      newPath
    );
    if (!equal) {
      diffs.push(...keyDiffs);
    }
  }

  return { equal: diffs.length === 0, diffs };
}

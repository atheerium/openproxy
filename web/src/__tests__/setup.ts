import "@testing-library/jest-dom/vitest";

// Mock fetch globally; tests override as needed
if (!globalThis.fetch) {
  // @ts-ignore
  globalThis.fetch = async () => ({ ok: true, json: async () => ({}) } as Response);
}

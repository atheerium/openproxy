import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { useHeaderSearchStore } from "@/store/headerSearchStore";
import { FREE_TIER_SET, isFreeTierProvider } from "@/shared/constants/providers";

// Pure helper mirrored from ProvidersPageClient.matchSearch
function matchSearch(name: string, query: string) {
  return !query.trim() || name.toLowerCase().includes(query.trim().toLowerCase());
}

describe("providers constants - free tier", () => {
  it("FREE_TIER_SET and isFreeTierProvider agree", () => {
    expect(isFreeTierProvider("kilocode")).toBe(true);
    expect(isFreeTierProvider("nvidia")).toBe(true);
    expect(isFreeTierProvider("claude")).toBe(false);
    expect(FREE_TIER_SET.has("kilocode")).toBe(true);
    expect(FREE_TIER_SET.has("openai")).toBe(false);
  });
});

describe("headerSearchStore - register clears stale query", () => {
  beforeEach(() => {
    useHeaderSearchStore.setState({ query: "", placeholder: "", visible: false });
  });
  it("register() clears query so entering providers page does not retain cross-page search", () => {
    const { setQuery, register } = useHeaderSearchStore.getState();
    setQuery("kilo");
    expect(useHeaderSearchStore.getState().query).toBe("kilo");
    register("Search providers...");
    expect(useHeaderSearchStore.getState().query).toBe("");
  });
  it("unregister clears state", () => {
    const { register, unregister } = useHeaderSearchStore.getState();
    register("Search providers...");
    useHeaderSearchStore.getState().setQuery("foo");
    unregister();
    expect(useHeaderSearchStore.getState().query).toBe("");
    expect(useHeaderSearchStore.getState().visible).toBe(false);
  });
});

describe("matchSearch", () => {
  it("empty query matches everything", () => {
    expect(matchSearch("Kilo Code", "")).toBe(true);
  });
  it("case-insensitive substring", () => {
    expect(matchSearch("Kilo Code", "kilo")).toBe(true);
    expect(matchSearch("OpenAI", "KILO")).toBe(false);
  });
  it("trimmed query", () => {
    expect(matchSearch("NVIDIA NIM", "  nvidia ")).toBe(true);
  });
});

describe("ProvidersPageClient fetch error banner", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    useHeaderSearchStore.setState({ query: "", placeholder: "", visible: false });
  });

  it("shows retry banner on 401 and clears on successful retry", async () => {
    let call = 0;
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        if (url.startsWith("/api/providers") || url.startsWith("/api/provider-nodes") || url.startsWith("/api/proxy-pools")) {
          call++;
          // first round: /api/providers 401, others ok; second round: all ok
          if (call <= 1) {
            return { ok: false, status: 401, json: async () => ({ error: "unauthorized" }) } as unknown as Response;
          }
          return { ok: true, status: 200, json: async () => ({ connections: [], nodes: [], proxyPools: [] }) } as unknown as Response;
        }
        return { ok: true, status: 200, json: async () => ({}) } as unknown as Response;
      })
    );

    const { default: ProvidersPageClient } = await import("@/components/providers/ProvidersPageClient");
    render(<ProvidersPageClient />);

    // error banner appears after initial fetch
    await waitFor(() => expect(screen.getByText(/Failed to load providers/)).toBeInTheDocument());
    expect(screen.getByText(/401/)).toBeInTheDocument();

    // retry clears error
    fireEvent.click(screen.getByText("Retry"));
    await waitFor(() => expect(screen.queryByText(/Failed to load providers/)).not.toBeInTheDocument());
    expect(call).toBeGreaterThan(1);
  });

  it("clear search button resets global query when no providers match", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({ ok: true, status: 200, json: async () => ({ connections: [], nodes: [], proxyPools: [] }) } as unknown as Response))
    );
    const { default: ProvidersPageClient } = await import("@/components/providers/ProvidersPageClient");
    render(<ProvidersPageClient />);
    // providers page clears query on mount via register; set after mount to simulate cross-page leak
    await waitFor(() => expect(screen.queryByText(/Failed to load providers/)).not.toBeInTheDocument());
    useHeaderSearchStore.getState().setQuery("zzzNoMatch999");
    await waitFor(() => expect(screen.getByText(/No providers match your search/)).toBeInTheDocument());
    fireEvent.click(screen.getByText("Clear search"));
    expect(useHeaderSearchStore.getState().query).toBe("");
  });
});

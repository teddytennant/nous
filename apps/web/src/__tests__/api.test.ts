import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// We test the API module by importing it and verifying fetch calls
// First, set up fetch mock before importing the module
const mockFetch = vi.fn();
globalThis.fetch = mockFetch;

// Dynamic import to get fresh module per test group
async function importApi() {
  // Clear module cache to get fresh import
  vi.resetModules();
  return import("@/lib/api");
}

describe("API client", () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("node endpoints", () => {
    it("calls /health endpoint correctly", async () => {
      const { node } = await importApi();
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            status: "healthy",
            version: "0.1.0",
            uptime_ms: 60000,
          }),
      });

      const result = await node.health();

      expect(mockFetch).toHaveBeenCalledWith(
        "http://localhost:8080/api/v1/health",
        expect.objectContaining({
          headers: { "Content-Type": "application/json" },
        }),
      );
      expect(result.status).toBe("healthy");
      expect(result.version).toBe("0.1.0");
    });

    it("calls /node endpoint correctly", async () => {
      const { node } = await importApi();
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            protocol: "nous/1.0",
            version: "0.1.0",
            features: ["identity", "messaging"],
          }),
      });

      const result = await node.info();

      expect(mockFetch).toHaveBeenCalledWith(
        "http://localhost:8080/api/v1/node",
        expect.anything(),
      );
      expect(result.features).toEqual(["identity", "messaging"]);
    });
  });

  describe("peers endpoints", () => {
    it("calls /peers for listing", async () => {
      const { peers } = await importApi();
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ peers: [], count: 0 }),
      });

      const result = await peers.list();

      expect(mockFetch).toHaveBeenCalledWith(
        "http://localhost:8080/api/v1/peers",
        expect.anything(),
      );
      expect(result.count).toBe(0);
    });

    it("calls POST /peers for connecting", async () => {
      const { peers } = await importApi();
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            peer_id: "12D3...",
            multiaddr: "/ip4/127.0.0.1/tcp/9000",
            latency_ms: null,
            bytes_sent: 0,
            bytes_recv: 0,
            connected_at: "2026-01-01T00:00:00Z",
            protocols: [],
          }),
      });

      await peers.connect("/ip4/127.0.0.1/tcp/9000");

      expect(mockFetch).toHaveBeenCalledWith(
        "http://localhost:8080/api/v1/peers",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ multiaddr: "/ip4/127.0.0.1/tcp/9000" }),
        }),
      );
    });

    it("calls DELETE /peers/:id for disconnecting", async () => {
      const { peers } = await importApi();
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(undefined),
      });

      await peers.disconnect("peer-123");

      expect(mockFetch).toHaveBeenCalledWith(
        "http://localhost:8080/api/v1/peers/peer-123",
        expect.objectContaining({ method: "DELETE" }),
      );
    });
  });

  describe("error handling", () => {
    it("throws on non-ok response with API error message", async () => {
      const { node } = await importApi();
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        statusText: "Internal Server Error",
        json: () =>
          Promise.resolve({ error: { message: "Database unavailable" } }),
      });

      await expect(node.health()).rejects.toThrow("Database unavailable");
    });

    it("throws with status text when no error body", async () => {
      const { node } = await importApi();
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 503,
        statusText: "Service Unavailable",
        json: () => Promise.reject(new Error("parse error")),
      });

      await expect(node.health()).rejects.toThrow("Service Unavailable");
    });

    it("throws generic message for unknown status", async () => {
      const { node } = await importApi();
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 418,
        statusText: "I'm a teapot",
        json: () => Promise.resolve({}),
      });

      await expect(node.health()).rejects.toThrow("Request failed: 418");
    });
  });
});

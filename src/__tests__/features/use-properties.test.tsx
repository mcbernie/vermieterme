import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useProperties } from "@/features/properties/use-properties";

type Resolver = (value: Response) => void;

const json = (data: unknown, ok = true) => ({ ok, json: async () => data }) as Response;
const prop = (id: string) => ({ id, street: `S${id}`, zip: "1", city: "C", totalShares: 100, _count: { units: 1 } });
const detailOf = (id: string) => ({ ...prop(id), units: [{ id: `u${id}`, propertyId: id, name: `Unit ${id}`, floor: "EG", shares: 1, tenants: [] }] });

/** Routes mocked fetch calls; pending detail requests are resolved manually. */
function setup(manual = true) {
  const pending: Record<string, Resolver> = {};
  const calls: { url: string; method: string; body?: string }[] = [];
  let list = [prop("a"), prop("b")];
  const fetchMock = vi.fn((url: string, init?: RequestInit) => {
    const method = init?.method ?? "GET";
    calls.push({ url, method, body: init?.body as string | undefined });
    if (url === "/api/properties" && method === "GET") return Promise.resolve(json(list));
    if (url === "/api/rent-changes") return Promise.resolve(json([]));
    if (/^\/api\/properties\/\w+$/.test(url) && method === "GET") {
      if (!manual) return Promise.resolve(json(detailOf(url.split("/").pop() as string)));
      return new Promise<Response>((resolve) => {
        pending[url.split("/").pop() as string] = resolve;
      });
    }
    return Promise.resolve(json({}));
  });
  vi.stubGlobal("fetch", fetchMock);
  return { pending, calls, setList: (l: typeof list) => (list = l) };
}

describe("useProperties", () => {
  beforeEach(() => vi.spyOn(console, "error").mockImplementation(() => {}));
  afterEach(() => vi.unstubAllGlobals());

  it("never exposes the units of a previously expanded property", async () => {
    const { pending } = setup();
    const { result } = renderHook(() => useProperties());
    await waitFor(() => expect(result.current.loading).toBe(false));

    let first: Promise<void>;
    let second: Promise<void>;
    act(() => {
      first = result.current.toggleExpand("a");
    });
    act(() => {
      second = result.current.toggleExpand("b");
    });
    expect(result.current.detail).toBeNull();

    await act(async () => {
      pending.b(json(detailOf("b")));
      await second;
    });
    await act(async () => {
      pending.a(json(detailOf("a")));
      await first;
    });
    expect(result.current.expandedId).toBe("b");
    expect(result.current.detail?.id).toBe("b");
  });

  it("drops a late response after collapsing", async () => {
    const { pending } = setup();
    const { result } = renderHook(() => useProperties());
    await waitFor(() => expect(result.current.loading).toBe(false));

    let first: Promise<void>;
    act(() => {
      first = result.current.toggleExpand("a");
    });
    await act(async () => {
      await result.current.toggleExpand("a");
    });
    await act(async () => {
      pending.a(json(detailOf("a")));
      await first;
    });
    expect(result.current.expandedId).toBeNull();
    expect(result.current.detail).toBeNull();
  });

  it("sends payloads to the right URLs and reloads after delete", async () => {
    const { calls, setList } = setup(false);
    const { result } = renderHook(() => useProperties());
    await waitFor(() => expect(result.current.loading).toBe(false));
    const values = { name: "W1", floor: "EG", shares: 5 };

    await act(async () => {
      await result.current.saveUnit({ propertyId: "a", values });
    });
    const post = calls.find((c) => c.url === "/api/units" && c.method === "POST");
    expect(JSON.parse(post?.body ?? "{}")).toEqual({ ...values, propertyId: "a" });

    await act(async () => {
      await result.current.saveProperty({ street: "x", zip: "1", city: "c", totalShares: 1 }, "a");
    });
    expect(calls.some((c) => c.url === "/api/properties/a" && c.method === "PUT")).toBe(true);

    setList([prop("b")]);
    await act(async () => {
      await result.current.deleteProperty("a");
    });
    expect(calls.some((c) => c.url === "/api/properties/a" && c.method === "DELETE")).toBe(true);
    expect(result.current.properties.map((p) => p.id)).toEqual(["b"]);
  });

  it("reports a failed save", async () => {
    setup(false);
    const { result } = renderHook(() => useProperties());
    await waitFor(() => expect(result.current.loading).toBe(false));
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve(json({}, false))));
    let ok = true;
    await act(async () => {
      ok = await result.current.saveProperty({ street: "x", zip: "1", city: "c", totalShares: 1 });
    });
    expect(ok).toBe(false);
  });
});

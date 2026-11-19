import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useSave } from "@/features/properties/use-save";

describe("useSave", () => {
  it("calls onDone after a successful action", async () => {
    const { result } = renderHook(() => useSave());
    const onDone = vi.fn();
    await act(async () => {
      await result.current.run(async () => true, onDone);
    });
    expect(onDone).toHaveBeenCalledTimes(1);
    expect(result.current.error).toBeNull();
  });

  it("ignores a second call while one is running", async () => {
    const { result } = renderHook(() => useSave());
    let resolve: (ok: boolean) => void = () => {};
    const action = vi.fn(() => new Promise<boolean>((r) => (resolve = r)));
    let first: Promise<void>;
    act(() => {
      first = result.current.run(action, vi.fn());
    });
    expect(result.current.saving).toBe(true);
    await act(async () => {
      await result.current.run(action, vi.fn());
    });
    expect(action).toHaveBeenCalledTimes(1);
    await act(async () => {
      resolve(true);
      await first;
    });
  });

  it("sets an error and resets saving when the action returns false", async () => {
    const { result } = renderHook(() => useSave());
    const onDone = vi.fn();
    await act(async () => {
      await result.current.run(async () => false, onDone);
    });
    expect(onDone).not.toHaveBeenCalled();
    expect(result.current.saving).toBe(false);
    expect(result.current.error).toMatch(/fehlgeschlagen/);
  });

  it("treats a rejection as failure and allows a retry", async () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    const { result } = renderHook(() => useSave());
    await act(async () => {
      await result.current.run(async () => Promise.reject(new Error("boom")), vi.fn());
    });
    expect(result.current.saving).toBe(false);
    expect(result.current.error).not.toBeNull();
    const onDone = vi.fn();
    await act(async () => {
      await result.current.run(async () => true, onDone);
    });
    expect(onDone).toHaveBeenCalledTimes(1);
    expect(result.current.error).toBeNull();
  });
});

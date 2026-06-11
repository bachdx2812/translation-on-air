import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "./use-translation";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const mockInvoke = vi.mocked(invoke);

describe("useTranslation", () => {
  beforeEach(() => mockInvoke.mockReset());

  it("stores the result on success", async () => {
    mockInvoke.mockResolvedValue({ translation: "xin chào", segments: [] });
    const { result } = renderHook(() => useTranslation());

    await act(async () => {
      await result.current.run("hello", "vi");
    });

    expect(result.current.state).toEqual({
      status: "result",
      result: { translation: "xin chào", segments: [] },
    });
  });

  it("applies only the latest of concurrent requests (stale guard)", async () => {
    let resolveSlow: (v: unknown) => void = () => {};
    mockInvoke
      .mockImplementationOnce(() => new Promise((res) => (resolveSlow = res)))
      .mockResolvedValueOnce({ translation: "fast", segments: [] });

    const { result } = renderHook(() => useTranslation());

    await act(async () => {
      const slow = result.current.run("x", "vi");
      const fast = result.current.run("x", "en");
      resolveSlow({ translation: "slow", segments: [] });
      await Promise.all([slow, fast]);
    });

    expect(result.current.state).toEqual({
      status: "result",
      result: { translation: "fast", segments: [] },
    });
  });
});

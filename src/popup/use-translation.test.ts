import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "./use-translation";

// Minimal Channel stand-in: tests push partials by calling `onmessage` on the
// channel the hook passed to invoke.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  Channel: class {
    onmessage: (msg: unknown) => void = () => {};
  },
}));
const mockInvoke = vi.mocked(invoke);

type PartialChannel = { onmessage: (msg: string) => void };
const channelOf = (call: number) =>
  (mockInvoke.mock.calls[call][1] as { onPartial: PartialChannel }).onPartial;

describe("useTranslation", () => {
  // Block body on purpose: a function returned from beforeEach runs as teardown,
  // and mockReset() returns the mock itself.
  beforeEach(() => {
    mockInvoke.mockReset();
  });

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

  it("shows streamed partials while loading", async () => {
    mockInvoke.mockImplementation(() => new Promise(() => {}));
    const { result } = renderHook(() => useTranslation());

    act(() => {
      void result.current.run("hello", "vi");
    });
    expect(result.current.state).toEqual({ status: "loading", partial: "" });

    act(() => channelOf(0).onmessage("xin"));
    expect(result.current.state).toEqual({ status: "loading", partial: "xin" });
  });

  it("ignores partials from a stale request or after the result", async () => {
    let resolveFirst: (v: unknown) => void = () => {};
    mockInvoke
      .mockImplementationOnce(() => new Promise((res) => (resolveFirst = res)))
      .mockImplementationOnce(() => new Promise(() => {}));
    const { result } = renderHook(() => useTranslation());

    await act(async () => {
      const first = result.current.run("x", "vi");
      resolveFirst({ translation: "done", segments: [] });
      await first;
    });
    act(() => channelOf(0).onmessage("late"));
    expect(result.current.state).toEqual({
      status: "result",
      result: { translation: "done", segments: [] },
    });

    act(() => {
      void result.current.run("x", "en");
    });
    act(() => channelOf(0).onmessage("stale"));
    expect(result.current.state).toEqual({ status: "loading", partial: "" });
  });
});

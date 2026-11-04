import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, cleanup, waitFor, renderHook } from "@testing-library/react";
import {
  AboutSection,
  UpdateBanner,
  UPGRADE_COMMAND,
  useUpdateStatus,
} from "./about-section";
import { checkForUpdate, copyText, openUpdatePage, updateStatus } from "../shared/tauri-api";
import type { UpdateStatus } from "../shared/types";

vi.mock("../shared/tauri-api", () => ({
  updateStatus: vi.fn(),
  checkForUpdate: vi.fn(),
  openUpdatePage: vi.fn().mockResolvedValue(undefined),
  copyText: vi.fn().mockResolvedValue(undefined),
}));

const upToDate: UpdateStatus = { current: "0.4.2", latest: "0.4.2", available: false, error: null };
const newer: UpdateStatus = { current: "0.4.2", latest: "0.4.3", available: true, error: null };
const unchecked: UpdateStatus = { current: "0.4.2", latest: null, available: false, error: null };

describe("AboutSection", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(cleanup);

  it("shows the version and the last check result", () => {
    const { getByText } = render(
      <AboutSection status={upToDate} checking={false} onCheck={() => {}} />,
    );
    expect(getByText("0.4.2")).toBeTruthy();
    expect(getByText(/Đang dùng bản mới nhất\./)).toBeTruthy();
  });

  it("explains a failed check", () => {
    const failed = { ...unchecked, error: "network" };
    const { getByText } = render(
      <AboutSection status={failed} checking={false} onCheck={() => {}} />,
    );
    expect(getByText(/Không kết nối được tới GitHub\./)).toBeTruthy();
  });

  it("checks on demand and disables the button meanwhile", () => {
    const onCheck = vi.fn();
    const { getByText, rerender } = render(
      <AboutSection status={upToDate} checking={false} onCheck={onCheck} />,
    );
    fireEvent.click(getByText("Kiểm tra"));
    expect(onCheck).toHaveBeenCalledOnce();

    rerender(<AboutSection status={upToDate} checking={true} onCheck={onCheck} />);
    expect((getByText("Kiểm tra") as HTMLButtonElement).disabled).toBe(true);
  });
});

describe("UpdateBanner", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(cleanup);

  it("copies the brew command and opens the release page", async () => {
    const { getByText, findByText } = render(<UpdateBanner status={newer} />);
    expect(getByText("Có bản mới 0.4.3")).toBeTruthy();

    fireEvent.click(getByText("Chép lệnh cập nhật"));
    expect(copyText).toHaveBeenCalledWith(UPGRADE_COMMAND);
    expect(await findByText("Đã chép lệnh")).toBeTruthy();

    fireEvent.click(getByText("Xem bản phát hành"));
    expect(openUpdatePage).toHaveBeenCalledOnce();
  });
});

describe("useUpdateStatus", () => {
  beforeEach(() => vi.clearAllMocks());

  it("uses the app's last result without asking GitHub again", async () => {
    vi.mocked(updateStatus).mockResolvedValue(upToDate);
    const { result } = renderHook(() => useUpdateStatus());

    await waitFor(() => expect(result.current.status).toEqual(upToDate));
    expect(checkForUpdate).not.toHaveBeenCalled();
  });

  it("checks when the app has no result yet", async () => {
    vi.mocked(updateStatus).mockResolvedValue(unchecked);
    vi.mocked(checkForUpdate).mockResolvedValue(newer);
    const { result } = renderHook(() => useUpdateStatus());

    await waitFor(() => expect(result.current.status).toEqual(newer));
    expect(checkForUpdate).toHaveBeenCalledOnce();
  });
});

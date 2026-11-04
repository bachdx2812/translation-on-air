import { useEffect, useRef, useState } from "react";
import { checkForUpdate, copyText, openUpdatePage, updateStatus } from "../shared/tauri-api";
import type { UpdateStatus } from "../shared/types";
import { IconCheck, IconCopy } from "../shared/icons";
import { SettingGroup, SettingRow } from "./setting-row";

export const UPGRADE_COMMAND = "brew upgrade --cask translation-on-air";

const ERRORS: Record<string, string> = {
  network: "Không kết nối được tới GitHub.",
  "http-403": "GitHub đang tạm giới hạn số lần kiểm tra, thử lại sau.",
};

/** Update status shared by the banner and the About group. Uses the result
 * the app already has (it checks daily) and only asks GitHub if there is none. */
export function useUpdateStatus() {
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [checking, setChecking] = useState(false);

  const check = async () => {
    setChecking(true);
    try {
      setStatus(await checkForUpdate());
    } finally {
      setChecking(false);
    }
  };

  useEffect(() => {
    void updateStatus().then((s) => {
      setStatus(s);
      if (!s.latest && !s.error) void check();
    });
  }, []);

  return { status, checking, check };
}

/** Shown at the top of Settings while a newer release exists. */
export function UpdateBanner({ status }: { status: UpdateStatus }) {
  const [copied, setCopied] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);

  const copy = async () => {
    await copyText(UPGRADE_COMMAND);
    setCopied(true);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => setCopied(false), 1500);
  };

  return (
    <section className="update-banner" role="status">
      <p className="update-title">Có bản mới {status.latest}</p>
      <p className="row-hint">
        Bạn đang dùng {status.current}. Cập nhật bằng lệnh <code>{UPGRADE_COMMAND}</code>
      </p>
      <div className="update-actions">
        <button type="button" className="btn btn-primary" onClick={() => void copy()}>
          {copied ? <IconCheck /> : <IconCopy />}
          {copied ? "Đã chép lệnh" : "Chép lệnh cập nhật"}
        </button>
        <button type="button" className="btn" onClick={() => void openUpdatePage()}>
          Xem bản phát hành
        </button>
      </div>
    </section>
  );
}

function statusText(status: UpdateStatus | null, checking: boolean): string {
  if (checking || !status) return "Đang kiểm tra…";
  if (status.error) return ERRORS[status.error] ?? "Không kiểm tra được bản mới.";
  if (status.available) return `Có bản mới ${status.latest} (xem phía trên).`;
  if (status.latest) return "Đang dùng bản mới nhất.";
  return "Chưa kiểm tra.";
}

export function AboutSection({
  status,
  checking,
  onCheck,
}: {
  status: UpdateStatus | null;
  checking: boolean;
  onCheck: () => void;
}) {
  return (
    <SettingGroup title="Giới thiệu">
      <SettingRow label="Phiên bản">
        <span className="version-value">{status?.current ?? "…"}</span>
      </SettingRow>
      <SettingRow
        label="Bản cập nhật"
        hint={`${statusText(status, checking)} App tự kiểm tra mỗi ngày một lần.`}
      >
        <button type="button" className="btn" disabled={checking} onClick={onCheck}>
          Kiểm tra
        </button>
      </SettingRow>
    </SettingGroup>
  );
}

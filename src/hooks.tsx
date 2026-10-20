import { useCallback, useEffect, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errMsg, type CmdError, type TunnelView } from "./api";
import { ConfirmDialog, MessageDialog } from "./components/Modal";
import { useI18n } from "./i18n";

/** 訂閱核心發布的通道狀態；兩個 UI 共用同一份來源 */
export function useTunnels() {
  const [views, setViews] = useState<TunnelView[] | null>(null);
  useEffect(() => {
    const refresh = () => api.getState().then(setViews).catch(() => {});
    refresh();
    const subs = [
      listen<TunnelView[]>("state", (e) => setViews(e.payload)),
      listen("panel-shown", refresh),
    ];
    window.addEventListener("focus", refresh);
    return () => {
      subs.forEach((p) => p.then((un) => un()));
      window.removeEventListener("focus", refresh);
    };
  }, []);
  return views;
}

interface ConfirmState {
  title: string;
  message: ReactNode;
  confirmText: string;
  danger?: boolean;
  onConfirm: () => void;
}

/** 連接／停止等操作，包含 port 衝突時的「停止並切換」確認 */
export function useTunnelActions() {
  const { t, tx } = useI18n();
  const [busy, setBusy] = useState<Record<string, boolean>>({});
  const [confirm, setConfirm] = useState<ConfirmState | null>(null);
  const [error, setError] = useState<string | null>(null);

  const guard = useCallback(async (id: string, fn: () => Promise<unknown>) => {
    setBusy((b) => ({ ...b, [id]: true }));
    try {
      await fn();
    } finally {
      setBusy((b) => ({ ...b, [id]: false }));
    }
  }, []);

  const connect = useCallback(
    (v: TunnelView) =>
      guard(v.tunnel.id, async () => {
        try {
          await api.connect(v.tunnel.id);
        } catch (e) {
          const err = e as CmdError;
          if (err?.code === "conflict" && err.conflicts?.length) {
            const c = err.conflicts[0];
            setConfirm({
              title: t("conflict.title"),
              message: tx("conflict.message", { other: c.name, endpoint: <code>{c.endpoint}</code>, name: v.tunnel.name }),
              confirmText: t("conflict.button"),
              onConfirm: () =>
                guard(v.tunnel.id, () => api.switchTo(v.tunnel.id).catch((e) => setError(errMsg(e)))),
            });
          } else {
            setError(errMsg(e));
          }
        }
      }),
    [guard, t, tx],
  );

  const disconnect = useCallback(
    (v: TunnelView) => guard(v.tunnel.id, () => api.disconnect(v.tunnel.id).catch((e) => setError(errMsg(e)))),
    [guard],
  );

  const restart = useCallback(
    (v: TunnelView) => guard(v.tunnel.id, () => api.restart(v.tunnel.id).catch((e) => setError(errMsg(e)))),
    [guard],
  );

  const ui = (
    <>
      {confirm && (
        <ConfirmDialog
          title={confirm.title}
          confirmText={confirm.confirmText}
          danger={confirm.danger}
          onCancel={() => setConfirm(null)}
          onConfirm={() => {
            setConfirm(null);
            confirm.onConfirm();
          }}
        >
          {confirm.message}
        </ConfirmDialog>
      )}
      {error && (
        <MessageDialog title={t("common.operationFailed")} onClose={() => setError(null)}>
          {error}
        </MessageDialog>
      )}
    </>
  );

  return { busy, connect, disconnect, restart, guard, setConfirm, setError, ui };
}

import type { Runtime } from "../api";
import { useI18n } from "../i18n";

const GLYPH: Record<Runtime["status"], string> = {
  disconnected: "○",
  connecting: "◌",
  waitingAuth: "◍",
  connected: "●",
  reconnecting: "↻",
  stopping: "◌",
  failed: "✕",
};

/** 狀態以符號 + 文字表示，不只依賴顏色 */
export function StatusBadge({ runtime, compact }: { runtime: Runtime; compact?: boolean }) {
  const { t } = useI18n();
  const s = runtime.status;
  let text = t(`status.${s}`);
  if (s === "reconnecting" && runtime.attempt) text += t("status.attempt", { n: runtime.attempt });
  return (
    <span className={`status status-${s}`} title={runtime.message ?? text}>
      <span className="status-glyph" aria-hidden>
        {GLYPH[s]}
      </span>
      {text}
      {!compact && runtime.destCheck && s === "connected" && (
        <span className={`dest ${runtime.destCheck.ok ? "dest-ok" : "dest-bad"}`} title={runtime.destCheck.message}>
          {runtime.destCheck.ok ? t("status.destOk") : t("status.destBad")}
        </span>
      )}
    </span>
  );
}

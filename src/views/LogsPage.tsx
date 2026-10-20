import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type LogEntry, type TunnelView } from "../api";
import { useI18n } from "../i18n";

export function LogsPage({
  views,
  filter,
  onFilter,
}: {
  views: TunnelView[];
  filter: string | null;
  onFilter: (id: string | null) => void;
}) {
  const { t } = useI18n();
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [showDebug, setShowDebug] = useState(false);
  const [follow, setFollow] = useState(true);
  const bottom = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api.getLogs(filter).then(setEntries);
    const un = listen<LogEntry>("log", (e) => {
      if (filter && e.payload.tunnelId !== filter) return;
      setEntries((xs) => [...xs.slice(-2999), e.payload]);
    });
    return () => {
      un.then((f) => f());
    };
  }, [filter]);

  const shown = useMemo(() => entries.filter((e) => showDebug || e.level !== "debug"), [entries, showDebug]);
  const names = useMemo(() => Object.fromEntries(views.map((v) => [v.tunnel.id, v.tunnel.name])), [views]);

  useEffect(() => {
    if (follow) bottom.current?.scrollIntoView({ block: "end" });
  }, [shown, follow]);

  return (
    <div className="page page-fill">
      <header className="page-head">
        <div>
          <h1>{t("logs.title")}</h1>
          <p className="muted">{t("logs.subtitle")}</p>
        </div>
        <div className="row gap center">
          <select value={filter ?? ""} onChange={(e) => onFilter(e.target.value || null)} aria-label={t("logs.filter")}>
            <option value="">{t("logs.allTunnels")}</option>
            {views.map((v) => (
              <option key={v.tunnel.id} value={v.tunnel.id}>
                {v.tunnel.name}
              </option>
            ))}
          </select>
          <label className="check">
            <input type="checkbox" checked={showDebug} onChange={(e) => setShowDebug(e.target.checked)} />
            {t("logs.showDebug")}
          </label>
          <label className="check">
            <input type="checkbox" checked={follow} onChange={(e) => setFollow(e.target.checked)} />
            {t("logs.follow")}
          </label>
          <button onClick={() => api.clearLogs().then(() => setEntries([]))}>{t("logs.clear")}</button>
          <button onClick={() => api.openLogDir()}>{t("logs.openDir")}</button>
        </div>
      </header>
      <div className="log-view" role="log">
        {shown.length === 0 && <div className="muted pad">{t("logs.empty")}</div>}
        {shown.map((e) => (
          <div key={e.id} className={`log-line log-${e.level}`}>
            <span className="log-ts">{e.ts.slice(11, 19)}</span>
            <span className="log-level">{t(`level.${e.level}`)}</span>
            {e.tunnelId && !filter && <span className="log-tunnel">{names[e.tunnelId] ?? t("logs.testTunnel")}</span>}
            <span className="log-msg">{e.message}</span>
          </div>
        ))}
        <div ref={bottom} />
      </div>
    </div>
  );
}

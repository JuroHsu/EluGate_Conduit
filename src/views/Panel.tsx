import { useEffect } from "react";
import { api, endpoint, isActive } from "../api";
import { useTunnelActions, useTunnels } from "../hooks";
import { useI18n } from "../i18n";
import { StatusBadge } from "../components/StatusBadge";
import logo from "../assets/logo.svg";

/** 系統匣快速面板 */
export function Panel() {
  const { t } = useI18n();
  const views = useTunnels();
  const actions = useTunnelActions();
  const connected = views?.filter((v) => v.runtime.status === "connected").length ?? 0;
  const anyActive = views?.some((v) => isActive(v.runtime.status));

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && api.hidePanel();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="panel">
      <header className="panel-head">
        <img src={logo} alt="" width={28} height={28} />
        <div>
          <div className="strong">EluGate Conduit</div>
          <div className="muted small">{connected ? t("brand.connected", { count: connected }) : t("brand.none")}</div>
        </div>
      </header>
      <div className="panel-list">
        {views?.length === 0 && <div className="muted pad">{t("panel.empty")}</div>}
        {views?.map((v) => {
          const s = v.runtime.status;
          const busy = actions.busy[v.tunnel.id] || s === "stopping";
          return (
            <div className="panel-item" key={v.tunnel.id}>
              <div className="panel-item-main">
                <div className="strong ellipsis">{v.tunnel.name}</div>
                <div className="mono small muted">{endpoint(v.tunnel)}</div>
                <StatusBadge runtime={v.runtime} compact />
                {s === "failed" && v.runtime.message && (
                  <div className="status-detail" title={v.runtime.message}>
                    {v.runtime.message}
                  </div>
                )}
              </div>
              {isActive(s) ? (
                <button disabled={busy} onClick={() => actions.disconnect(v)}>
                  {t("action.stop")}
                </button>
              ) : (
                <button className="primary" disabled={busy} onClick={() => actions.connect(v)}>
                  {t("action.connect")}
                </button>
              )}
            </div>
          );
        })}
      </div>
      <footer className="panel-foot">
        <button
          onClick={() => {
            api.showMain();
            api.hidePanel();
          }}
        >
          {t("panel.openMain")}
        </button>
        <button disabled={!anyActive} onClick={() => api.stopAll()}>
          {t("tunnels.stopAll")}
        </button>
      </footer>
      {actions.ui}
    </div>
  );
}

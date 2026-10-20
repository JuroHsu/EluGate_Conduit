import { useState } from "react";
import { api, destination, endpoint, errMsg, isActive, newTunnel, type Tunnel, type TunnelView } from "../api";
import { useTunnelActions } from "../hooks";
import { useI18n } from "../i18n";
import { StatusBadge } from "../components/StatusBadge";
import { TunnelDialog } from "./TunnelDialog";

export function TunnelsPage({ views, onShowLogs }: { views: TunnelView[] | null; onShowLogs: (id: string) => void }) {
  const { t, tx } = useI18n();
  const actions = useTunnelActions();
  const [editing, setEditing] = useState<Tunnel | null>(null);
  const [copied, setCopied] = useState<string | null>(null);

  const copy = async (v: TunnelView) => {
    await navigator.clipboard.writeText(endpoint(v.tunnel));
    setCopied(v.tunnel.id);
    setTimeout(() => setCopied(null), 1500);
  };

  const remove = (v: TunnelView) =>
    actions.setConfirm({
      title: t("delete.title"),
      message: t(isActive(v.runtime.status) ? "delete.running" : "delete.confirm", { name: v.tunnel.name }),
      confirmText: t("delete.button"),
      danger: true,
      onConfirm: () =>
        actions.guard(v.tunnel.id, () => api.deleteTunnel(v.tunnel.id).catch((e) => actions.setError(errMsg(e)))),
    });

  const anyActive = views?.some((v) => isActive(v.runtime.status));

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>{t("tunnels.title")}</h1>
          <p className="muted">{tx("tunnels.subtitle", { cmd: <code>ssh -N -L</code> })}</p>
        </div>
        <div className="row gap">
          <button disabled={!anyActive} onClick={() => api.stopAll()}>
            {t("tunnels.stopAll")}
          </button>
          <button className="primary" onClick={() => setEditing(newTunnel())}>
            {t("tunnels.add")}
          </button>
        </div>
      </header>

      {views && views.length === 0 && (
        <div className="empty">
          <div className="empty-title">{t("tunnels.emptyTitle")}</div>
          <p className="muted">
            {tx("tunnels.emptyText", {
              local: <code>127.0.0.1:16000</code>,
              host: <code>my_server</code>,
              dest: <code>127.0.0.1:80</code>,
            })}
          </p>
          <button className="primary" onClick={() => setEditing(newTunnel())}>
            {t("tunnels.createFirst")}
          </button>
        </div>
      )}

      {views && views.length > 0 && (
        <div className="table-wrap">
          <table className="table">
            <thead>
              <tr>
                <th>{t("tunnels.colName")}</th>
                <th>{t("tunnels.colHost")}</th>
                <th>{t("tunnels.colLocal")}</th>
                <th>{t("tunnels.colDest")}</th>
                <th>{t("tunnels.colStatus")}</th>
                <th className="right">{t("tunnels.colActions")}</th>
              </tr>
            </thead>
            <tbody>
              {views.map((v) => {
                const tn = v.tunnel;
                const s = v.runtime.status;
                const busy = actions.busy[tn.id] || s === "stopping";
                return (
                  <tr key={tn.id}>
                    <td>
                      <div className="strong">{tn.name}</div>
                      {v.pendingChanges && <div className="tag warn">{t("tunnels.pending")}</div>}
                    </td>
                    <td>
                      <code>{tn.sshHost}</code>
                    </td>
                    <td>
                      <button className="link mono" title={t("tunnels.copyTitle")} onClick={() => copy(v)}>
                        {endpoint(tn)}
                      </button>
                      {copied === tn.id && <span className="tag ok">{t("common.copied")}</span>}
                    </td>
                    <td className="mono">{destination(tn)}</td>
                    <td>
                      <StatusBadge runtime={v.runtime} />
                      {v.runtime.message && (s === "failed" || s === "reconnecting") && (
                        <div className="status-detail" title={v.runtime.message}>
                          {v.runtime.message}
                        </div>
                      )}
                      {s === "connected" && v.runtime.destCheck && !v.runtime.destCheck.ok && (
                        <div className="status-detail">{v.runtime.destCheck.message}</div>
                      )}
                    </td>
                    <td className="right nowrap">
                      {v.pendingChanges && (
                        <button disabled={busy} onClick={() => actions.restart(v)}>
                          {t("tunnels.apply")}
                        </button>
                      )}
                      {isActive(s) ? (
                        <button disabled={busy} onClick={() => actions.disconnect(v)}>
                          {t("action.stop")}
                        </button>
                      ) : (
                        <button className="primary" disabled={busy} onClick={() => actions.connect(v)}>
                          {t("action.connect")}
                        </button>
                      )}
                      <RowMenu
                        label={t("menu.more")}
                        items={[
                          { label: t("menu.edit"), onClick: () => setEditing(tn) },
                          {
                            label: t("menu.duplicate"),
                            onClick: () => api.duplicateTunnel(tn.id).catch((e) => actions.setError(errMsg(e))),
                          },
                          {
                            label: t("menu.checkDest"),
                            disabled: s !== "connected",
                            onClick: () => api.checkDestination(tn.id),
                          },
                          { label: t("menu.logs"), onClick: () => onShowLogs(tn.id) },
                          { label: t("menu.delete"), danger: true, onClick: () => remove(v) },
                        ]}
                      />
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {editing && (
        <TunnelDialog
          initial={editing}
          running={views?.find((v) => v.tunnel.id === editing.id && isActive(v.runtime.status)) != null}
          onClose={() => setEditing(null)}
        />
      )}
      {actions.ui}
    </div>
  );
}

function RowMenu({
  label,
  items,
}: {
  label: string;
  items: { label: string; onClick: () => void; danger?: boolean; disabled?: boolean }[];
}) {
  const [open, setOpen] = useState(false);
  return (
    <span className="menu-anchor">
      <button className="icon-btn" aria-label={label} aria-expanded={open} onClick={() => setOpen((o) => !o)}>
        ⋯
      </button>
      {open && (
        <>
          <div className="menu-backdrop" onClick={() => setOpen(false)} />
          <div className="menu" role="menu">
            {items.map((i) => (
              <button
                key={i.label}
                role="menuitem"
                className={i.danger ? "danger-text" : ""}
                disabled={i.disabled}
                onClick={() => {
                  setOpen(false);
                  i.onClick();
                }}
              >
                {i.label}
              </button>
            ))}
          </div>
        </>
      )}
    </span>
  );
}

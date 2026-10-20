import { useEffect, useState } from "react";
import { api, errMsg, type CmdError, type HostEntry, type TestResult, type Tunnel, type Validation } from "../api";
import { Modal } from "../components/Modal";
import { useI18n } from "../i18n";

const STAGE_ICON = { ok: "✓", failed: "✕", skipped: "–" } as const;

export function TunnelDialog({ initial, running, onClose }: { initial: Tunnel; running: boolean; onClose: () => void }) {
  const { t: tr } = useI18n();
  const [t, setT] = useState<Tunnel>(initial);
  const [hosts, setHosts] = useState<HostEntry[]>([]);
  const [validation, setValidation] = useState<Validation>({ errors: {}, warnings: [] });
  const [touched, setTouched] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);
  const [test, setTest] = useState<TestResult | null>(null);
  const [effective, setEffective] = useState<Record<string, string> | null>(null);

  useEffect(() => {
    api.listHosts(t.sshConfigFile).then((l) => setHosts(l.hosts)).catch(() => setHosts([]));
  }, [t.sshConfigFile]);

  useEffect(() => {
    const h = setTimeout(() => api.validateTunnel(t).then(setValidation).catch(() => {}), 200);
    return () => clearTimeout(h);
  }, [t]);

  // 顯示 OpenSSH 解析後的實際連線目標，方便確認別名
  useEffect(() => {
    setEffective(null);
    if (!t.sshHost.trim()) return;
    const h = setTimeout(
      () =>
        api
          .effectiveHost(t.sshHost.trim(), t.sshConfigFile)
          .then((kv) => setEffective(Object.fromEntries(kv)))
          .catch(() => setEffective(null)),
      400,
    );
    return () => clearTimeout(h);
  }, [t.sshHost, t.sshConfigFile]);

  const set = <K extends keyof Tunnel>(k: K, v: Tunnel[K]) => {
    setT((x) => ({ ...x, [k]: v }));
    setTest(null);
  };
  const port = (s: string) => {
    const n = Number(s.replace(/\D/g, ""));
    return Number.isFinite(n) ? Math.min(n, 65535) : 0;
  };
  const err = (k: string) => (touched ? validation.errors[k] : undefined);

  const save = async () => {
    setTouched(true);
    if (Object.keys(validation.errors).length) return;
    setSaving(true);
    setError(null);
    try {
      await api.saveTunnel(t);
      onClose();
    } catch (e) {
      const ce = e as CmdError;
      if (ce?.fields) setValidation((v) => ({ ...v, errors: ce.fields! }));
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  const runTest = async () => {
    setTouched(true);
    setTesting(true);
    setTest(null);
    setError(null);
    try {
      setTest(await api.testTunnel(t));
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setTesting(false);
    }
  };

  const effectiveText = () => {
    if (!effective) return undefined;
    const target = `${effective.user ?? ""}@${effective.hostname ?? ""}:${effective.port ?? ""}`;
    const jump = effective.proxyjump && effective.proxyjump !== "none" ? tr("field.via", { jump: effective.proxyjump }) : "";
    return tr("field.effective", { target }) + jump;
  };

  const field = (k: keyof Tunnel & string, label: string, input: React.ReactNode, hint?: string) => (
    <label className={`field ${err(k) ? "has-error" : ""}`}>
      <span className="field-label">{label}</span>
      {input}
      {err(k) ? <span className="field-error">{err(k)}</span> : hint && <span className="field-hint">{hint}</span>}
    </label>
  );

  return (
    <Modal
      title={tr(initial.id ? "tunnel.editTitle" : "tunnel.newTitle")}
      onClose={onClose}
      wide
      footer={
        <>
          <button onClick={runTest} disabled={testing}>
            {tr(testing ? "tunnel.testing" : "tunnel.test")}
          </button>
          <span className="spacer" />
          <button onClick={onClose}>{tr("common.cancel")}</button>
          <button className="primary" onClick={save} disabled={saving}>
            {tr("common.save")}
          </button>
        </>
      }
    >
      <div className="form-grid">
        {field(
          "name",
          tr("field.name"),
          <input value={t.name} autoFocus onChange={(e) => set("name", e.target.value)} placeholder={tr("field.namePlaceholder")} />,
        )}
        {field(
          "sshHost",
          tr("field.sshHost"),
          <>
            <input
              list="ssh-hosts"
              value={t.sshHost}
              onChange={(e) => set("sshHost", e.target.value)}
              placeholder={tr("field.sshHostPlaceholder")}
              spellCheck={false}
            />
            <datalist id="ssh-hosts">
              {hosts.map((h) => (
                <option key={h.alias} value={h.alias} />
              ))}
            </datalist>
          </>,
          effectiveText(),
        )}
        <div className="field-row">
          {field(
            "bindAddress",
            tr("field.bind"),
            <input value={t.bindAddress} onChange={(e) => set("bindAddress", e.target.value)} spellCheck={false} />,
            tr("field.bindHint"),
          )}
          {field(
            "localPort",
            tr("field.localPort"),
            <input inputMode="numeric" value={t.localPort || ""} onChange={(e) => set("localPort", port(e.target.value))} />,
            tr("field.localPortHint"),
          )}
        </div>
        <div className="field-row">
          {field(
            "destHost",
            tr("field.destHost"),
            <input value={t.destHost} onChange={(e) => set("destHost", e.target.value)} spellCheck={false} />,
            tr("field.destHostHint"),
          )}
          {field(
            "destPort",
            tr("field.destPort"),
            <input inputMode="numeric" value={t.destPort || ""} onChange={(e) => set("destPort", port(e.target.value))} />,
          )}
        </div>
        <details className="advanced">
          <summary>{tr("tunnel.advanced")}</summary>
          {field(
            "sshConfigFile",
            tr("field.configFile"),
            <input
              value={t.sshConfigFile ?? ""}
              onChange={(e) => set("sshConfigFile", e.target.value || null)}
              placeholder={tr("field.configFilePlaceholder")}
              spellCheck={false}
            />,
          )}
        </details>
        <label className="check">
          <input type="checkbox" checked={t.autoReconnect} onChange={(e) => set("autoReconnect", e.target.checked)} />
          {tr("field.autoReconnect")}
        </label>
        <label className="check">
          <input type="checkbox" checked={t.connectOnLaunch} onChange={(e) => set("connectOnLaunch", e.target.checked)} />
          {tr("field.connectOnLaunch")}
        </label>
      </div>

      <div className="cmd-preview">
        <span className="muted">{tr("tunnel.equivalent")}</span>
        <code>
          ssh -N -L {t.bindAddress}:{t.localPort}:{t.destHost}:{t.destPort} {t.sshHost || "<host>"}
        </code>
      </div>

      {validation.warnings.map((w) => (
        <div key={w} className="notice warn">
          {w}
        </div>
      ))}
      {running && <div className="notice info">{tr("tunnel.runningNotice")}</div>}
      {error && <div className="notice error">{error}</div>}

      {(testing || test) && (
        <div className="test-result" aria-live="polite">
          {testing && <div className="muted">{tr("tunnel.testingNotice")}</div>}
          {test &&
            (
              [
                ["stage.ssh", test.ssh],
                ["stage.forward", test.forward],
                ["stage.dest", test.destination],
              ] as const
            ).map(([label, st]) => (
              <div key={label} className={`stage stage-${st.status}`}>
                <span className="stage-icon" aria-hidden>
                  {STAGE_ICON[st.status]}
                </span>
                <span className="stage-label">{tr(label)}</span>
                <span className="stage-msg">{st.message}</span>
              </div>
            ))}
        </div>
      )}
    </Modal>
  );
}

import { useCallback, useEffect, useState } from "react";
import { api, errMsg, type HostForm, type HostList, type Preview } from "../api";
import { Modal } from "../components/Modal";
import { useI18n } from "../i18n";

const EMPTY_FORM: HostForm = { alias: "", hostName: "", user: "", port: "", identityFile: "", proxyJump: "" };

export function HostsPage() {
  const { t, tx } = useI18n();
  const [path, setPath] = useState<string | null>(null);
  const [list, setList] = useState<HostList | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [effective, setEffective] = useState<[string, string][] | null>(null);
  const [effError, setEffError] = useState<string | null>(null);
  const [form, setForm] = useState<{ form: HostForm; isNew: boolean } | null>(null);
  const [rawEdit, setRawEdit] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    api
      .listHosts(path)
      .then((l) => {
        setList(l);
        setError(null);
      })
      .catch((e) => setError(errMsg(e)));
  }, [path]);

  useEffect(load, [load]);

  useEffect(() => {
    setEffective(null);
    setEffError(null);
    if (!selected) return;
    api.effectiveHost(selected, list?.path).then(setEffective).catch((e) => setEffError(errMsg(e)));
  }, [selected, list?.path]);

  const host = list?.hosts.find((h) => h.alias === selected);
  const KEYS = ["hostname", "user", "port", "identityfile", "proxyjump", "proxycommand", "serveraliveinterval", "stricthostkeychecking"];

  const editForm = async () => {
    if (!selected) return;
    const f = await api.hostForm(selected, list?.path).catch(() => null);
    setForm({ form: f ?? { ...EMPTY_FORM, alias: selected }, isNew: false });
  };

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>{t("hosts.title")}</h1>
          <p className="muted">{tx("hosts.subtitle", { cmd: <code>ssh -G</code> })}</p>
        </div>
        <div className="row gap">
          <button onClick={() => setRawEdit(true)}>{t("hosts.rawEdit")}</button>
          <button className="primary" onClick={() => setForm({ form: { ...EMPTY_FORM }, isNew: true })}>
            {t("hosts.add")}
          </button>
        </div>
      </header>

      <div className="row gap center path-bar">
        <span className="muted">{t("hosts.configFile")}</span>
        <input
          className="grow mono"
          value={path ?? list?.path ?? ""}
          onChange={(e) => setPath(e.target.value)}
          onBlur={load}
          onKeyDown={(e) => e.key === "Enter" && load()}
          spellCheck={false}
        />
        <button onClick={load}>{t("common.reload")}</button>
      </div>
      {list && !list.exists && <div className="notice info">{t("hosts.notExist")}</div>}
      {error && <div className="notice error">{error}</div>}

      <div className="split">
        <div className="list" role="listbox" aria-label={t("hosts.listLabel")}>
          {list?.hosts.length === 0 && <div className="muted pad">{t("hosts.none")}</div>}
          {list?.hosts.map((h) => (
            <button
              key={h.alias}
              role="option"
              aria-selected={selected === h.alias}
              className={`list-item ${selected === h.alias ? "active" : ""}`}
              onClick={() => setSelected(h.alias)}
            >
              <span className="strong mono">{h.alias}</span>
              {!h.editable && <span className="tag">{t("hosts.advancedTag")}</span>}
            </button>
          ))}
        </div>
        <div className="detail">
          {!selected && <div className="muted pad">{t("hosts.selectHint")}</div>}
          {selected && host && (
            <>
              <div className="row between center">
                <h2 className="mono">{selected}</h2>
                {host.editable ? (
                  <button onClick={editForm}>{t("hosts.edit")}</button>
                ) : (
                  <button onClick={() => setRawEdit(true)}>{t("hosts.editRaw")}</button>
                )}
              </div>
              <div className="muted small">{t("hosts.definedAt", { file: host.source, line: host.line })}</div>
              {!host.editable && <div className="notice info">{t("hosts.notEditable")}</div>}
              <h3>{t("hosts.effective")}</h3>
              {effError && <div className="notice error">{effError}</div>}
              {effective && (
                <table className="kv">
                  <tbody>
                    {effective
                      .filter(([k]) => KEYS.includes(k))
                      .sort((a, b) => KEYS.indexOf(a[0]) - KEYS.indexOf(b[0]))
                      .map(([k, v], i) => (
                        <tr key={k + i}>
                          <th>{k}</th>
                          <td className="mono">{v}</td>
                        </tr>
                      ))}
                  </tbody>
                </table>
              )}
            </>
          )}
        </div>
      </div>

      {form && (
        <HostFormDialog
          initial={form.form}
          isNew={form.isNew}
          path={list?.path ?? null}
          existing={list?.hosts.map((h) => h.alias) ?? []}
          onClose={(saved) => {
            setForm(null);
            if (saved) {
              load();
              setSelected(saved);
            }
          }}
        />
      )}
      {rawEdit && (
        <RawEditDialog
          path={list?.path ?? null}
          onClose={(saved) => {
            setRawEdit(false);
            if (saved) load();
          }}
        />
      )}
    </div>
  );
}

function HostFormDialog({
  initial,
  isNew,
  path,
  existing,
  onClose,
}: {
  initial: HostForm;
  isNew: boolean;
  path: string | null;
  existing: string[];
  onClose: (saved?: string) => void;
}) {
  const { t } = useI18n();
  const [f, setF] = useState(initial);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const set = (k: keyof HostForm, v: string) => setF((x) => ({ ...x, [k]: v }));

  const next = async () => {
    setError(null);
    if (isNew && existing.includes(f.alias.trim())) {
      setError(t("hostForm.duplicate"));
      return;
    }
    try {
      setPreview(await api.previewHostForm(f, path));
    } catch (e) {
      setError(errMsg(e));
    }
  };

  if (preview)
    return (
      <DiffConfirm
        preview={preview}
        path={path}
        onBack={() => setPreview(null)}
        onDone={(ok) => (ok ? onClose(f.alias.trim()) : onClose())}
      />
    );

  const input = (k: keyof HostForm, label: string, placeholder = "", disabled = false) => (
    <label className="field">
      <span className="field-label">{label}</span>
      <input value={f[k]} placeholder={placeholder} disabled={disabled} spellCheck={false} onChange={(e) => set(k, e.target.value)} />
    </label>
  );

  return (
    <Modal
      title={isNew ? t("hostForm.newTitle") : t("hostForm.editTitle", { alias: initial.alias })}
      onClose={() => onClose()}
      footer={
        <>
          <button onClick={() => onClose()}>{t("common.cancel")}</button>
          <button className="primary" onClick={next}>
            {t("hostForm.review")}
          </button>
        </>
      }
    >
      <div className="form-grid">
        {input("alias", t("hostForm.alias"), t("hostForm.aliasPlaceholder"), !isNew)}
        {input("hostName", "HostName", t("hostForm.hostNamePlaceholder"))}
        <div className="field-row">
          {input("user", "User", t("hostForm.userPlaceholder"))}
          {input("port", "Port", "22")}
        </div>
        {input("identityFile", "IdentityFile", "~/.ssh/id_ed25519")}
        {input("proxyJump", "ProxyJump", t("hostForm.proxyJumpPlaceholder"))}
        <p className="muted small">{t("hostForm.emptyHint")}</p>
      </div>
      {error && <div className="notice error">{error}</div>}
    </Modal>
  );
}

function RawEditDialog({ path, onClose }: { path: string | null; onClose: (saved?: boolean) => void }) {
  const { t } = useI18n();
  const [content, setContent] = useState<string | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .readSshConfig(path)
      .then((s) => setContent(s.content))
      .catch((e) => setError(errMsg(e)));
  }, [path]);

  if (preview) return <DiffConfirm preview={preview} path={path} onBack={() => setPreview(null)} onDone={(ok) => onClose(ok)} />;

  return (
    <Modal
      title={t("raw.title")}
      wide
      onClose={() => onClose()}
      footer={
        <>
          <span className="muted small grow mono">{path}</span>
          <button onClick={() => onClose()}>{t("common.cancel")}</button>
          <button
            className="primary"
            disabled={content == null}
            onClick={() =>
              api
                .previewSshConfig(content ?? "", path)
                .then(setPreview)
                .catch((e) => setError(errMsg(e)))
            }
          >
            {t("hostForm.review")}
          </button>
        </>
      }
    >
      {error && <div className="notice error">{error}</div>}
      <textarea
        className="code-editor"
        value={content ?? ""}
        onChange={(e) => setContent(e.target.value)}
        spellCheck={false}
        aria-label={t("raw.label")}
      />
    </Modal>
  );
}

/** 寫入前顯示差異；儲存時會建立備份、以 ssh -G 驗證語法並偵測外部修改 */
function DiffConfirm({
  preview,
  path,
  onBack,
  onDone,
}: {
  preview: Preview;
  path: string | null;
  onBack: () => void;
  onDone: (saved: boolean) => void;
}) {
  const { t } = useI18n();
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const empty = !preview.diff.trim();

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await api.saveSshConfig(preview.content, preview.baseHash, path);
      onDone(true);
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      title={t("diff.title")}
      wide
      onClose={() => onDone(false)}
      footer={
        <>
          <button onClick={onBack}>{t("diff.back")}</button>
          <span className="spacer" />
          <button onClick={() => onDone(false)}>{t("common.cancel")}</button>
          <button className="primary" disabled={empty || saving} onClick={save}>
            {t("diff.save")}
          </button>
        </>
      }
    >
      {empty ? (
        <div className="muted">{t("diff.empty")}</div>
      ) : (
        <pre className="diff">
          {preview.diff.split("\n").map((l, i) => (
            <div
              key={i}
              className={l.startsWith("+") && !l.startsWith("+++") ? "add" : l.startsWith("-") && !l.startsWith("---") ? "del" : l.startsWith("@@") ? "hunk" : ""}
            >
              {l || " "}
            </div>
          ))}
        </pre>
      )}
      {error && <div className="notice error">{error}</div>}
    </Modal>
  );
}
